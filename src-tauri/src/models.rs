use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize)]
pub struct BootstrapStatus {
    pub installed: bool,
    pub running: bool,
    pub connected: bool,
    pub phase: String,
    pub package_version: String,
    pub release_tag: String,
    pub hermes_home: String,
    pub install_dir: String,
    pub ws_url: Option<String>,
    pub message: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct PapersSession {
    pub id: String,
    pub hermes_session_id: Option<String>,
    pub title: String,
    pub mode: String,
    pub state: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ChangeRecord {
    pub id: String,
    pub title: String,
    pub request: String,
    pub status: String,
    pub branch: String,
    pub worktree_path: String,
    pub base_commit: String,
    pub accepted_commit: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InspectSelection {
    #[serde(rename = "nodeId")]
    pub node_id: String,
    pub source: String,
    pub tag: String,
    pub role: String,
    pub text: String,
    #[serde(rename = "ariaLabel")]
    pub aria_label: String,
    pub rect: Value,
    pub appearance: Value,
}

#[derive(Debug, Clone, Serialize)]
pub struct PolicyDecision {
    pub action_id: String,
    pub decision: String,
    pub risk: String,
    pub reason: String,
    pub reversible: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionEntry {
    pub id: String,
    pub commit: String,
    pub executable: String,
    pub installed_at: String,
    pub healthy: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct VersionRegistry {
    pub active: Option<String>,
    pub previous: Option<String>,
    pub versions: Vec<VersionEntry>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct HermesLock {
    #[serde(rename = "packageVersion")]
    pub package_version: String,
    #[serde(rename = "releaseTag")]
    pub release_tag: String,
    pub commit: String,
    #[serde(rename = "installerUrl")]
    pub installer_url: String,
    #[serde(rename = "installerSha256")]
    pub installer_sha256: String,
}
