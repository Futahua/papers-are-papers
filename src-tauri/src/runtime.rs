use crate::models::{BootstrapStatus, HermesLock};
use crate::paths::PapersPaths;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::net::TcpListener;
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::time::Duration;

#[cfg(windows)]
use std::os::windows::process::CommandExt;

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x08000000;

struct RuntimeProcess {
    child: Option<Child>,
    port: Option<u16>,
}

#[derive(Clone)]
pub struct RuntimeManager {
    paths: PapersPaths,
    lock: HermesLock,
    process: Arc<Mutex<RuntimeProcess>>,
    client: reqwest::Client,
}

impl RuntimeManager {
    pub fn new(paths: PapersPaths) -> Result<Self, String> {
        let lock: HermesLock = serde_json::from_str(include_str!("../../hermes.lock.json"))
            .map_err(|error| format!("The bundled Hermes lock is invalid: {error}"))?;
        Ok(Self {
            paths,
            lock,
            process: Arc::new(Mutex::new(RuntimeProcess {
                child: None,
                port: None,
            })),
            client: reqwest::Client::builder()
                .timeout(Duration::from_secs(20))
                .build()
                .map_err(|error| error.to_string())?,
        })
    }

    pub async fn status(&self) -> BootstrapStatus {
        let executable = self.paths.hermes_executable();
        let installed = executable.is_file();
        let (running, port) = {
            let mut process = match self.process.lock() {
                Ok(process) => process,
                Err(_) => {
                    return self.make_status(
                        installed,
                        false,
                        false,
                        "error",
                        None,
                        "The Hermes process lock could not be read.",
                    )
                }
            };
            let running = match process.child.as_mut() {
                Some(child) => matches!(child.try_wait(), Ok(None)),
                None => false,
            };
            if !running {
                process.child = None;
                process.port = None;
            }
            (running, process.port)
        };

        let connected = if running {
            if let Some(port) = port {
                self.health(port).await
            } else {
                false
            }
        } else {
            false
        };

        let (phase, message) = if !installed {
            (
                "not_installed",
                format!(
                    "Hermes {} is pinned and ready to install in Papers' private runtime.",
                    self.lock.package_version
                ),
            )
        } else if connected {
            (
                "ready",
                "Hermes is running locally and its control channel is ready.".to_string(),
            )
        } else if running {
            (
                "starting",
                "Hermes is running but has not finished becoming ready.".to_string(),
            )
        } else {
            (
                "stopped",
                "Hermes is installed in Papers' private runtime.".to_string(),
            )
        };

        self.make_status(installed, running, connected, phase, port, &message)
    }

    pub async fn install(&self) -> Result<BootstrapStatus, String> {
        if self.paths.hermes_executable().is_file() {
            self.write_guarded_config()?;
            return Ok(self.status().await);
        }

        std::fs::create_dir_all(&self.paths.runtime).map_err(|error| error.to_string())?;
        let installer_path = self.paths.runtime.join("hermes-install.ps1");
        let bytes = self
            .client
            .get(&self.lock.installer_url)
            .send()
            .await
            .map_err(|error| format!("Could not download the pinned Hermes installer: {error}"))?
            .error_for_status()
            .map_err(|error| format!("Hermes installer download was rejected: {error}"))?
            .bytes()
            .await
            .map_err(|error| format!("Could not read the Hermes installer: {error}"))?;

        let actual_hash = hex::encode(Sha256::digest(&bytes));
        if actual_hash != self.lock.installer_sha256.to_lowercase() {
            return Err(format!(
                "Hermes installer verification failed. Expected {}, received {}. Nothing was run.",
                self.lock.installer_sha256, actual_hash
            ));
        }
        std::fs::write(&installer_path, &bytes)
            .map_err(|error| format!("Could not save the verified Hermes installer: {error}"))?;

        let paths = self.paths.clone();
        let commit = self.lock.commit.clone();
        let install_log = self.paths.logs.join("hermes-install.log");
        tokio::task::spawn_blocking(move || -> Result<(), String> {
            let previous_hermes_home = read_user_environment("HERMES_HOME");
            let previous_path = read_user_environment("Path");
            let log = create_log(&install_log)?;
            let stderr = log
                .try_clone()
                .map_err(|error| format!("Could not clone install log: {error}"))?;
            let mut command = Command::new("powershell.exe");
            command
                .args([
                    "-NoLogo",
                    "-NoProfile",
                    "-ExecutionPolicy",
                    "Bypass",
                    "-File",
                ])
                .arg(&installer_path)
                .arg("-Commit")
                .arg(commit)
                .arg("-HermesHome")
                .arg(&paths.hermes_home)
                .arg("-InstallDir")
                .arg(&paths.hermes_install)
                .args(["-SkipSetup", "-NonInteractive"])
                .env("HERMES_HOME", &paths.hermes_home)
                .stdout(Stdio::from(log))
                .stderr(Stdio::from(stderr));
            hide_console(&mut command);
            let install_result = command
                .status()
                .map_err(|error| format!("Could not run the verified Hermes installer: {error}"));
            let restore_home =
                restore_user_environment("HERMES_HOME", previous_hermes_home.as_deref());
            let restore_path = restore_user_environment("Path", previous_path.as_deref());
            let status = install_result?;
            restore_home?;
            restore_path?;
            if !status.success() {
                return Err(format!(
                    "Hermes installation stopped with {status}. See {}.",
                    install_log.display()
                ));
            }
            Ok(())
        })
        .await
        .map_err(|error| format!("Hermes installation task failed: {error}"))??;

        if !self.paths.hermes_executable().is_file() {
            return Err(
                "The Hermes installer finished without producing its executable. The install log was preserved."
                    .to_string(),
            );
        }

        self.write_guarded_config()?;
        self.install_computer_use().await;
        Ok(self.status().await)
    }

