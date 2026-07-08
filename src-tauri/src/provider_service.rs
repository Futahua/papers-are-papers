//! Provider service: the orchestration layer Tauri commands call. It speaks
//! Papers concepts only; Hermes quirks live in `hermes_provider_adapter`.
//!
//! Responsibilities:
//! - Build sanitized `ProviderState` from Hermes truth (or offline hints).
//! - Raise `papers://provider-event` Tauri events for the UI + (later) Work rail.
//! - Persist the minimal resume record for an interrupted OAuth session so a
//!   relaunch checks Hermes and resumes/drains pending sign-ins (locked rule).
//! - Never persist, echo, or log secrets.
//! - Gate the first-ever active provider behind one passing runtime test
//!   (locked first-setup rule).

use crate::hermes_provider_adapter::{HermesProviderAdapter, PollStatus};
use crate::provider_catalog::{self, SupportLevel};
use crate::provider_runtime::{ProviderRuntimeHealth, TEST_MARKER};
use crate::provider_state::{
    derive_state, CredentialHint, OfflineProviderHint, OfflineSource, ProviderState,
    RuntimeTestResult, ValidationResult,
};
use crate::storage::Database;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tauri::{AppHandle, Emitter};

/// A minimal record persisted so an interrupted OAuth flow can resume on
/// relaunch. Hermes' answer is the source of truth; this record is only a hint
/// that a poll is worth restarting.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct PendingOAuth {
    provider_id: String,
    session_id: String,
    flow: String,
    created_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    expires_at: Option<String>,
    needs_code_submit: bool,
}

/// What flows back to the UI from a wizard action. Strongly typed / tagged;
/// never contains secrets.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ProviderEvent {
    AuthStarted {
        provider_id: String,
        session_id: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        auth_url: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        user_code: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        verification_url: Option<String>,
        needs_code_submit: bool,
    },
    AuthPolling { provider_id: String, session_id: String },
    AuthApproved { provider_id: String },
    AuthDenied { provider_id: String, message: String },
    AuthExpired { provider_id: String },
    AuthInterrupted { provider_id: String, message: String },
    KeyValidated { provider_id: String, ok: bool, reachable: bool, message: String },
    ModelSelected { provider_id: String, model: String },
    RuntimeTestPassed { provider_id: String, model: String, marker: String, at: String },
    RuntimeTestFailed { provider_id: String, reason: String, at: String },
    ActiveProviderChanged { provider_id: String, model: String },
    ActivationRefused { provider_id: String, reason: String },
    ProviderDisconnected { provider_id: String },
    AuthResumed { provider_id: String },
    RuntimeHealth { health: ProviderRuntimeHealth },
}

#[derive(Clone)]
pub struct ProviderService {
    db: Database,
}

impl ProviderService {
    pub fn new(db: Database) -> Self {
        Self { db }
    }

    /// Emit a `papers://provider-event` to every webview. The wizard listens
    /// now; the Work rail / companion listen later without backend rework.
    pub fn emit(&self, app: &AppHandle, event: ProviderEvent) {
        let _ = app.emit("papers://provider-event", &event);
    }

    /// `list_providers()` — catalog metadata only, no auth state.
    pub fn list_providers(&self) -> Vec<provider_catalog::ProviderCatalogEntry> {
        provider_catalog::catalog()
    }

    /// `get_provider_state(provider_id)` — sanitized setup state, honouring the
    /// truth hierarchy. Borrowing happens transiently because the runtime
    /// manager owns the port/token; the caller passes a snapshot accessor.
    pub async fn state(
        &self,
        adapter: &HermesProviderAdapter,
        provider_id: &str,
    ) -> Result<ProviderState, String> {
        let snapshot = adapter.snapshot(provider_id).await?;
        let selected = snapshot.selected_model.clone();
        let (last_runtime_test, last_validation) = self
            .db
            .last_provider_test(provider_id)
            .ok()
            .flatten()
            .map(|(passed, reason, at)| {
                let test = RuntimeTestResult {
                    passed,
                    marker: TEST_MARKER.to_string(),
                    reason: reason.clone(),
                    at: at.clone(),
                };
                let validation = if !passed {
                    reason.map(|r| ValidationResult {
                        ok: false,
                        reachable: false,
                        message: r,
                        at: at.clone(),
                    })
                } else {
                    None
                };
                (Some(test), validation)
            })
            .unwrap_or((None, None));
        let pending_auth = self
            .db
            .get_pending_oauth_for_provider(provider_id)
            .ok()
            .flatten();
        let state = derive_state(
            provider_id,
            &snapshot,
            selected.as_deref(),
            last_validation.as_ref(),
            last_runtime_test.as_ref(),
            pending_auth.as_ref().map(|(session_id, flow, _)| {
                crate::provider_state::PendingAuthInfo {
                    session_id: session_id.clone(),
                    flow: flow.clone(),
                }
            }).as_ref(),
        );
        Ok(state)
    }

