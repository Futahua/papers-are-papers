//! Hermes provider adapter — the only module that knows Hermes endpoint
//! paths, env var names, OAuth flow quirks, and Hermes-managed file locations.
//! Everything above this layer speaks Papers concepts and never touches Hermes
//! routes directly. If Hermes changes its contract, only this module moves.

use crate::paths::PapersPaths;
use crate::models::HermesLock;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// A sanitized, Papers-security-boundary-safe snapshot of one provider's auth
/// state as Hermes sees it. Contains no secrets.
#[derive(Debug, Clone)]
pub struct HermesAuthSnapshot {
    /// Whether Hermes' gateway was reachable when this snapshot was taken.
    pub hermes_running: bool,
    /// Hermes reports a saved/configured credential for this provider.
    pub configured: bool,
    /// Hermes reports an authenticated, usable credential for this provider.
    pub authenticated: bool,
    /// `active_provider` field from Hermes' auth.json (OAuth case), if any.
    pub active_auth_provider: Option<String>,
    /// `providers[provider].logged_in` style flag from the OAuth status
    /// endpoint, when available.
    pub owned_auth_provider: Option<String>,
    /// Models Hermes can list for this provider (when authenticated).
    pub available_models: Vec<String>,
    /// Hermes' recommended default model for the provider, if any.
    pub recommended_model: Option<String>,
    /// The model currently selected in Hermes config for this provider.
    pub selected_model: Option<String>,
}

/// What Hermes' `/api/providers/oauth/{id}/start` returns for a provider.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthStartResult {
    pub provider_id: String,
    pub session_id: String,
    /// Browser URL to open (device_code + some external) or PKCE auth URL.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auth_url: Option<String>,
    /// Device-code UX: copy this code then visit verification_url.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub verification_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<String>,
    /// Whether the creator must paste a callback code back (PKCE flows).
    pub needs_code_submit: bool,
}

/// What `/api/providers/oauth/{id}/poll/{session_id}` reports.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum PollStatus {
    Pending,
    Approved,
    Denied,
    Expired,
    Error,
    Missing,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PollResult {
    pub status: PollStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_message: Option<String>,
}

/// Result of an API-key validation probe (Hermes' `/api/providers/validate`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CredentialProbe {
    pub ok: bool,
    pub reachable: bool,
    pub message: String,
}

/// Adapter owns the inputs it needs (cheap clones of PapersPaths/HermesLock/
/// reqwest::Client). This lets Tauri commands construct it without lifetime
/// gymnastics. The cost is paid only on a settings-flow call, never a hot path.
pub struct HermesProviderAdapter {
    paths: PapersPaths,
    #[allow(dead_code)]
    lock: HermesLock,
    client: reqwest::Client,
    port: Option<u16>,
    token: Option<String>,
}

impl HermesProviderAdapter {
    pub fn new(
        paths: PapersPaths,
        lock: HermesLock,
        client: reqwest::Client,
        port: Option<u16>,
        token: Option<String>,
    ) -> Self {
        Self {
            paths,
            lock,
            client,
            port,
            token,
        }
    }

    fn hermes_home(&self) -> std::path::PathBuf {
        self.paths.hermes_home.clone()
    }

    fn base(&self) -> Result<String, String> {
        let port = self.port.ok_or_else(|| "Hermes is not running. Start it first.".to_string())?;
        Ok(format!("http://127.0.0.1:{port}"))
    }

    fn token(&self) -> Result<&str, String> {
        self.token
            .as_deref()
            .ok_or_else(|| "Hermes did not report its local session credential.".to_string())
    }

    fn authed(&self, url: &str) -> reqwest::RequestBuilder {
        self.client
            .post(url)
            .bearer_auth(self.token().unwrap_or(""))
    }

    fn authed_get(&self, url: &str) -> reqwest::RequestBuilder {
        self.client
            .get(url)
            .bearer_auth(self.token().unwrap_or(""))
    }

    /// Read Hermes' `auth.json` for an OAuth-provider presence hint. Offline
    /// fallback only — never a "working" claim.
    fn read_auth_json_oauth(&self, provider_id: &str) -> Option<bool> {
        let path = self.hermes_home().join("auth.json");
        let bytes = std::fs::read(path).ok()?;
        let auth: Value = serde_json::from_slice(&bytes).ok()?;
        // Hermes" active_provider + providers map shape.
        let active = auth
            .get("active_provider")
            .and_then(Value::as_str)
            .unwrap_or("");
        if active.eq_ignore_ascii_case(provider_id) {
            return Some(true);
        }
        let providers = auth.get("providers").and_then(Value::as_object)?;
        providers.contains_key(provider_id).then_some(true)
    }