    pub async fn start(&self) -> Result<BootstrapStatus, String> {
        if !self.paths.hermes_executable().is_file() {
            return Err("Hermes is not installed yet.".to_string());
        }

        let already_running = {
            let mut process = self.process.lock().map_err(|_| "Process lock failed")?;
            if let Some(child) = process.child.as_mut() {
                if child
                    .try_wait()
                    .map_err(|error| error.to_string())?
                    .is_none()
                {
                    true
                } else {
                    process.child = None;
                    process.port = None;
                    false
                }
            } else {
                false
            }
        };
        if already_running {
            return Ok(self.status().await);
        }

        self.write_guarded_config()?;
        let port = available_port()?;
        let log_path = self.paths.logs.join("hermes-serve.log");
        let stdout = create_log(&log_path)?;
        let stderr = stdout
            .try_clone()
            .map_err(|error| format!("Could not clone Hermes log: {error}"))?;
        let mut command = Command::new(self.paths.hermes_executable());
        command
            .args(["serve", "--host", "127.0.0.1", "--port"])
            .arg(port.to_string())
            .current_dir(&self.paths.canonical_repo)
            .env("HERMES_HOME", &self.paths.hermes_home)
            .env("HERMES_EXEC_ASK", "1")
            .stdout(Stdio::from(stdout))
            .stderr(Stdio::from(stderr));
        hide_console(&mut command);
        let child = command
            .spawn()
            .map_err(|error| format!("Could not start Hermes: {error}"))?;
        {
            let mut process = self.process.lock().map_err(|_| "Process lock failed")?;
            process.child = Some(child);
            process.port = Some(port);
        }

        for _ in 0..120 {
            if self.health(port).await {
                return Ok(self.status().await);
            }
            let exited = {
                let mut process = self.process.lock().map_err(|_| "Process lock failed")?;
                match process.child.as_mut() {
                    Some(child) => !matches!(child.try_wait(), Ok(None)),
                    None => true,
                }
            };
            if exited {
                return Err(format!(
                    "Hermes stopped while starting. See {}.",
                    log_path.display()
                ));
            }
            tokio::time::sleep(Duration::from_millis(250)).await;
        }
        Err(format!(
            "Hermes did not become ready within 30 seconds. See {}.",
            log_path.display()
        ))
    }

    pub fn stop(&self) -> Result<(), String> {
        let mut process = self.process.lock().map_err(|_| "Process lock failed")?;
        if let Some(child) = process.child.as_mut() {
            child
                .kill()
                .map_err(|error| format!("Could not stop Hermes: {error}"))?;
            let _ = child.wait();
        }
        process.child = None;
        process.port = None;
        Ok(())
    }

