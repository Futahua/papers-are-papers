//! Provider state machine: the real, multi-branch setup state for a single
//! provider. Rendered by the wizard; not a "step number + booleans" model.
//!
//! Truth hierarchy (locked, per creator direction):
//! - Hermes running  → Hermes HTTP API is the sole authority for authenticated
//!   / validated / models / active / runtime-test results.
//! - Hermes down     → file reads may only surface weak, clearly-labelled hints
//!   ("Saved (not verified)"); never Authenticated/Validated/Ready/Active.
//!
//! `OfflineProviderHint` always carries `verified: false`.

use crate::hermes_provider_adapter::HermesAuthSnapshot;
use crate::provider_catalog::{self, SupportLevel};
use serde::Serialize;

/// The full self-contained setup state for one provider.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ProviderSetupState {
    /// No credentials or model selection detected.
    Unconfigured,
    /// A browser/handoff OAuth flow is in progress.
    AuthInProgress {
        flow: String,
        session_id: String,
        /// Device-code UX: shown to the creator alongside the verification URL.
        #[serde(skip_serializing_if = "Option::is_none")]
        user_code: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        verification_url: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        expires_at: Option<String>,
    },
    /// PKCE flow is waiting for the creator to paste the callback code.
    AwaitingPkceCode {
        session_id: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        expires_at: Option<String>,
    },
    /// Credentials exist and Hermes can use the provider, but no model selected.
    ConfiguredNoModel,
    /// Credentials exist and a model is selected, but not yet validated/tested.
    ConfiguredModelSelected,
    /// A credential was submitted but Hermes rejected it.
    ValidationFailed {
        message: String,
    },
    /// Config is readable and Hermes is ready; a runtime test has not run yet.
    ReadyForTest,
    /// A real tiny model turn succeeded; the provider may be activated.
    RuntimeTestPassed {
        at: String,
        /// The marker Hermes was asked to echo back, proving a real turn ran.
        marker: String,
    },
    /// A real tiny model turn ran but failed (quota, revoked, bad model, etc.).
    RuntimeTestFailed {
        reason: String,
        at: String,
    },
    /// Provider is recognized but the wizard does not guide setup; show
    /// instructions instead of pretending to handle it.
    ExternalSetupRequired {
        instructions: String,
        cli_command: String,
    },
    /// Provider is in the catalog but the wizard intentionally does not guide
    /// its setup yet (first slice scoping). Surface a "recognized, not guided"
    /// message and point the creator at Hermes setup.
    RecognizedNotGuided {
        hint: String,
    },
}