    /// Compose a sanitized snapshot for one provider. Honours the truth
    /// hierarchy: Hermes HTTP when running; file-readable offline hint
    /// (labelled unverified) otherwise.
    pub async fn snapshot(&self, provider_id: &str) -> Result<HermesAuthSnapshot, String> {
        // Offline path first: file hints only.
        if self.port.is_none() {
            let entry = crate::provider_catalog::find(provider_id);
            let configured = match entry.as_ref().map(|e| e.auth_method) {
                Some(crate::provider_catalog::AuthMethod::OauthPortal) => {
                    self.read_auth_json_oauth(provider_id).unwrap_or(false)
                }
                Some(crate::provider_catalog::AuthMethod::ApiKey) => {
                    entry
                        .and_then(|e| e.api_key_descriptor())
                        .map(|desc| self.env_present(desc.env_var))
                        .unwrap_or(false)
                }
                Some(crate::provider_catalog::AuthMethod::Local) => false,
                _ => false,
            };
            let selected_model = self.read_selected_model(provider_id);
            return Ok(HermesAuthSnapshot {
                hermes_running: false,
                configured,
                authenticated: false,
                active_auth_provider: None,
                owned_auth_provider: None,
                available_models: Vec::new(),
                recommended_model: None,
                selected_model,
            });
        }

        // Hermes running: ask it.
        let base = self.base()?;
        let mut available_models = Vec::new();
        let mut authenticated = false;
        let mut owned_auth_provider: Option<String> = None;

        // GET /api/providers/oauth → per-provider logged_in flag.
        if let Ok(resp) = self.authed_get(&format!("{base}/api/providers/oauth")).send().await {
            if resp.status().is_success() {
                if let Ok(body) = resp.json::<Value>().await {
                    if let Some(providers) = body.as_array() {
                        for provider in providers {
                            let id = provider.get("id").and_then(Value::as_str).unwrap_or("");
                            if id == provider_id {
                                let logged_in = provider
                                    .get("status")
                                    .and_then(|s| s.get("logged_in"))
                                    .and_then(Value::as_bool)
                                    .unwrap_or(false);
                                authenticated = logged_in;
                                if logged_in {
                                    owned_auth_provider = Some(provider_id.to_string());
                                }
                            }
                        }
                    }
                }
            }
        }

        // GET /api/model/options → curated models for this provider.
        if let Ok(resp) = self.authed_get(&format!("{base}/api/model/options")).send().await {
            if resp.status().is_success() {
                if let Ok(body) = resp.json::<Value>().await {
                    if let Some(providers) = body.get("providers").and_then(Value::as_array) {
                        for provider in providers {
                            let slug = provider.get("slug").and_then(Value::as_str).unwrap_or("");
                            if slug.eq_ignore_ascii_case(provider_id) {
                                if let Some(models) = provider.get("models").and_then(Value::as_array) {
                                    available_models = models
                                        .iter()
                                        .filter_map(|m| m.as_str().map(String::from))
                                        .collect();
                                }
                            }
                        }
                    }
                }
            }
        }

        let model_cfg = self.read_selected_model(provider_id);
        let configured = authenticated
            || model_cfg.is_some()
            || self.read_auth_json_oauth(provider_id).unwrap_or(false);
        let recommended_model = self.recommended_default(provider_id).await.ok().flatten();

        Ok(HermesAuthSnapshot {
            hermes_running: true,
            configured,
            authenticated,
            active_auth_provider: owned_auth_provider.clone(),
            owned_auth_provider,
            available_models,
            recommended_model,
            selected_model: model_cfg,
        })
    }