    pub async fn start_nous_login(&self) -> Result<String, String> {
        let status = self.start().await?;
        let port = status
            .ws_url
            .as_ref()
            .and_then(|url| url.split(':').nth(2))
            .and_then(|part| part.split('/').next())
            .and_then(|part| part.parse::<u16>().ok())
            .ok_or_else(|| "Hermes did not report its local port".to_string())?;
        let response: Value = self
            .client
            .post(format!(
                "http://127.0.0.1:{port}/api/providers/oauth/nous/start"
            ))
            .json(&serde_json::json!({}))
            .send()
            .await
            .map_err(|error| format!("Could not start Nous sign-in: {error}"))?
            .error_for_status()
            .map_err(|error| format!("Nous sign-in could not start: {error}"))?
            .json()
            .await
            .map_err(|error| format!("Nous returned an unreadable sign-in response: {error}"))?;

        let url = response
            .get("auth_url")
            .or_else(|| response.get("verification_url"))
            .and_then(Value::as_str)
            .ok_or_else(|| "Nous did not return a browser sign-in address".to_string())?;
        open::that(url).map_err(|error| format!("Could not open Nous sign-in: {error}"))?;
        let session_id = response
            .get("session_id")
            .and_then(Value::as_str)
            .ok_or_else(|| "Nous did not return a sign-in session identifier".to_string())?;
        let code = response
            .get("user_code")
            .and_then(Value::as_str)
            .map(|value| value.to_string());
        let poll_interval = response
            .get("poll_interval")
            .and_then(Value::as_u64)
            .unwrap_or(2)
            .clamp(1, 10);
        let expires_in = response
            .get("expires_in")
            .and_then(Value::as_u64)
            .unwrap_or(300)
            .clamp(30, 600);
        let attempts = expires_in / poll_interval;
        for _ in 0..attempts {
            tokio::time::sleep(Duration::from_secs(poll_interval)).await;
            let poll = self
                .client
                .get(format!(
                    "http://127.0.0.1:{port}/api/providers/oauth/nous/poll/{session_id}"
                ))
                .send()
                .await;
            let Ok(response) = poll else { continue };
            let Ok(status) = response.json::<Value>().await else {
                continue;
            };
            match status.get("status").and_then(Value::as_str) {
                Some("approved") => {
                    return Ok(
                        "Nous is connected. Hermes can now use your selected model.".to_string()
                    )
                }
                Some("denied") => return Err("Nous sign-in was denied.".to_string()),
                Some("expired") => return Err("Nous sign-in expired. Start it again.".to_string()),
                Some("error") => {
                    return Err(status
                        .get("error_message")
                        .and_then(Value::as_str)
                        .unwrap_or("Nous sign-in failed.")
                        .to_string())
                }
                _ => {}
            }
        }
        Err(match code {
            Some(code) => format!(
                "Nous sign-in did not complete in time. The browser code was {code}; start sign-in again."
            ),
            None => "Nous sign-in did not complete in time. Start it again.".to_string(),
        })
    }

    async fn install_computer_use(&self) {
        let executable = self.paths.hermes_executable();
        let home = self.paths.hermes_home.clone();
        let log_path = self.paths.logs.join("computer-use-install.log");
        let _ = tokio::task::spawn_blocking(move || {
            let log = create_log(&log_path)?;
            let stderr = log.try_clone().map_err(|error| error.to_string())?;
            let mut command = Command::new(executable);
            command
                .args(["computer-use", "install"])
                .env("HERMES_HOME", home)
                .stdout(Stdio::from(log))
                .stderr(Stdio::from(stderr));
            hide_console(&mut command);
            command
                .status()
                .map(|_| ())
                .map_err(|error| error.to_string())
        })
        .await;
    }

