#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::process::{Child, Command};
use std::thread;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct VersionEntry {
    id: String,
    commit: String,
    executable: String,
    installed_at: String,
    healthy: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct VersionRegistry {
    active: Option<String>,
    previous: Option<String>,
    versions: Vec<VersionEntry>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct LauncherState {
    consecutive_failures: u32,
    last_failed_version: Option<String>,
    updated_at: String,
}

fn main() {
    if let Err(error) = run() {
        let _ = show_error(&error);
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let root = std::env::var_os("PAPERS_DATA_HOME")
        .map(PathBuf::from)
        .or_else(|| dirs::data_local_dir().map(|path| path.join("Papers")))
        .ok_or_else(|| "Windows did not provide a local application-data directory.".to_string())?;
    let registry_path = root.join("data").join("versions.json");
    let state_path = root.join("data").join("launcher-state.json");
    let mut registry = read_registry(&registry_path)?;
    let active = resolve_version(&registry, registry.active.as_deref())
        .ok_or_else(|| "No healthy Papers version has been activated yet.".to_string())?
        .clone();

    match launch_and_check(&root, &active) {
        Ok(_) => {
            write_state(
                &state_path,
                &LauncherState {
                    consecutive_failures: 0,
                    last_failed_version: None,
                    updated_at: chrono::Utc::now().to_rfc3339(),
                },
            )?;
            Ok(())
        }
        Err(primary_error) => {
            let previous = resolve_version(&registry, registry.previous.as_deref())
                .filter(|version| version.id != active.id)
                .cloned()
                .ok_or_else(|| {
                    format!("{primary_error}\nNo previous healthy Papers version is available.")
                })?;
            registry.active = Some(previous.id.clone());
            registry.previous = Some(active.id.clone());
            atomic_json(&registry_path, &registry)?;
            let mut state = read_state(&state_path);
            state.consecutive_failures = state.consecutive_failures.saturating_add(1);
            state.last_failed_version = Some(active.id);
            state.updated_at = chrono::Utc::now().to_rfc3339();
            write_state(&state_path, &state)?;
            launch_and_check(&root, &previous).map(|_| ())
        }
    }
}

fn launch_and_check(root: &Path, version: &VersionEntry) -> Result<Child, String> {
    let executable = PathBuf::from(&version.executable);
    if !executable.is_file() {
        return Err(format!(
            "Papers version {} is missing its executable.",
            version.id
        ));
    }
    let health = root
        .join("data")
        .join(format!("health-launch-{}.ready", version.id));
    let _ = std::fs::remove_file(&health);
    let mut child = Command::new(&executable)
        .env("PAPERS_VERSION_ID", &version.id)
        .env("PAPERS_HEALTH_FILE", &health)
        .spawn()
        .map_err(|error| format!("Could not launch Papers {}: {error}", version.id))?;
    let deadline = Instant::now() + Duration::from_secs(15);
    while Instant::now() < deadline {
        if health.is_file() {
            return Ok(child);
        }
        if let Some(status) = child.try_wait().map_err(|error| error.to_string())? {
            return Err(format!(
                "Papers {} exited before its health check ({status}).",
                version.id
            ));
        }
        thread::sleep(Duration::from_millis(150));
    }
    let _ = child.kill();
    Err(format!(
        "Papers {} did not report healthy within 15 seconds.",
        version.id
    ))
}

fn resolve_version<'a>(
    registry: &'a VersionRegistry,
    id: Option<&str>,
) -> Option<&'a VersionEntry> {
    let id = id?;
    registry
        .versions
        .iter()
        .find(|version| version.id == id && version.healthy)
}

fn read_registry(path: &Path) -> Result<VersionRegistry, String> {
    serde_json::from_slice(&std::fs::read(path).map_err(|error| {
        format!(
            "Could not read Papers recovery state at {}: {error}",
            path.display()
        )
    })?)
    .map_err(|error| format!("Papers recovery state is invalid: {error}"))
}

fn read_state(path: &Path) -> LauncherState {
    std::fs::read(path)
        .ok()
        .and_then(|bytes| serde_json::from_slice(&bytes).ok())
        .unwrap_or_default()
}

fn write_state(path: &Path, state: &LauncherState) -> Result<(), String> {
    atomic_json(path, state)
}

fn atomic_json<T: Serialize>(path: &Path, value: &T) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    let temporary = path.with_extension("json.tmp");
    std::fs::write(
        &temporary,
        serde_json::to_vec_pretty(value).map_err(|error| error.to_string())?,
    )
    .map_err(|error| error.to_string())?;
    std::fs::rename(&temporary, path).map_err(|error| error.to_string())
}

fn show_error(error: &str) -> Result<(), String> {
    let path = std::env::temp_dir().join("papers-launcher-error.txt");
    std::fs::write(&path, error).map_err(|write_error| write_error.to_string())?;
    #[cfg(windows)]
    {
        let _ = Command::new("notepad.exe").arg(path).spawn();
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_only_healthy_active_version() {
        let registry = VersionRegistry {
            active: Some("broken".into()),
            previous: Some("good".into()),
            versions: vec![
                VersionEntry {
                    id: "broken".into(),
                    commit: "a".into(),
                    executable: "broken.exe".into(),
                    installed_at: String::new(),
                    healthy: false,
                },
                VersionEntry {
                    id: "good".into(),
                    commit: "b".into(),
                    executable: "good.exe".into(),
                    installed_at: String::new(),
                    healthy: true,
                },
            ],
        };
        assert!(resolve_version(&registry, registry.active.as_deref()).is_none());
        assert_eq!(
            resolve_version(&registry, registry.previous.as_deref())
                .unwrap()
                .id,
            "good"
        );
    }
}