    /// GET /api/model/recommended-default?provider=… → onboarding default.
    async fn recommended_default(&self, provider_id: &str) -> Result<Option<String>, String> {
        let base = self.base()?;
        let resp = self
            .authed_get(&format!(
                "{base}/api/model/recommended-default?provider={provider_id}"
            ))
            .send()
            .await
            .map_err(|error| format!("Could not ask Hermes for a recommended model: {error}"))?;
        if !resp.status().is_success() {
            return Ok(None);
        }
        let body: Value = resp
            .json()
            .await
            .map_err(|error| format!("Hermes' recommended-model reply was unreadable: {error}"))?;
        Ok(body
            .get("model")
            .and_then(Value::as_str)
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty()))
    }

    /// Begin an OAuth flow at POST /api/providers/oauth/{id}/start.
    pub async fn start_oauth(&self, provider_id: &str) -> Result<OAuthStartResult, String> {
        let base = self.base()?;
        let resp = self
            .authed(&format!("{base}/api/providers/oauth/{provider_id}/start"))
            .json(&serde_json::json!({}))
            .send()
            .await
            .map_err(|error| format!("Could not start {provider_id} sign-in: {error}"))?
            .error_for_status()
            .map_err(|error| format!("{provider_id} sign-in could not start: {error}"))?;
        let body: Value = resp
            .json()
            .await
            .map_err(|error| format!("{provider_id} returned an unreadable sign-in response: {error}"))?;

        let session_id = body
            .get("session_id")
            .and_then(Value::as_str)
            .ok_or_else(|| format!("{provider_id} did not return a sign-in session identifier"))?
            .to_string();
        let auth_url = body
            .get("auth_url")
            .or_else(|| body.get("verification_url"))
            .and_then(Value::as_str)
            .map(String::from);
        let user_code = body.get("user_code").and_then(Value::as_str).map(String::from);
        let verification_url = body
            .get("verification_url")
            .and_then(Value::as_str)
            .map(String::from);
        let expires_at = body.get("expires_at").and_then(Value::as_str).map(String::from);
        let poll_interval = body
            .get("poll_interval")
            .and_then(Value::as_u64)
            .unwrap_or(2)
            .clamp(1, 10);
        let _ = poll_interval; // adapter doesn't poll here; service does

        // PKCE flows expect a pasted code; device-code/loopback do not.
        let flow = crate::provider_catalog::find(provider_id)
            .and_then(|e| e.flow)
            .map(|f| format!("{f:?}").to_ascii_lowercase());
        let needs_code_submit = matches!(flow.as_deref(), Some("pkce"));

        Ok(OAuthStartResult {
            provider_id: provider_id.to_string(),
            session_id,
            auth_url,
            user_code,
            verification_url,
            expires_at,
            needs_code_submit,
        })
    }

    /// Poll an in-progress OAuth session at GET /api/providers/oauth/{id}/poll/{session_id}.
    pub async fn poll_oauth(&self, provider_id: &str, session_id: &str) -> Result<PollResult, String> {
        let base = self.base()?;
        let resp = self
            .client
            .get(format!("{base}/api/providers/oauth/{provider_id}/poll/{session_id}"))
            .bearer_auth(self.token()?)
            .send()
            .await
            .map_err(|error| format!("Could not poll {provider_id} sign-in: {error}"))?;
        if resp.status() == reqwest::StatusCode::NOT_FOUND {
            return Ok(PollResult {
                status: PollStatus::Missing,
                error_message: Some("That sign-in session is no longer available.".into()),
            });
        }
        if !resp.status().is_success() {
            return Err(format!("{provider_id} reported an unknown polling state."));
        }
        let body: Value = resp
            .json()
            .await
            .map_err(|error| format!("{provider_id} returned an unreadable polling reply: {error}"))?;
        let status = match body.get("status").and_then(Value::as_str) {
            Some("approved") => PollStatus::Approved,
            Some("denied") => PollStatus::Denied,
            Some("expired") => PollStatus::Expired,
            Some("error") => PollStatus::Error,
            Some("pending") | None => PollStatus::Pending,
            Some(other) => return Err(format!("{provider_id} returned unknown poll status \"{other}\".")),
        };
        Ok(PollResult {
            status,
            error_message: body.get("error_message").and_then(Value::as_str).map(String::from),
        })
    }

    /// PKCE-only: submit the callback code at POST /api/providers/oauth/{id}/submit.
    pub async fn submit_pkce_code(&self, provider_id: &str, session_id: &str, code: &str) -> Result<(), String> {
        let base = self.base()?;
        let resp = self
            .authed(&format!("{base}/api/providers/oauth/{provider_id}/submit"))
            .json(&serde_json::json!({ "session_id": session_id, "code": code }))
            .send()
            .await
            .map_err(|error| format!("Could not submit the {provider_id} code: {error}"))?;
        if !resp.status().is_success() {
            return Err(format!("{provider_id} rejected the pasted code."));
        }
        Ok(())
    }

    /// Disconnect a provider at DELETE /api/providers/oauth/{id}.
    pub async fn disconnect_oauth(&self, provider_id: &str) -> Result<(), String> {
        let base = self.base()?;
        let _ = self
            .client
            .delete(format!("{base}/api/providers/oauth/{provider_id}"))
            .bearer_auth(self.token()?)
            .send()
            .await
            .map_err(|error| format!("Could not disconnect {provider_id}: {error}"))?;
        Ok(())
    }

    /// Cancel a pending OAuth session at DELETE /api/providers/oauth/sessions/{id}.
    pub async fn cancel_oauth_session(&self, session_id: &str) -> Result<(), String> {
        let base = self.base()?;
        let _ = self
            .client
            .delete(format!("{base}/api/providers/oauth/sessions/{session_id}"))
            .bearer_auth(self.token()?)
            .send()
            .await
            .map_err(|error| format!("Could not cancel sign-in: {error}"))?;
        Ok(())
    }

    /// Live-probe an API key before saving it via POST /api/providers/validate.
    /// The probe never returns the key back; only ok/reachable/message.
    pub async fn validate_api_key(&self, env_var: &str, key: &str) -> Result<CredentialProbe, String> {
        let base = self.base()?;
        let resp = self
            .authed(&format!("{base}/api/providers/validate"))
            .json(&serde_json::json!({ "key": env_var, "value": key }))
            .send()
            .await
            .map_err(|error| format!("Could not validate the key: {error}"))?;
        if !resp.status().is_success() {
            return Err(format!("Hermes rejected the key probe with HTTP {}.", resp.status()));
        }
        let body: Value = resp
            .json()
            .await
            .map_err(|error| format!("Hermes returned an unreadable validation reply: {error}"))?;
        Ok(CredentialProbe {
            ok: body.get("ok").and_then(Value::as_bool).unwrap_or(false),
            reachable: body.get("reachable").and_then(Value::as_bool).unwrap_or(false),
            message: body
                .get("message")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string(),
        })
    }

    /// Save an API key to Hermes' private .env via PUT /api/env. The key is
    /// handed straight to Hermes; Papers never persists it itself.
    pub async fn save_api_key(&self, env_var: &str, key: &str) -> Result<(), String> {
        let base = self.base()?;
        let resp = self
            .client
            .put(format!("{base}/api/env"))
            .bearer_auth(self.token()?)
            .json(&serde_json::json!({ "key": env_var, "value": key }))
            .send()
            .await
            .map_err(|error| format!("Could not save the key to Hermes: {error}"))?;
        if !resp.status().is_success() {
            return Err(format!("Hermes refused to save the key (HTTP {}).", resp.status()));
        }
        Ok(())
    }

    /// Ask Hermes to set the active model via POST /api/model/set (scope=main).
    pub async fn set_model(&self, provider_id: &str, model: &str) -> Result<(), String> {
        let base = self.base()?;
        let resp = self
            .authed(&format!("{base}/api/model/set"))
            .json(&serde_json::json!({ "scope": "main", "provider": provider_id, "model": model }))
            .send()
            .await
            .map_err(|error| format!("Could not ask Hermes to set the model: {error}"))?;
        if !resp.status().is_success() {
            return Err(format!("Hermes refused to set the model (HTTP {}).", resp.status()));
        }
        Ok(())
    }

    // --- File-read helpers (offline hints only, never strong claims) ---

    /// Read the currently selected model from Hermes' config.yaml for a provider.
    fn read_selected_model(&self, provider_id: &str) -> Option<String> {
        let path = self.hermes_home().join("config.yaml");
        let bytes = std::fs::read(&path).ok()?;
        let config: Value = serde_yaml::from_slice(&bytes).ok()?;
        let model = config.get("model")?;
        let provider_in_cfg = model
            .get("provider")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_ascii_lowercase();
        if provider_in_cfg != provider_id.to_ascii_lowercase() {
            return None;
        }
        model
            .get("default")
            .or_else(|| model.get("model"))
            .and_then(Value::as_str)
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
    }

    /// Best-effort check that Hermes' .env contains a non-empty value for a
    /// var. Offline hint only — never implies the key is valid.
    fn env_present(&self, env_var: &str) -> bool {
        // Hermes' .env lives at <hermes_home>/.env by convention.
        let path = self.hermes_home().join(".env");
        let Ok(bytes) = std::fs::read_to_string(&path) else {
            return false;
        };
        let needle = format!("{env_var}=");
        bytes
            .lines()
            .any(|line| line.trim_start().starts_with(&needle) && line[needle.len()..].trim() != "" && !line.trim_start().starts_with(&format!("{env_var}='#")))
    }
}