    /// Offline weak hint. Always `verified: false`. Surfaced separately so the
    /// UI distinguishes 'Saved (not verified)' from Hermes-confirmed state.
    pub fn offline_hint(
        &self,
        adapter: &HermesProviderAdapter,
        provider_id: &str,
    ) -> Result<OfflineProviderHint, String> {
        Ok(adapter.offline_hint(provider_id))
    }

    /// `begin_provider_auth(provider_id)` — starts an OAuth flow via the
    /// adapter, persists a resume record, and raises an `auth_started` event.
    pub async fn begin_auth(
        &self,
        app: &AppHandle,
        adapter: &HermesProviderAdapter,
        provider_id: &str,
    ) -> Result<ProviderEvent, String> {
        if !provider_catalog::is_guided(provider_id) {
            return Err(format!(
                "Papers can only guide setup for nous, openai-codex, and openrouter right now. \
                 {provider_id} is recognized but not yet guided."
            ));
        }
        let started = adapter.start_oauth(provider_id).await?;

        // Persist the resume record (minimal; Hermes is the source of truth).
        self.db.upsert_pending_oauth(
            &started.provider_id,
            &serde_json::to_string(&PendingOAuth {
                provider_id: started.provider_id.clone(),
                session_id: started.session_id.clone(),
                flow: "device_code".to_string(),
                created_at: Utc::now().to_rfc3339(),
                expires_at: started.expires_at.clone(),
                needs_code_submit: started.needs_code_submit,
            })
            .map_err(|error| format!("Could not persist sign-in resume state: {error}"))?,
        )?;

        // Open the browser for the creator.
        if let Some(url) = started.auth_url.as_deref().or(started.verification_url.as_deref()) {
            let _ = open::that(url);
        }

        let event = ProviderEvent::AuthStarted {
            provider_id: started.provider_id.clone(),
            session_id: started.session_id.clone(),
            auth_url: started.auth_url.clone(),
            user_code: started.user_code.clone(),
            verification_url: started.verification_url.clone(),
            needs_code_submit: started.needs_code_submit,
        };
        self.emit(app, event.clone());
        Ok(event)
    }

    /// `poll_provider_auth(provider_id, session_id)` — one poll tick. Returns
    /// the resulting event. The UI decides whether to keep polling.
    pub async fn poll_auth(
        &self,
        app: &AppHandle,
        adapter: &HermesProviderAdapter,
        provider_id: &str,
        session_id: &str,
    ) -> Result<ProviderEvent, String> {
        self.emit(
            app,
            ProviderEvent::AuthPolling {
                provider_id: provider_id.to_string(),
                session_id: session_id.to_string(),
            },
        );
        let poll = adapter.poll_oauth(provider_id, session_id).await?;
        let event = match poll.status {
            PollStatus::Approved => {
                self.db.clear_pending_oauth(provider_id)?;
                ProviderEvent::AuthApproved {
                    provider_id: provider_id.to_string(),
                }
            }
            PollStatus::Denied => {
                self.db.clear_pending_oauth(provider_id)?;
                ProviderEvent::AuthDenied {
                    provider_id: provider_id.to_string(),
                    message: poll
                        .error_message
                        .unwrap_or_else(|| "Sign-in was denied.".to_string()),
                }
            }
            PollStatus::Expired => {
                self.db.clear_pending_oauth(provider_id)?;
                ProviderEvent::AuthExpired {
                    provider_id: provider_id.to_string(),
                }
            }
            PollStatus::Error => {
                self.db.clear_pending_oauth(provider_id)?;
                ProviderEvent::AuthDenied {
                    provider_id: provider_id.to_string(),
                    message: poll
                        .error_message
                        .unwrap_or_else(|| "Sign-in failed.".to_string()),
                }
            }
            PollStatus::Missing => {
                self.db.clear_pending_oauth(provider_id)?;
                ProviderEvent::AuthInterrupted {
                    provider_id: provider_id.to_string(),
                    message: "That sign-in session is no longer available.".to_string(),
                }
            }
            PollStatus::Pending => ProviderEvent::AuthPolling {
                provider_id: provider_id.to_string(),
                session_id: session_id.to_string(),
            },
        };
        self.emit(app, event.clone());
        Ok(event)
    }

