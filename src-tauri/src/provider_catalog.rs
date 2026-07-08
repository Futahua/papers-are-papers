//! Provider catalog: pure metadata describing what each provider is and how
//! it is set up. No auth state, no models, no runtime health. Those live in
//! `provider_state` and `provider_runtime`.
//!
//! This is a Papers-native concept layer. Hermes endpoint quirks are kept in
//! `hermes_provider_adapter`; this module does not know Hermes routes.

use serde::Serialize;

/// How a provider authenticates the creator.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AuthMethod {
    /// Browser OAuth/portal flow owned by Hermes; Papers opens the browser and
    /// polls Hermes for completion. Examples: Nous, OpenAI Codex.
    OauthPortal,
    /// Provider needs an API key issued by its console. Papers routes the key
    /// through a minimal isolated credential prompt; the key is stored by
    /// Hermes and never persisted or echoed by Papers.
    ApiKey,
    /// Local daemon the creator points Papers at (e.g. Ollama). Auth is a
    /// reachable base URL; no secrets required.
    Local,
    /// Provider is set up by an external CLI the creator runs themselves
    /// (e.g. `claude setup-token`, `qwen-code` auth). Papers cannot guide it.
    External,
}

/// Concrete OAuth shape Hermes uses, when applicable. Drives the wizard UI.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AuthFlow {
    /// Device-code: show user_code + verification URL, poll Hermes.
    DeviceCode,
    /// PKCE: open browser, then the creator pastes the callback code.
    Pkce,
    /// Loopback: Hermes binds a local callback server; no code to paste.
    Loopback,
    /// Papers only shows the CLI command; no browser handoff.
    ManualCli,
}

/// Honest support label so the catalog never claims a capability it lacks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SupportLevel {
    /// Implemented and exercised end-to-end on the creator's machine.
    GuidedTested,
    /// Shown in the catalog; Papers recognizes it but does not yet guide setup.
    /// The wizard renders an `ExternalSetupRequired` / `RecognizedNotGuided`
    /// state rather than faking support.
    RecognizedNotGuided,
    /// Reserved for future providers. Visible only as a name, not selectable.
    Future,
}

/// One entry in the provider catalog. Pure metadata.
#[derive(Debug, Clone, Serialize)]
pub struct ProviderCatalogEntry {
    pub id: String,
    pub label: String,
    pub auth_method: AuthMethod,
    /// Present when `auth_method` is `oauth_portal` or `external`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub flow: Option<AuthFlow>,
    pub supports_model_listing: bool,
    pub supports_disconnect: bool,
    pub supports_live_validation: bool,
    pub supports_runtime_test: bool,
    /// UI copy driven centrally so the wizard stays consistent.
    pub setup_copy: String,
    pub docs_hint: Option<String>,
    pub support_level: SupportLevel,
}

/// The env var Hermes stores an API key under, when `auth_method == ApiKey`.
/// Kept here (and only here) so the rest of Papers never hardcodes env names.
#[derive(Debug, Clone)]
pub struct ApiKeyDescriptor {
    pub env_var: &'static str,
    pub label: String,
    pub docs_url: &'static str,
}

impl ProviderCatalogEntry {
    pub fn api_key_descriptor(&self) -> Option<ApiKeyDescriptor> {
        match self.id.as_str() {
            "openrouter" => Some(ApiKeyDescriptor {
                env_var: "OPENROUTER_API_KEY",
                label: "OpenRouter API key".to_string(),
                docs_url: "https://openrouter.ai/keys",
            }),
            "openai" => Some(ApiKeyDescriptor {
                env_var: "OPENAI_API_KEY",
                label: "OpenAI API key".to_string(),
                docs_url: "https://platform.openai.com/api-keys",
            }),
            "xai" => Some(ApiKeyDescriptor {
                env_var: "XAI_API_KEY",
                label: "xAI API key".to_string(),
                docs_url: "https://console.x.ai",
            }),
            "google" => Some(ApiKeyDescriptor {
                env_var: "GEMINI_API_KEY",
                label: "Gemini API key".to_string(),
                docs_url: "https://aistudio.google.dev/app/apikey",
            }),
            _ => None,
        }
    }
}

