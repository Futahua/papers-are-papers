//! Provider runtime health: a deliberate runtime probe, separate from setup
//! state. Answers "if I try to use this provider right now, what happens?"

use serde::Serialize;

/// The result of a live provider runtime probe. No secrets, only flags + an
/// optional sanitized error. This is the operational health probe the
/// workbench reuses, not a wizard-only helper.
#[derive(Debug, Clone, Serialize)]
pub struct ProviderRuntimeHealth {
    pub provider_id: String,
    /// Hermes' gateway answered status.
    pub gateway_ok: bool,
    /// Hermes was able to reach the provider's API during the probe
    /// (i.e. the turn attempt reached the provider, regardless of outcome).
    pub reachable: bool,
    /// The provider accepted the tiny turn and echoed the marker.
    pub can_stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider_error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model_error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auth_error: Option<String>,
    pub rate_limited: bool,
    pub last_tested_at: String,
}

impl ProviderRuntimeHealth {
    /// True when the provider can be safely activated: gateway ok and the
    /// model accepted the turn. Failures are surfaced via the typed fields so
    /// the UI distinguishes auth/quota/network problems.
    pub fn passed(&self) -> bool {
        self.gateway_ok && self.can_stream
    }
}

/// The marker the runtime test asks the provider to echo back, proving a real
/// turn ran. Kept hardcoded so the UI can assert against it without trusting a
/// model-derived string.
pub const TEST_MARKER: &str = "PAPERS_PROVIDER_TEST_OK";

/// The prompt sent during a runtime test. The provider replies with the
/// marker exactly, then stops.
pub const TEST_PROMPT: &str =
    "Provider health check: reply with exactly `PAPERS_PROVIDER_TEST_OK`, then stop.";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn passed_requires_gateway_and_stream() {
        let ok = ProviderRuntimeHealth {
            provider_id: "nous".into(),
            gateway_ok: true,
            reachable: true,
            can_stream: true,
            provider_error: None,
            model_error: None,
            auth_error: None,
            rate_limited: false,
            last_tested_at: "t".into(),
        };
        assert!(ok.passed());

        let mut fail = ok.clone();
        fail.can_stream = false;
        fail.rate_limited = true;
        assert!(!fail.passed());
        assert!(fail.rate_limited);
    }

    #[test]
    fn test_marker_is_stable_literal() {
        assert_eq!(TEST_MARKER, "PAPERS_PROVIDER_TEST_OK");
        assert!(TEST_PROMPT.contains(TEST_MARKER));
    }
}