    fn write_guarded_config(&self) -> Result<(), String> {
        std::fs::create_dir_all(&self.paths.hermes_home).map_err(|error| error.to_string())?;
        let main_config = self.paths.hermes_home.join("config.yaml");
        let mut config: serde_json::Value = if main_config.exists() {
            serde_yaml::from_slice(
                &std::fs::read(&main_config)
                    .map_err(|error| format!("Could not read Hermes settings: {error}"))?,
            )
            .unwrap_or_else(|_| serde_json::json!({}))
        } else {
            serde_json::json!({})
        };
        if !config.is_object() {
            config = serde_json::json!({});
        }
        let root = config.as_object_mut().expect("object checked above");
        root.insert(
            "approvals".to_string(),
            serde_json::json!({ "mode": "manual" }),
        );
        root.insert(
            "toolsets".to_string(),
            serde_json::json!(["hermes-cli", "computer_use"]),
        );
        root.entry("agent".to_string())
            .or_insert_with(|| serde_json::json!({ "yolo": false }));
        std::fs::write(
            &main_config,
            serde_yaml::to_string(&config)
                .map_err(|error| format!("Could not serialize Hermes settings: {error}"))?,
        )
        .map_err(|error| format!("Could not save guarded Hermes settings: {error}"))?;

        let builder_home = self
            .paths
            .hermes_home
            .join("profiles")
            .join("papers-builder");
        std::fs::create_dir_all(&builder_home).map_err(|error| error.to_string())?;
        let mcp_executable = std::env::current_exe()
            .map_err(|error| error.to_string())?
            .with_file_name(if cfg!(windows) {
                "papers-mcp.exe"
            } else {
                "papers-mcp"
            });
        let config = serde_json::json!({
            "approvals": { "mode": "manual" },
            "toolsets": ["mcp-papers", "todo", "clarify"],
            "agent": {
                "yolo": false,
                "disabled_toolsets": [
                    "terminal", "file", "browser", "computer_use", "code_execution",
                    "delegation", "cronjob", "messaging"
                ]
            },
            "mcp_servers": {
                "papers": {
                    "command": mcp_executable.to_string_lossy(),
                    "args": [],
                    "env": {
                        "PAPERS_STAGING_ROOT": self.paths.staging.to_string_lossy()
                    }
                }
            }
        });
        let yaml = serde_yaml::to_string(&config)
            .map_err(|error| format!("Could not create builder profile: {error}"))?;
        std::fs::write(builder_home.join("config.yaml"), yaml)
            .map_err(|error| format!("Could not save builder profile: {error}"))?;
        Ok(())
    }

    async fn health(&self, port: u16) -> bool {
        self.client
            .get(format!("http://127.0.0.1:{port}/api/status"))
            .send()
            .await
            .map(|response| response.status().is_success())
            .unwrap_or(false)
    }

    fn make_status(
        &self,
        installed: bool,
        running: bool,
        connected: bool,
        phase: &str,
        port: Option<u16>,
        message: &str,
    ) -> BootstrapStatus {
        BootstrapStatus {
            installed,
            running,
            connected,
            phase: phase.to_string(),
            package_version: self.lock.package_version.clone(),
            release_tag: self.lock.release_tag.clone(),
            hermes_home: self.paths.hermes_home.to_string_lossy().into_owned(),
            install_dir: self.paths.hermes_install.to_string_lossy().into_owned(),
            ws_url: port.map(|port| format!("ws://127.0.0.1:{port}/api/ws")),
            message: message.to_string(),
        }
    }
}

fn available_port() -> Result<u16, String> {
    TcpListener::bind(("127.0.0.1", 0))
        .and_then(|listener| listener.local_addr())
        .map(|address| address.port())
        .map_err(|error| format!("Could not reserve a local Hermes port: {error}"))
}

fn create_log(path: &std::path::Path) -> Result<File, String> {
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(|error| format!("Could not open {}: {error}", path.display()))?;
    let _ = writeln!(file, "\n--- {} ---", chrono::Utc::now().to_rfc3339());
    Ok(file)
}

fn hide_console(command: &mut Command) {
    #[cfg(windows)]
    command.creation_flags(CREATE_NO_WINDOW);
}

fn read_user_environment(name: &str) -> Option<String> {
    let script = format!(
        "[Environment]::GetEnvironmentVariable('{}', 'User')",
        name.replace('\'', "''")
    );
    let mut command = Command::new("powershell.exe");
    command.args(["-NoLogo", "-NoProfile", "-Command", &script]);
    hide_console(&mut command);
    command
        .output()
        .ok()
        .filter(|output| output.status.success())
        .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_string())
        .filter(|value| !value.is_empty())
}

fn restore_user_environment(name: &str, value: Option<&str>) -> Result<(), String> {
    let escaped_name = name.replace('\'', "''");
    let script = if value.is_some() {
        format!(
            "[Environment]::SetEnvironmentVariable('{escaped_name}', $env:PAPERS_RESTORE_VALUE, 'User')"
        )
    } else {
        format!("[Environment]::SetEnvironmentVariable('{escaped_name}', $null, 'User')")
    };
    let mut command = Command::new("powershell.exe");
    command.args(["-NoLogo", "-NoProfile", "-Command", &script]);
    if let Some(value) = value {
        command.env("PAPERS_RESTORE_VALUE", value);
    }
    hide_console(&mut command);
    let status = command
        .status()
        .map_err(|error| format!("Could not restore the user's {name} setting: {error}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!(
            "Hermes installed, but Papers could not restore the user's {name} setting."
        ))
    }
}