#[derive(Debug, Clone, Serialize)]
pub struct ValidationResult {
    pub ok: bool,
    pub reachable: bool,
    pub message: String,
    pub at: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct RuntimeTestResult {
    pub passed: bool,
    pub marker: String,
    pub reason: Option<String>,
    pub at: String,
}

/// Sanitized configured/authenticated/selected state for one provider.
/// Never contains secrets. `authenticated`/`validated` derive only from
/// Hermes when it is running; offline snapshots keep `verified: false`.
#[derive(Debug, Clone, Serialize)]
pub struct ProviderState {
    pub provider_id: String,
    pub setup_state: ProviderSetupState,
    pub configured: bool,
    pub authenticated: bool,
    /// The model Hermes is configured to use for this provider, if any.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub selected_model: Option<String>,
    pub can_disconnect: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_validation: Option<ValidationResult>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_runtime_test: Option<RuntimeTestResult>,
    /// True only when Hermes was reachable when this state was computed.
    pub verified_by_hermes: bool,
    pub message: String,
}

/// Weak offline hint read directly from Hermes-managed files when Hermes is
/// down. Always `verified: false` — never a "working" claim.
#[derive(Debug, Clone, Serialize)]
pub struct OfflineProviderHint {
    pub provider_id: String,
    pub credential_hint: CredentialHint,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub selected_model: Option<String>,
    pub source: OfflineSource,
    /// Always false for file-derived hints. Documents the truth hierarchy.
    pub verified: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CredentialHint {
    None,
    Present,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum OfflineSource {
    AuthJson,
    Env,
    Config,
}

/// Builds a `ProviderState` from a possibly-offline Hermes snapshot.
///
/// This is the only constructor the rest of Papers should call. It encodes the
/// truth hierarchy: when `snapshot.hermes_running` is true the state comes
/// from Hermes' answers; when false it collapses to an offline hint with
/// `verified_by_hermes = false` and a clearly-labelled "not verified" message.
pub fn derive_state(
    provider_id: &str,
    snapshot: &HermesAuthSnapshot,
    selected_model: Option<&str>,
    last_validation: Option<&ValidationResult>,
    last_runtime_test: Option<&RuntimeTestResult>,
) -> ProviderState {
    let entry = provider_catalog::find(provider_id);
    let can_disconnect = entry
        .as_ref()
        .map(|e| e.supports_disconnect)
        .unwrap_or(false);

    if entry.is_none() || entry.as_ref().map(|e| e.id.is_empty()).unwrap_or(true) {
        let _ = entry;
        return ProviderState {
            provider_id: provider_id.to_string(),
            setup_state: ProviderSetupState::Unconfigured,
            configured: false,
            authenticated: false,
            selected_model: None,
            can_disconnect: false,
            last_validation: None,
            last_runtime_test: None,
            verified_by_hermes: false,
            message: format!("Papers does not recognize a provider called \"{provider_id}\"."),
        };
    }
    let entry = entry.unwrap();

    // External CLI providers: the wizard never guides these.
    if entry.support_level == SupportLevel::RecognizedNotGuided
        && entry
            .setup_copy
            .starts_with("Claude Code is set up by an external CLI")
    {
        return ProviderState {
            provider_id: provider_id.to_string(),
            setup_state: ProviderSetupState::ExternalSetupRequired {
                instructions: entry.setup_copy.clone(),
                cli_command: "claude setup-token".to_string(),
            },
            configured: false,
            authenticated: false,
            selected_model: None,
            can_disconnect: false,
            last_validation: None,
            last_runtime_test: None,
            verified_by_hermes: snapshot.hermes_running,
            message: entry.setup_copy.clone(),
        };
    }

    // Recognized, not guided (non-external): honest "not yet guided" state.
    if entry.support_level == SupportLevel::RecognizedNotGuided {
        return ProviderState {
            provider_id: provider_id.to_string(),
            setup_state: ProviderSetupState::RecognizedNotGuided {
                hint: entry.setup_copy.clone(),
            },
            configured: false,
            authenticated: false,
            selected_model: selected_model.map(|s| s.to_string()),
            can_disconnect,
            last_validation: None,
            last_runtime_test: None,
            verified_by_hermes: snapshot.hermes_running,
            message: format!(
                "{provider_id} is recognized but Papers has not yet guided its setup. \
                 Set it up in Hermes for now; this flow will be exercised in a later pass."
            ),
        };
    }

    // Guided providers: honour the truth hierarchy.
    let message = compose_message(provider_id, snapshot, selected_model);
    let setup_state = compose_setup_state(provider_id, snapshot, selected_model, last_runtime_test);

    ProviderState {
        provider_id: provider_id.to_string(),
        setup_state,
        configured: snapshot.configured,
        authenticated: snapshot.authenticated,
        selected_model: selected_model.map(|s| s.to_string()),
        can_disconnect,
        last_validation: last_validation.cloned(),
        last_runtime_test: last_runtime_test.cloned(),
        verified_by_hermes: snapshot.hermes_running,
        message,
    }
}

fn compose_setup_state(
    provider_id: &str,
    snapshot: &HermesAuthSnapshot,
    selected_model: Option<&str>,
    last_runtime_test: Option<&RuntimeTestResult>,
) -> ProviderSetupState {
    let entry = match provider_catalog::find(provider_id) {
        Some(entry) => entry,
        None => return ProviderSetupState::Unconfigured,
    };

    // Offline: only weak hints, never strong claims.
    if !snapshot.hermes_running {
        if snapshot.configured || selected_model.is_some() {
            // Keep it as Unconfigured-but-message-bearing; a true offline hint
            // is surfaced via `OfflineProviderHint` separately for the UI.
            return ProviderSetupState::Unconfigured;
        }
        return ProviderSetupState::Unconfigured;
    }

    // Hermes running: its answers are authoritative.
    if !snapshot.configured && !snapshot.authenticated {
        return ProviderSetupState::Unconfigured;
    }

    // If a runtime test already passed, surface Ready/RuntimeTestPassed first.
    if let Some(test) = last_runtime_test {
        if test.passed {
            return ProviderSetupState::RuntimeTestPassed {
                at: test.at.clone(),
                marker: test.marker.clone(),
            };
        }
        return ProviderSetupState::RuntimeTestFailed {
            reason: test.reason.clone().unwrap_or_else(|| "Runtime test failed.".into()),
            at: test.at.clone(),
        };
    }

    match selected_model {
        Some(model) if !model.trim().is_empty() => {
            let _ = entry;
            ProviderSetupState::ConfiguredModelSelected
        }
        _ => ProviderSetupState::ConfiguredNoModel,
    }
}

fn compose_message(
    provider_id: &str,
    snapshot: &HermesAuthSnapshot,
    selected_model: Option<&str>,
) -> String {
    if !snapshot.hermes_running {
        if snapshot.configured || selected_model.is_some() {
            return format!(
                "Saved setup detected for {provider_id}, but Hermes is not running. \
                 Start Hermes to verify and use it."
            );
        }
        return format!("No saved setup detected for {provider_id}.");
    }

    if !snapshot.configured && !snapshot.authenticated {
        return format!("{provider_id} is not set up yet.");
    }
    if let Some(model) = selected_model {
        if snapshot.authenticated {
            format!("{provider_id} / {model} is configured and Hermes confirmed the sign-in.")
        } else {
            format!("{provider_id} / {model} is selected, but Hermes has not confirmed a saved sign-in for it.")
        }
    } else {
        format!("{provider_id} credentials detected, but no model is selected yet.")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hermes_provider_adapter::HermesAuthSnapshot;
    use crate::provider_catalog::SupportLevel;

    fn snapshot(running: bool, configured: bool, authenticated: bool) -> HermesAuthSnapshot {
        HermesAuthSnapshot {
            hermes_running: running,
            configured,
            authenticated,
            active_auth_provider: None,
            owned_auth_provider: None,
            available_models: Vec::new(),
            recommended_model: None,
            selected_model: None,
        }
    }

    #[test]
    fn offline_never_claims_authenticated() {
        let state = derive_state("nous", &snapshot(false, true, false), Some("m1"), None, None);
        assert!(!state.verified_by_hermes);
        assert!(!state.authenticated);
        assert!(matches!(state.setup_state, ProviderSetupState::Unconfigured));
        assert!(state.message.contains("not running"));
    }

    #[test]
    fn online_configured_with_model_is_configured_model_selected() {
        let state = derive_state(
            "nous",
            &snapshot(true, true, true),
            Some("stepfun/step-3.7-flash:free"),
            None,
            None,
        );
        assert!(state.verified_by_hermes);
        assert!(state.authenticated);
        assert!(matches!(
            state.setup_state,
            ProviderSetupState::ConfiguredModelSelected
        ));
    }

    #[test]
    fn online_without_model_is_configured_no_model() {
        let state = derive_state("nous", &snapshot(true, true, true), None, None, None);
        assert!(matches!(
            state.setup_state,
            ProviderSetupState::ConfiguredNoModel
        ));
    }

    #[test]
    fn recognized_not_guided_shows_honest_state() {
        let state = derive_state("anthropic", &snapshot(true, true, true), Some("claude-3"), None, None);
        assert!(matches!(
            state.setup_state,
            ProviderSetupState::RecognizedNotGuided { .. }
        ));
        assert!(!state.authenticated);
    }

    #[test]
    fn runtime_test_passed_wins_over_model_selected() {
        let test = RuntimeTestResult {
            passed: true,
            marker: "PAPERS_PROVIDER_TEST_OK".into(),
            reason: None,
            at: "2026-07-08T00:00:00Z".into(),
        };
        let state = derive_state(
            "nous",
            &snapshot(true, true, true),
            Some("m"),
            None,
            Some(&test),
        );
        assert!(matches!(
            state.setup_state,
            ProviderSetupState::RuntimeTestPassed { .. }
        ));
    }

    #[test]
    fn unknown_provider_is_unconfigured_and_disconnected() {
        let state = derive_state("not-a-real-provider", &snapshot(true, true, true), None, None, None);
        assert!(matches!(state.setup_state, ProviderSetupState::Unconfigured));
        assert!(!state.can_disconnect);
        assert!(!state.verified_by_hermes); // hermes_running true but unknown
        let _ = state.message;
    }

    #[test]
    fn offline_hint_payload_is_stamped_unverified() {
        let hint = OfflineProviderHint {
            provider_id: "openrouter".into(),
            credential_hint: CredentialHint::Present,
            selected_model: Some("openrouter/auto".into()),
            source: OfflineSource::Env,
            verified: false,
        };
        let json = serde_json::to_string(&hint).unwrap();
        assert!(json.contains("\"verified\":false"));
    }

    #[test]
    fn support_level_serializes_snake_case() {
        assert_eq!(
            serde_json::to_string(&SupportLevel::GuidedTested).unwrap(),
            "\"guided_tested\""
        );
    }
}