/// Returns the full catalog. Order is stable; first slice providers first.
///
/// First slice (guided + tested): nous, openai-codex, openrouter.
/// Recognized-not-guided: anthropic, xai, google, ollama, minimax-oauth, xai-oauth.
/// External-CLI only: claude-code, qwen-oauth.
/// Future: empty for now.
pub fn catalog() -> Vec<ProviderCatalogEntry> {
    use AuthFlow::*;
    use AuthMethod::*;
    use SupportLevel::*;

    vec![
        // --- Guided + tested first slice ---
        ProviderCatalogEntry {
            id: "nous".into(),
            label: "Nous Portal".into(),
            auth_method: OauthPortal,
            flow: Some(DeviceCode),
            supports_model_listing: true,
            supports_disconnect: true,
            supports_live_validation: true,
            supports_runtime_test: true,
            setup_copy: "Sign in through the Nous Portal in your browser. Papers waits for Hermes to confirm the sign-in, then lists your available models.".into(),
            docs_hint: Some("https://portal.nousresearch.com".into()),
            support_level: GuidedTested,
        },
        ProviderCatalogEntry {
            id: "openai-codex".into(),
            label: "OpenAI Codex (ChatGPT)".into(),
            auth_method: OauthPortal,
            flow: Some(DeviceCode),
            supports_model_listing: true,
            supports_disconnect: true,
            supports_live_validation: true,
            supports_runtime_test: true,
            setup_copy: "Sign in with your ChatGPT account through OpenAI's device flow. Papers waits for Hermes to confirm the sign-in, then lists your available models.".into(),
            docs_hint: Some("https://platform.openai.com/docs".into()),
            support_level: GuidedTested,
        },
        ProviderCatalogEntry {
            id: "openrouter".into(),
            label: "OpenRouter (API key)".into(),
            auth_method: ApiKey,
            flow: None,
            supports_model_listing: true,
            supports_disconnect: true,
            supports_live_validation: true,
            supports_runtime_test: true,
            setup_copy: "Paste an OpenRouter API key. Papers sends it only to Hermes for validation and storage; the main app never keeps it.".into(),
            docs_hint: Some("https://openrouter.ai/keys".into()),
            support_level: GuidedTested,
        },
        // --- Recognized, not guided ---
        ProviderCatalogEntry {
            id: "anthropic".into(),
            label: "Anthropic (Claude API)".into(),
            auth_method: OauthPortal,
            flow: Some(Pkce),
            supports_model_listing: true,
            supports_disconnect: true,
            supports_live_validation: true,
            supports_runtime_test: true,
            setup_copy: "Anthropic PKCE sign-in is recognized. Papers has not yet exercised this flow end-to-end; set it up in Hermes for now.".into(),
            docs_hint: Some("https://docs.claude.com/en/api/getting-started".into()),
            support_level: RecognizedNotGuided,
        },
        ProviderCatalogEntry {
            id: "xai".into(),
            label: "xAI (API key)".into(),
            auth_method: ApiKey,
            flow: None,
            supports_model_listing: true,
            supports_disconnect: true,
            supports_live_validation: true,
            supports_runtime_test: true,
            setup_copy: "xAI API-key setup is recognized. Papers has not yet exercised this flow end-to-end; set it up in Hermes for now.".into(),
            docs_hint: Some("https://console.x.ai".into()),
            support_level: RecognizedNotGuided,
        },
        ProviderCatalogEntry {
            id: "xai-oauth".into(),
            label: "xAI Grok OAuth (SuperGrok / Premium+)".into(),
            auth_method: OauthPortal,
            flow: Some(Loopback),
            supports_model_listing: true,
            supports_disconnect: true,
            supports_live_validation: true,
            supports_runtime_test: true,
            setup_copy: "xAI Grok OAuth loopback is recognized. Papers has not yet exercised this flow end-to-end; set it up in Hermes for now.".into(),
            docs_hint: Some("https://hermes-agent.nousresearch.com/docs/guides/xai-grok-oauth".into()),
            support_level: RecognizedNotGuided,
        },
        ProviderCatalogEntry {
            id: "google".into(),
            label: "Google Gemini (API key)".into(),
            auth_method: ApiKey,
            flow: None,
            supports_model_listing: true,
            supports_disconnect: true,
            supports_live_validation: true,
            supports_runtime_test: true,
            setup_copy: "Gemini API-key setup is recognized. Papers has not yet exercised this flow end-to-end; set it up in Hermes for now.".into(),
            docs_hint: Some("https://aistudio.google.dev/app/apikey".into()),
            support_level: RecognizedNotGuided,
        },
        ProviderCatalogEntry {
            id: "minimax-oauth".into(),
            label: "MiniMax (OAuth)".into(),
            auth_method: OauthPortal,
            flow: Some(DeviceCode),
            supports_model_listing: true,
            supports_disconnect: true,
            supports_live_validation: true,
            supports_runtime_test: true,
            setup_copy: "MiniMax OAuth device-code is recognized. Papers has not yet exercised this flow end-to-end; set it up in Hermes for now.".into(),
            docs_hint: Some("https://www.minimax.io".into()),
            support_level: RecognizedNotGuided,
        },
        ProviderCatalogEntry {
            id: "ollama".into(),
            label: "Ollama (local)".into(),
            auth_method: Local,
            flow: None,
            supports_model_listing: true,
            supports_disconnect: false,
            supports_live_validation: true,
            supports_runtime_test: true,
            setup_copy: "Point Papers at a running local Ollama daemon. No API key required. Papers has not yet exercised this flow end-to-end; set it up in Hermes for now.".into(),
            docs_hint: Some("https://ollama.com".into()),
            support_level: RecognizedNotGuided,
        },
        // --- External CLI only ---
        ProviderCatalogEntry {
            id: "claude-code".into(),
            label: "Claude Code (subscription)".into(),
            auth_method: External,
            flow: Some(ManualCli),
            supports_model_listing: false,
            supports_disconnect: false,
            supports_live_validation: false,
            supports_runtime_test: false,
            setup_copy: "Claude Code is set up by an external CLI. Papers cannot guide this flow; run `claude setup-token` yourself, then Papers will detect it.".into(),
            docs_hint: Some("https://docs.claude.com/en/docs/claude-code".into()),
            support_level: RecognizedNotGuided,
        },
        ProviderCatalogEntry {
            id: "qwen-oauth".into(),
            label: "Qwen (via Qwen CLI)".into(),
            auth_method: External,
            flow: Some(ManualCli),
            supports_model_listing: false,
            supports_disconnect: false,
            supports_live_validation: false,
            supports_runtime_test: false,
            setup_copy: "Qwen is set up by an external CLI. Papers cannot guide this flow; run `hermes auth add qwen-oauth` yourself, then Papers will detect it.".into(),
            docs_hint: Some("https://github.com/QwenLM/qwen-code".into()),
            support_level: RecognizedNotGuided,
        },
    ]
}