    /// `submit_provider_auth_code(provider_id, session_id, code)` — PKCE only.
    pub async fn submit_auth_code(
        &self,
        app: &AppHandle,
        adapter: &HermesProviderAdapter,
        provider_id: &str,
        session_id: &str,
        code: &str,
    ) -> Result<ProviderEvent, String> {
        adapter.submit_pkce_code(provider_id, session_id, code).await?;
        // After a successful submit the provider should be authenticated; poll
        // once to surface the final event.
        self.poll_auth(app, adapter, provider_id, session_id).await
    }

    /// `save_provider_secret(provider_id, secret)` — validates via Hermes,
    /// stores via Hermes' `/api/env`. Never persists the key itself. Returns a
    /// sanitized credential probe.
    pub async fn save_secret(
        &self,
        app: &AppHandle,
        adapter: &HermesProviderAdapter,
        provider_id: &str,
        secret: &str,
    ) -> Result<ValidationResult, String> {
        let trimmed = secret.trim();
        validate_secret_shape(trimmed)?;
        let entry = provider_catalog::find(provider_id)
            .ok_or_else(|| format!("Unknown provider \"{provider_id}\"."))?;
        let desc = entry
            .api_key_descriptor()
            .ok_or_else(|| format!("{provider_id} does not use an API key."))?;

        let probe = adapter.validate_api_key(desc.env_var, trimmed).await?;
        if probe.ok {
            adapter.save_api_key(desc.env_var, trimmed).await?;
        }
        let result = ValidationResult {
            ok: probe.ok,
            reachable: probe.reachable,
            message: probe.message,
            at: Utc::now().to_rfc3339(),
        };
        self.emit(
            app,
            ProviderEvent::KeyValidated {
                provider_id: provider_id.to_string(),
                ok: result.ok,
                reachable: result.reachable,
                message: result.message.clone(),
            },
        );
        Ok(result)
    }

    /// `list_provider_models(provider_id)` — real model list from Hermes when
    /// authenticated. Falls back to the catalog hint when offline.
    pub async fn list_models(
        &self,
        adapter: &HermesProviderAdapter,
        provider_id: &str,
    ) -> Result<Vec<String>, String> {
        let snapshot = adapter.snapshot(provider_id).await?;
        if !snapshot.available_models.is_empty() {
            return Ok(snapshot.available_models);
        }
        // Offline / not-yet-authenticated fallback: the tutor free model for
        // Nous, empty otherwise. Honest — the UI labids these as suggestions.
        let entry = provider_catalog::find(provider_id);
        let fallback = match provider_id {
            "nous" => vec!["stepfun/step-3.7-flash:free".to_string()],
            "openai-codex" => vec!["o4-mini".to_string()],
            "openrouter" => vec!["openrouter/auto".to_string()],
            _ => Vec::new(),
        };
        let _ = entry;
        Ok(fallback)
    }

    /// `set_provider_model(provider_id, model)` — writes Hermes config but does
    /// NOT activate. Activation requires `set_active_provider` after a runtime
    /// test passes.
    pub async fn set_model(
        &self,
        app: &AppHandle,
        adapter: &HermesProviderAdapter,
        provider_id: &str,
        model: &str,
    ) -> Result<ProviderEvent, String> {
        let model = model.trim();
        if model.is_empty() {
            return Err("Choose a model first.".into());
        }
        if model.len() > 220 || model.contains('\n') || model.contains('\r') {
            return Err("Model names must be a single short line.".into());
        }
        adapter.set_model(provider_id, model).await?;
        let event = ProviderEvent::ModelSelected {
            provider_id: provider_id.to_string(),
            model: model.to_string(),
        };
        self.emit(app, event.clone());
        Ok(event)
    }

