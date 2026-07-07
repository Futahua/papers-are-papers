use crate::models::{AgentProviderStatus, BootstrapStatus, HermesLock};
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

const DEFAULT_PROVIDER: &str = "nous";
const DEFAULT_MODEL: &str = "stepfun/step-3.7-flash:free";

struct RuntimeProcess {
    child: Option<Child>,
    port: Option<u16>,
    session_token: Option<String>,
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
                session_token: None,
            })),
            client: reqwest::Client::builder()
                .timeout(Duration::from_secs(20))
                .build()
                .map_err(|error| error.to_string())?,
        })
    }

    /// Cheap-cloned inputs the provider adapter needs to talk to Hermes when it
    /// is running. Returns the current port + session token (both None when
    /// Hermes is stopped). Paid only on a provider-settings call, never a hot
    /// path, so cloning PapersPaths/HermesLock/reqwest::Client here is fine.
    pub fn provider_adapter_inputs(
        &self,
    ) -> (PapersPaths, HermesLock, reqwest::Client, Option<u16>, Option<String>) {
        let (port, token) = match self.process.lock() {
            Ok(process) => (process.port, process.session_token.clone()),
            Err(_) => (None, None),
        };
        (
            self.paths.clone(),
            self.lock.clone(),
            self.client.clone(),
            port,
            token,
        )
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
                process.session_token = None;
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
            self.install_computer_use().await?;
            return Ok(self.status().await);
        }

        self.preserve_partial_install()?;
        let git_config = self.write_installer_git_config()?;
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
                .env("GIT_CONFIG_GLOBAL", &git_config)
                .env("PLAYWRIGHT_BROWSERS_PATH", paths.runtime.join("playwright"))
                .stdout(Stdio::from(log))
                .stderr(Stdio::from(stderr));
            if let Some(path) = installer_path_without_package_managers() {
                command.env("PATH", path);
            }
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
        self.install_computer_use().await?;
        Ok(self.status().await)
    }

    fn preserve_partial_install(&self) -> Result<(), String> {
        if !self.paths.hermes_install.exists() || self.paths.hermes_executable().is_file() {
            return Ok(());
        }

        let backup = self.paths.staging.join(format!(
            "hermes-agent-partial-{}",
            chrono::Utc::now().timestamp_millis()
        ));
        std::fs::rename(&self.paths.hermes_install, &backup).map_err(|error| {
            format!(
                "Could not preserve the incomplete Hermes install at {}: {error}",
                backup.display()
            )
        })
    }

    fn write_installer_git_config(&self) -> Result<std::path::PathBuf, String> {
        let path = self.paths.data.join("installer.gitconfig");
        std::fs::write(
            &path,
            "[core]\n\tautocrlf = false\n[windows]\n\tappendAtomically = false\n",
        )
        .map_err(|error| {
            format!(
                "Could not prepare Papers' isolated Git settings at {}: {error}",
                path.display()
            )
        })?;
        Ok(path)
    }

    pub async fn start(&self) -> Result<BootstrapStatus, String> {
        if !self.paths.hermes_executable().is_file() {
            return Err("Hermes is not installed yet.".to_string());
        }
        self.install_computer_use().await?;

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
                    process.session_token = None;
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
        let session_token = format!("{}{}", uuid::Uuid::new_v4(), uuid::Uuid::new_v4());
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
            .env("HERMES_DASHBOARD_SESSION_TOKEN", &session_token)
            .env(
                "PLAYWRIGHT_BROWSERS_PATH",
                self.paths.runtime.join("playwright"),
            )
            .env("HERMES_CUA_DRIVER_CMD", self.computer_use_executable())
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
            process.session_token = Some(session_token);
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
        process.session_token = None;
        Ok(())
    }

    pub async fn start_nous_login(&self) -> Result<String, String> {
        self.start().await?;
        let (port, session_token) = self.gateway_credentials()?;
        let response: Value = self
            .client
            .post(format!(
                "http://127.0.0.1:{port}/api/providers/oauth/nous/start"
            ))
            .bearer_auth(&session_token)
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
                .bearer_auth(&session_token)
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

    pub async fn provider_status(&self) -> AgentProviderStatus {
        let config_path = self.paths.hermes_home.join("config.yaml");
        let config = self.read_main_config().unwrap_or_else(|_| serde_json::json!({}));
        let (configured_provider, model) = model_config_values(&config);
        let auth_provider = self.active_auth_provider();
        let provider = if configured_provider.is_empty() || configured_provider == "auto" {
            auth_provider
                .clone()
                .unwrap_or_else(|| DEFAULT_PROVIDER.to_string())
        } else {
            configured_provider.clone()
        };
        let authenticated = self.provider_authenticated(&provider, auth_provider.as_deref());
        let runtime = self.status().await;
        let runtime_ready = runtime.connected;
        let message = if !runtime.installed {
            "Hermes is not installed yet. Install the local agent engine first.".to_string()
        } else if !authenticated {
            format!("{provider} is selected, but Papers has not verified a saved sign-in for it.")
        } else if runtime_ready {
            format!("{provider} / {model} is selected and Hermes is reachable.")
        } else {
            format!("{provider} / {model} is selected. Start Hermes to run a live test.")
        };

        AgentProviderStatus {
            provider,
            model,
            configured_provider,
            auth_provider,
            authenticated,
            runtime_ready,
            hermes_home: self.paths.hermes_home.to_string_lossy().into_owned(),
            config_path: config_path.to_string_lossy().into_owned(),
            known_providers: known_providers(),
            suggested_models: suggested_models(),
            message,
        }
    }

    pub async fn set_provider_model(
        &self,
        provider: String,
        model: String,
    ) -> Result<AgentProviderStatus, String> {
        let provider = normalize_provider(&provider)?;
        let model = normalize_model(&model)?;
        let mut config = self.read_main_config().unwrap_or_else(|_| serde_json::json!({}));
        if !config.is_object() {
            config = serde_json::json!({});
        }
        let root = config.as_object_mut().expect("object checked above");
        let previous_model = root.get("model").cloned();
        let mut model_section = match previous_model {
            Some(Value::Object(map)) => map,
            Some(Value::String(existing)) if !existing.trim().is_empty() => {
                let mut map = serde_json::Map::new();
                map.insert("default".to_string(), Value::String(existing));
                map
            }
            _ => serde_json::Map::new(),
        };
        model_section.insert("default".to_string(), Value::String(model));
        model_section.insert("provider".to_string(), Value::String(provider));
        for stale_key in ["api_key", "base_url", "api_mode", "auth_mode"] {
            model_section.remove(stale_key);
        }
        root.insert("model".to_string(), Value::Object(model_section));
        self.write_main_config(&config)?;
        self.write_guarded_config()?;
        Ok(self.provider_status().await)
    }

    pub async fn start_provider_login(&self, provider: String) -> Result<String, String> {
        let provider = normalize_provider(&provider)?;
        match provider.as_str() {
            "auto" | "nous" => self.start_nous_login().await,
            other => Err(format!(
                "Papers can open Nous sign-in today. {other} credentials stay Hermes-owned; use Hermes setup for that provider until Papers adds its safe login wrapper."
            )),
        }
    }

    pub async fn validate_provider(&self) -> AgentProviderStatus {
        let mut status = self.provider_status().await;
        if status.model.trim().is_empty() {
            status.message =
                "No model is selected. Choose a model before sending a live test.".to_string();
        } else if status.runtime_ready {
            status.message =
                "Config is readable and Hermes is ready. Run the live test prompt next.".to_string();
        } else {
            status.message =
                "Config is readable. Start Hermes before running the live test prompt.".to_string();
        }
        status
    }

    pub fn gateway_url(&self) -> Result<String, String> {
        let (port, token) = self.gateway_credentials()?;
        Ok(format!("ws://127.0.0.1:{port}/api/ws?token={token}"))
    }

    fn gateway_credentials(&self) -> Result<(u16, String), String> {
        let process = self.process.lock().map_err(|_| "Process lock failed")?;
        let port = process
            .port
            .ok_or_else(|| "Hermes did not report its local port".to_string())?;
        let token = process
            .session_token
            .clone()
            .ok_or_else(|| "Hermes did not report its local session credential".to_string())?;
        Ok((port, token))
    }

    fn read_main_config(&self) -> Result<Value, String> {
        let path = self.paths.hermes_home.join("config.yaml");
        if !path.exists() {
            return Ok(serde_json::json!({}));
        }
        serde_yaml::from_slice(
            &std::fs::read(&path)
                .map_err(|error| format!("Could not read Hermes settings: {error}"))?,
        )
        .map_err(|error| format!("Could not parse Hermes settings: {error}"))
    }

    fn write_main_config(&self, config: &Value) -> Result<(), String> {
        std::fs::create_dir_all(&self.paths.hermes_home).map_err(|error| error.to_string())?;
        let path = self.paths.hermes_home.join("config.yaml");
        let tmp = path.with_extension("yaml.tmp");
        let yaml = serde_yaml::to_string(config)
            .map_err(|error| format!("Could not serialize Hermes settings: {error}"))?;
        std::fs::write(&tmp, yaml)
            .map_err(|error| format!("Could not stage Hermes settings: {error}"))?;
        std::fs::rename(&tmp, &path)
            .map_err(|error| format!("Could not save Hermes settings: {error}"))
    }

    fn active_auth_provider(&self) -> Option<String> {
        let path = self.paths.hermes_home.join("auth.json");
        let bytes = std::fs::read(path).ok()?;
        let auth: Value = serde_json::from_slice(&bytes).ok()?;
        auth.get("active_provider")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToString::to_string)
    }

    fn provider_authenticated(&self, provider: &str, auth_provider: Option<&str>) -> bool {
        if auth_provider
            .map(|active| active.eq_ignore_ascii_case(provider))
            .unwrap_or(false)
        {
            return true;
        }
        if provider == "auto" && auth_provider.is_some() {
            return true;
        }
        let path = self.paths.hermes_home.join("auth.json");
        let Ok(bytes) = std::fs::read(path) else {
            return false;
        };
        let Ok(auth) = serde_json::from_slice::<Value>(&bytes) else {
            return false;
        };
        auth.get("providers")
            .and_then(Value::as_object)
            .map(|providers| providers.contains_key(provider))
            .unwrap_or(false)
    }

    async fn install_computer_use(&self) -> Result<(), String> {
        let executable = self.computer_use_executable();
        if executable.is_file() {
            return Ok(());
        }

        let install_root = self
            .paths
            .runtime
            .join("computer-use")
            .join(&self.lock.computer_use.version);
        if install_root.exists() {
            let backup = self.paths.staging.join(format!(
                "computer-use-partial-{}",
                chrono::Utc::now().timestamp_millis()
            ));
            std::fs::rename(&install_root, &backup).map_err(|error| {
                format!(
                    "Could not preserve the incomplete Computer Use install at {}: {error}",
                    backup.display()
                )
            })?;
        }

        std::fs::create_dir_all(&install_root)
            .map_err(|error| format!("Could not prepare Computer Use: {error}"))?;
        let archive = self
            .client
            .get(&self.lock.computer_use.archive_url)
            .timeout(Duration::from_secs(180))
            .send()
            .await
            .map_err(|error| format!("Could not download Computer Use: {error}"))?
            .error_for_status()
            .map_err(|error| format!("Computer Use download was rejected: {error}"))?
            .bytes()
            .await
            .map_err(|error| format!("Could not read the Computer Use archive: {error}"))?;
        let actual_hash = hex::encode(Sha256::digest(&archive));
        if actual_hash != self.lock.computer_use.archive_sha256.to_lowercase() {
            return Err(format!(
                "Computer Use verification failed. Expected {}, received {}. Nothing was extracted.",
                self.lock.computer_use.archive_sha256, actual_hash
            ));
        }

        let archive_path = self
            .paths
            .staging
            .join(format!("cua-driver-{}.zip", self.lock.computer_use.version));
        std::fs::write(&archive_path, &archive)
            .map_err(|error| format!("Could not stage Computer Use: {error}"))?;
        let log_path = self.paths.logs.join("computer-use-install.log");
        let destination = install_root.clone();
        tokio::task::spawn_blocking(move || -> Result<(), String> {
            let log = create_log(&log_path)?;
            let stderr = log.try_clone().map_err(|error| error.to_string())?;
            let mut command = Command::new("powershell.exe");
            command
                .args([
                    "-NoLogo",
                    "-NoProfile",
                    "-Command",
                    "Expand-Archive -LiteralPath $env:PAPERS_CUA_ARCHIVE -DestinationPath $env:PAPERS_CUA_DESTINATION -Force",
                ])
                .env("PAPERS_CUA_ARCHIVE", &archive_path)
                .env("PAPERS_CUA_DESTINATION", &destination)
                .stdout(Stdio::from(log))
                .stderr(Stdio::from(stderr));
            hide_console(&mut command);
            let status = command
                .status()
                .map_err(|error| format!("Could not extract Computer Use: {error}"))?;
            if status.success() {
                Ok(())
            } else {
                Err(format!(
                    "Computer Use extraction stopped with {status}. See {}.",
                    log_path.display()
                ))
            }
        })
        .await
        .map_err(|error| format!("Computer Use installation task failed: {error}"))??;

        if !executable.is_file() {
            return Err(format!(
                "Computer Use {} was extracted but {} is missing.",
                self.lock.computer_use.release_tag,
                executable.display()
            ));
        }
        Ok(())
    }

    fn computer_use_executable(&self) -> std::path::PathBuf {
        self.paths
            .runtime
            .join("computer-use")
            .join(&self.lock.computer_use.version)
            .join(&self.lock.computer_use.archive_root)
            .join("cua-driver.exe")
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
            ws_url: port.map(|_| "native://hermes".to_string()),
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

fn model_config_values(config: &Value) -> (String, String) {
    match config.get("model") {
        Some(Value::Object(map)) => {
            let provider = map
                .get("provider")
                .and_then(Value::as_str)
                .unwrap_or(DEFAULT_PROVIDER)
                .trim()
                .to_string();
            let model = map
                .get("default")
                .or_else(|| map.get("model"))
                .and_then(Value::as_str)
                .unwrap_or(DEFAULT_MODEL)
                .trim()
                .to_string();
            (provider, model)
        }
        Some(Value::String(model)) if !model.trim().is_empty() => {
            (DEFAULT_PROVIDER.to_string(), model.trim().to_string())
        }
        _ => (DEFAULT_PROVIDER.to_string(), DEFAULT_MODEL.to_string()),
    }
}

fn normalize_provider(value: &str) -> Result<String, String> {
    let provider = value.trim().to_ascii_lowercase();
    if provider.is_empty() {
        return Err("Choose a provider first.".to_string());
    }
    if provider.len() > 64
        || !provider
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.' | ':'))
    {
        return Err("Provider names may only use letters, numbers, dash, underscore, dot, or colon.".to_string());
    }
    Ok(provider)
}

fn normalize_model(value: &str) -> Result<String, String> {
    let model = value.trim();
    if model.is_empty() {
        return Err("Choose a model first.".to_string());
    }
    if model.len() > 220 || model.contains('\n') || model.contains('\r') {
        return Err("Model names must be a single short line.".to_string());
    }
    Ok(model.to_string())
}

fn known_providers() -> Vec<String> {
    [
        "nous",
        "openrouter",
        "anthropic",
        "openai",
        "google",
        "xai",
        "ollama",
        "auto",
    ]
    .into_iter()
    .map(ToString::to_string)
    .collect()
}

fn suggested_models() -> Vec<String> {
    [DEFAULT_MODEL]
        .into_iter()
        .map(ToString::to_string)
        .collect()
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

fn installer_path_without_package_managers() -> Option<std::ffi::OsString> {
    let current = std::env::var_os("PATH")?;
    let entries = std::env::split_paths(&current)
        .filter(|entry| installer_path_entry_allowed(entry))
        .collect::<Vec<_>>();
    std::env::join_paths(entries).ok()
}

fn installer_path_entry_allowed(entry: &std::path::Path) -> bool {
    let normalized = entry.to_string_lossy().replace('/', "\\").to_lowercase();
    !normalized.contains("\\windowsapps")
        && !normalized.contains("\\chocolatey\\bin")
        && !normalized.contains("\\scoop\\shims")
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

#[cfg(test)]
mod tests {
    use super::installer_path_entry_allowed;
    use std::path::Path;

    #[test]
    fn installer_path_hides_system_package_manager_shims() {
        assert!(!installer_path_entry_allowed(Path::new(
            r"C:\Users\person\AppData\Local\Microsoft\WindowsApps"
        )));
        assert!(!installer_path_entry_allowed(Path::new(
            r"C:\ProgramData\chocolatey\bin"
        )));
        assert!(!installer_path_entry_allowed(Path::new(
            r"C:\Users\person\scoop\shims"
        )));
        assert!(installer_path_entry_allowed(Path::new(
            r"C:\Program Files\Git\cmd"
        )));
        assert!(installer_path_entry_allowed(Path::new(
            r"C:\Program Files\nodejs"
        )));
    }
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