/// True for providers the wizard will actually guide end-to-end in the first
/// slice. Everything else surfaces a `RecognizedNotGuided` /
/// `ExternalSetupRequired` state instead of pretending to support it.
pub fn is_guided(provider_id: &str) -> bool {
    matches!(provider_id, "nous" | "openai-codex" | "openrouter")
}

/// Find one catalog entry by id. None if unknown.
pub fn find(provider_id: &str) -> Option<ProviderCatalogEntry> {
    catalog().into_iter().find(|entry| entry.id == provider_id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_has_stable_first_slice() {
        let ids: Vec<_> = catalog().into_iter().map(|e| e.id).collect();
        assert!(ids.contains(&"nous".to_string()));
        assert!(ids.contains(&"openai-codex".to_string()));
        assert!(ids.contains(&"openrouter".to_string()));
        // First slice is exactly these three guided providers.
        let guided: Vec<_> = catalog()
            .into_iter()
            .filter(|e| e.support_level == SupportLevel::GuidedTested)
            .map(|e| e.id)
            .collect();
        assert_eq!(guided, vec!["nous", "openai-codex", "openrouter"]);
    }

    #[test]
    fn api_key_descriptors_live_only_here() {
        let orc = find("openrouter").unwrap();
        let desc = orc.api_key_descriptor().unwrap();
        assert_eq!(desc.env_var, "OPENROUTER_API_KEY");
        // OAuth providers must not advertise an API-key descriptor.
        assert!(find("nous").unwrap().api_key_descriptor().is_none());
    }

    #[test]
    fn external_providers_are_not_guided_or_disconnectable() {
        let cc = find("claude-code").unwrap();
        assert_eq!(cc.auth_method, AuthMethod::External);
        assert!(!cc.supports_disconnect);
        assert!(!is_guided(&cc.id));
    }

    #[test]
    fn recognized_not_guided_providers_refuse_wizard() {
        for id in ["anthropic", "xai", "xai-oauth", "google", "ollama", "minimax-oauth"] {
            assert!(!is_guided(id), "{id} should not be guided yet");
            assert!(find(id).is_some(), "{id} should be in the catalog");
        }
    }
}