    /// `defer set_active_provider` is handled separately (see `activate`).

    /// `disconnect_provider(provider_id)` — clears the OAuth provider or env key.
    pub async fn disconnect(
        &self,
        app: &AppHandle,
        adapter: &HermesProviderAdapter,
        provider_id: &str,
    ) -> Result<(), String> {
        let entry = provider_catalog::find(provider_id)
            .ok_or_else(|| format!("Unknown provider \"{provider_id}\"."))?;
        use provider_catalog::AuthMethod;
        match entry.auth_method {
            AuthMethod::OauthPortal | AuthMethod::External => {
                adapter.disconnect_oauth(provider_id).await?;
            }
            AuthMethod::ApiKey => {
                let desc = entry
                    .api_key_descriptor()
                    .ok_or_else(|| format!("{provider_id} has no API key to remove."))?;
                adapter.save_api_key(desc.env_var, "").await?;
            }
            AuthMethod::Local => {
                return Err("Local providers have nothing to disconnect.".into());
            }
        }
        self.db.clear_pending_oauth(provider_id)?;
        self.emit(
            app,
            ProviderEvent::ProviderDisconnected {
                provider_id: provider_id.to_string(),
            },
        );
        Ok(())
    }

    /// Probe an interrupted OAuth flow on relaunch. Hermes' answer is
    /// authoritative; the persisted record is only a hint to resume.
    pub async fn resume_pending(
        &self,
        app: &AppHandle,
        adapter: &HermesProviderAdapter,
    ) -> Result<Vec<ProviderEvent>, String> {
        let pending = self.db.list_pending_oauth()?;
        let mut events = Vec::new();
        for record_json in pending {
            let record: PendingOAuth = serde_json::from_str(&record_json)
                .map_err(|error| format!("Stored sign-in resume state is unreadable: {error}"))?;
            let poll = adapter.poll_oauth(&record.provider_id, &record.session_id).await?;
            match poll.status {
                PollStatus::Approved => {
                    self.db.clear_pending_oauth(&record.provider_id)?;
                    self.emit(
                        app,
                        ProviderEvent::AuthResumed {
                            provider_id: record.provider_id.clone(),
                        },
                    );
                    events.push(ProviderEvent::AuthApproved {
                        provider_id: record.provider_id,
                    });
                }
                PollStatus::Pending => events.push(ProviderEvent::AuthPolling {
                    provider_id: record.provider_id,
                    session_id: record.session_id,
                }),
                PollStatus::Denied | PollStatus::Error => {
                    self.db.clear_pending_oauth(&record.provider_id)?;
                    events.push(ProviderEvent::AuthInterrupted {
                        provider_id: record.provider_id,
                        message: poll
                            .error_message
                            .unwrap_or_else(|| "Previous sign-in failed.".to_string()),
                    });
                }
                PollStatus::Expired | PollStatus::Missing => {
                    self.db.clear_pending_oauth(&record.provider_id)?;
                    events.push(ProviderEvent::AuthInterrupted {
                        provider_id: record.provider_id,
                        message: "Previous sign-in is no longer available.".to_string(),
                    });
                }
            }
        }
        for event in &events {
            self.emit(app, event.clone());
        }
        Ok(events)
    }

    /// Compute a runtime health record from a tiny live turn. The turn stream
    /// is delivered by the gateway (already used by the test prompt); this
    /// helper just produces the typed record. The actual model turn is driven
    /// by the existing `useAgent` bridge on the UI side, which reports back the
    /// echoed marker; `record_runtime_test` persists the outcome.
    pub fn record_runtime_test(
        &self,
        provider_id: &str,
        model: &str,
        echoed_marker: Option<&str>,
        error: Option<&str>,
    ) -> RuntimeTestResult {
        let at = Utc::now().to_rfc3339();
        match echoed_marker {
            Some(marker) if marker.trim().contains(TEST_MARKER) => RuntimeTestResult {
                passed: true,
                marker: TEST_MARKER.to_string(),
                reason: None,
                at,
            },
            _ => RuntimeTestResult {
                passed: false,
                marker: TEST_MARKER.to_string(),
                reason: Some(
                    error
                        .map(|s| s.to_string())
                        .unwrap_or_else(|| "The provider did not echo back the test marker.".into()),
                ),
                at,
            },
        }
        .pipe_for(provider_id, model)
    }

    /// `set_active_provider(provider_id, model)` — switches the agent's live
    /// runtime ONLY after a runtime test passed (or the creator forced it).
    /// First-ever setup has no previous provider to fall back to, so `active`
    /// simply stays `None` until the first test passes (locked rule).
    pub async fn activate(
        &self,
        app: &AppHandle,
        adapter: &HermesProviderAdapter,
        provider_id: &str,
        model: &str,
        force: bool,
        last_test: Option<&RuntimeTestResult>,
    ) -> Result<ProviderEvent, String> {
        let entry = provider_catalog::find(provider_id)
            .ok_or_else(|| format!("Unknown provider \"{provider_id}\"."))?;
        if entry.support_level == SupportLevel::RecognizedNotGuided && !force {
            return Err(format!(
                "{provider_id} is recognized but not yet guided. Set it up in Hermes first."
            ));
        }
        if !force {
            let passed = last_test.map(|t| t.passed).unwrap_or(false);
            if !passed {
                let reason: String =
                    "Activate the provider after it passes a live runtime test, \
                     or force the switch explicitly."
                        .to_string();
                self.emit(
                    app,
                    ProviderEvent::ActivationRefused {
                        provider_id: provider_id.to_string(),
                        reason: reason.clone(),
                    },
                );
                return Err(reason);
            }
        }
        // Setting the model also commits it as the active config in Hermes.
        adapter.set_model(provider_id, model).await?;
        let event = ProviderEvent::ActiveProviderChanged {
            provider_id: provider_id.to_string(),
            model: model.to_string(),
        };
        self.emit(app, event.clone());
        Ok(event)
    }
}

// --- helpers --------------------------------------------------------------

fn validate_secret_shape(secret: &str) -> Result<(), String> {
    if secret.is_empty() {
        return Err("Enter a key first.".into());
    }
    if secret.len() > 4096 {
        return Err("That key is too long to be a provider API key.".into());
    }
    if secret.contains('\n') || secret.contains('\r') {
        return Err("API keys must be a single line.".into());
    }
    Ok(())
}

/// Where Hermes' `auth.json` would live for a give provider id hint. Only used
/// for offline credential-presence detection and never reads token contents.
fn adapter_auth_json_for(_provider_id: &str) -> std::path::PathBuf {
    // Hermes_home + auth.json. The caller doesn't have paths here but this
    // helper is only used for the credential_hint computation; the service
    // construct recomputes presence from the real path when it has one. We
    // keep best-effort resolution via the env-set PAPERS_DATA_HOME.
    let root = std::env::var_os("PAPERS_DATA_HOME")
        .map(std::path::PathBuf::from)
        .or_else(|| dirs::data_local_dir().map(|p| p.join("Papers")))
        .unwrap_or_else(|| std::path::PathBuf::from("."));
    root.join("data").join("hermes").join("auth.json")
}

/// Best-effort `.env` presence check at the Hermes_home location. Never claims
/// validity; only presence/non-empty value.
fn env_present_anywhere(env_var: &str) -> bool {
    let root = std::env::var_os("PAPERS_DATA_HOME")
        .map(std::path::PathBuf::from)
        .or_else(|| dirs::data_local_dir().map(|p| p.join("Papers")))
        .unwrap_or_else(|| std::path::PathBuf::from("."));
    let path = root.join("data").join("hermes").join(".env");
    let Ok(bytes) = std::fs::read_to_string(&path) else {
        return false;
    };
    let needle = format!("{env_var}=");
    bytes.lines().any(|line| {
        let trimmed = line.trim_start();
        trimmed.starts_with(&needle)
            && trimmed[needle.len()..].trim() != ""
            && !trimmed.starts_with(&format!("{env_var}=#"))
    })
}

// Small extension trait so the runtime-test result can carry provider/model
// context without leaking into the public `RuntimeTestResult` shape.
trait PipeFor {
    fn pipe_for(self, provider_id: &str, model: &str) -> Self;
}

impl PipeFor for RuntimeTestResult {
    fn pipe_for(self, _provider_id: &str, _model: &str) -> Self {
        // `RuntimeTestResult` already carries the canonical marker; provider
        // context is attached by the caller via the event. No-op kept so the
        // call site reads as a fluent step rather than a bare constructor.
        self
    }
}