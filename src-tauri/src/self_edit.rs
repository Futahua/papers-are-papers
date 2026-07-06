use crate::models::{ChangeRecord, InspectSelection, VersionEntry, VersionRegistry};
use crate::paths::PapersPaths;
use crate::storage::Database;
use chrono::Utc;
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};
use uuid::Uuid;

#[derive(Clone)]
pub struct SelfEditService {
    paths: PapersPaths,
    database: Database,
}

impl SelfEditService {
    pub fn new(paths: PapersPaths, database: Database) -> Self {
        Self { paths, database }
    }

    pub fn create(
        &self,
        title: &str,
        request: &str,
        selection: Option<InspectSelection>,
    ) -> Result<ChangeRecord, String> {
        self.ensure_repository()?;
        self.ensure_clean_main()?;
        let base_commit = git_output(&self.paths.canonical_repo, &["rev-parse", "HEAD"])?;
        let short = Uuid::new_v4().simple().to_string()[..10].to_string();
        let id = format!("change-{short}");
        let branch = format!("papers/{id}");
        let worktree = self.paths.staging.join(&id);
        if worktree.exists() {
            return Err("The requested staging directory already exists.".to_string());
        }

        git_status(
            &self.paths.canonical_repo,
            &[
                "worktree",
                "add",
                "-b",
                &branch,
                worktree
                    .to_str()
                    .ok_or_else(|| "The staging path is not valid Unicode".to_string())?,
                &base_commit,
            ],
        )?;

        let now = Utc::now().to_rfc3339();
        let record = ChangeRecord {
            id,
            title: clean_title(title),
            request: request.trim().to_string(),
            status: "staging".to_string(),
            branch,
            worktree_path: worktree.to_string_lossy().into_owned(),
            base_commit,
            accepted_commit: None,
            created_at: now.clone(),
            updated_at: now,
        };
        let selection_json = selection
            .as_ref()
            .and_then(|selection| serde_json::to_value(selection).ok());
        self.database
            .insert_change(&record, selection_json.as_ref())?;
        Ok(record)
    }

    pub fn list(&self) -> Result<Vec<ChangeRecord>, String> {
        self.database.list_changes()
    }

    pub fn build(&self, id: &str) -> Result<ChangeRecord, String> {
        let record = self.database.change(id)?;
        if !matches!(record.status.as_str(), "staging" | "failed") {
            return Err(format!(
                "A change in state '{}' cannot be built.",
                record.status
            ));
        }
        let worktree = self.checked_worktree(&record)?;
        self.database.update_change(id, "building", None)?;
        let log_path = self.paths.logs.join(format!("{id}-build.log"));

        let result = (|| {
            run_logged(
                "npm.cmd",
                &["ci", "--no-audit", "--no-fund"],
                &worktree,
                &log_path,
            )?;
            run_logged("npm.cmd", &["run", "build"], &worktree, &log_path)?;
            run_logged(
                "cargo.exe",
                &["test", "--locked"],
                &worktree.join("src-tauri"),
                &log_path,
            )?;
            run_logged(
                "cargo.exe",
                &["build", "--locked"],
                &worktree.join("src-tauri"),
                &log_path,
            )?;
            let executable = candidate_executable(&worktree);
            if !executable.is_file() {
                return Err(format!(
                    "The build completed without producing {}.",
                    executable.display()
                ));
            }
            Ok(())
        })();

        match result {
            Ok(()) => self.database.update_change(id, "preview_ready", None),
            Err(error) => {
                let _ = self.database.update_change(id, "failed", None);
                Err(format!("{error} Build details: {}", log_path.display()))
            }
        }
    }

    pub fn launch_preview(&self, id: &str) -> Result<ChangeRecord, String> {
        let record = self.database.change(id)?;
        if record.status != "preview_ready" {
            return Err("Build this temporary version before experiencing it.".to_string());
        }
        let worktree = self.checked_worktree(&record)?;
        let executable = candidate_executable(&worktree);
        if !executable.is_file() {
            return Err("The temporary executable is missing. Build it again.".to_string());
        }
        let preview_data = self.paths.staging.join(&record.id).join(".preview-data");
        std::fs::create_dir_all(&preview_data).map_err(|error| error.to_string())?;
        Command::new(executable)
            .env("PAPERS_PREVIEW", "1")
            .env("PAPERS_DATA_HOME", preview_data)
            .spawn()
            .map_err(|error| format!("Could not launch the temporary Papers version: {error}"))?;
        Ok(record)
    }

    pub fn accept(&self, id: &str) -> Result<ChangeRecord, String> {
        let record = self.database.change(id)?;
        if record.status != "preview_ready" {
            return Err("Only a built and preview-ready change can be kept.".to_string());
        }
        let worktree = self.checked_worktree(&record)?;
        self.ensure_clean_main()?;
        let current_main = git_output(&self.paths.canonical_repo, &["rev-parse", "HEAD"])?;
        if current_main != record.base_commit {
            self.database.update_change(id, "conflict", None)?;
            return Err(
                "The main project changed after this temporary version began. Papers preserved both versions and requires a fresh preview against the new main."
                    .to_string(),
            );
        }

        git_status(&worktree, &["add", "-A"])?;
        let has_changes = Command::new("git")
            .args(["diff", "--cached", "--quiet"])
            .current_dir(&worktree)
            .status()
            .map_err(|error| error.to_string())?
            .code()
            == Some(1);
        if !has_changes {
            return Err("The temporary version contains no source changes to keep.".to_string());
        }
        let commit_message = format!("Let Papers improve itself: {}", clean_title(&record.title));
        git_status(&worktree, &["commit", "-m", &commit_message])?;
        let commit = git_output(&worktree, &["rev-parse", "HEAD"])?;
        git_status(
            &self.paths.canonical_repo,
            &["merge", "--ff-only", &record.branch],
        )?;

        let built = candidate_executable(&worktree);
        let version_dir = self.paths.versions.join(&commit);
        std::fs::create_dir_all(&version_dir).map_err(|error| error.to_string())?;
        let installed = version_dir.join(if cfg!(windows) {
            "papers.exe"
        } else {
            "papers"
        });
        std::fs::copy(&built, &installed)
            .map_err(|error| format!("Could not preserve the accepted executable: {error}"))?;
        self.health_check_candidate(&record.id, &installed)?;
        self.activate_version(&commit, &installed)?;

        let push = git_status(&self.paths.canonical_repo, &["push", "origin", "main"]);
        if let Err(error) = push {
            self.database.enqueue_sync(&commit, &error)?;
        }
        let updated = self.database.update_change(id, "accepted", Some(&commit))?;

        let _ = git_status(
            &self.paths.canonical_repo,
            &[
                "worktree",
                "remove",
                "--force",
                worktree.to_string_lossy().as_ref(),
            ],
        );
        let _ = git_status(
            &self.paths.canonical_repo,
            &["branch", "-d", &record.branch],
        );
        Ok(updated)
    }

    pub fn reject(&self, id: &str) -> Result<(), String> {
        let record = self.database.change(id)?;
        if record.status == "accepted" {
            return Err("An accepted change must be rolled back, not rejected.".to_string());
        }
        let worktree = PathBuf::from(&record.worktree_path);
        if worktree.exists() {
            self.checked_worktree(&record)?;
            git_status(
                &self.paths.canonical_repo,
                &[
                    "worktree",
                    "remove",
                    "--force",
                    worktree.to_string_lossy().as_ref(),
                ],
            )?;
        }
        let _ = git_status(
            &self.paths.canonical_repo,
            &["branch", "-D", &record.branch],
        );
        self.database.update_change(id, "rejected", None)?;
        Ok(())
    }

    pub fn rollback_last(&self) -> Result<String, String> {
        let mut registry = self.load_registry()?;
        let current_id = registry
            .active
            .clone()
            .ok_or_else(|| "No AI-installed Papers version is active yet.".to_string())?;
        let previous_id = registry
            .previous
            .clone()
            .ok_or_else(|| "No previous working version has been recorded yet.".to_string())?;
        let current = registry
            .versions
            .iter()
            .find(|version| version.id == current_id)
            .cloned()
            .ok_or_else(|| "The active version record is incomplete.".to_string())?;
        let previous = registry
            .versions
            .iter()
            .find(|version| version.id == previous_id)
            .ok_or_else(|| "The previous executable is no longer available.".to_string())?;
        if !Path::new(&previous.executable).is_file() {
            return Err("The previous executable is no longer available.".to_string());
        }

        registry.active = Some(previous_id.clone());
        registry.previous = Some(current_id);
        self.save_registry(&registry)?;

        self.ensure_clean_main()?;
        git_status(
            &self.paths.canonical_repo,
            &["revert", "--no-edit", &current.commit],
        )?;
        let revert_commit = git_output(&self.paths.canonical_repo, &["rev-parse", "HEAD"])?;
        if let Err(error) = git_status(&self.paths.canonical_repo, &["push", "origin", "main"]) {
            self.database.enqueue_sync(&revert_commit, &error)?;
            return Ok(
                "Returned the launcher to the previous working version. The source rollback is saved locally and GitHub sync is pending."
                    .to_string(),
            );
        }
        Ok(
            "Returned the launcher and source to the previous working version. Restart Papers to use it."
                .to_string(),
        )
    }

    fn ensure_repository(&self) -> Result<(), String> {
        if !self.paths.canonical_repo.join(".git").exists() {
            return Err(format!(
                "The canonical Papers repository was not found at {}.",
                self.paths.canonical_repo.display()
            ));
        }
        Ok(())
    }

    fn ensure_clean_main(&self) -> Result<(), String> {
        let status = git_output(&self.paths.canonical_repo, &["status", "--porcelain"])?;
        if !status.trim().is_empty() {
            return Err(
                "The canonical Papers repository contains uncommitted work. Papers will not build over changes it cannot safely identify."
                    .to_string(),
            );
        }
        Ok(())
    }

    fn checked_worktree(&self, record: &ChangeRecord) -> Result<PathBuf, String> {
        let path = PathBuf::from(&record.worktree_path);
        let staging = self
            .paths
            .staging
            .canonicalize()
            .map_err(|error| format!("Could not verify the staging root: {error}"))?;
        let checked = path
            .canonicalize()
            .map_err(|error| format!("Could not verify the temporary version: {error}"))?;
        if !checked.starts_with(&staging) {
            return Err("The temporary version escaped the protected staging root.".to_string());
        }
        Ok(checked)
    }

    fn health_check_candidate(&self, id: &str, executable: &Path) -> Result<(), String> {
        let health = self.paths.data.join(format!("health-{id}.ready"));
        let _ = std::fs::remove_file(&health);
        let mut child = Command::new(executable)
            .env("PAPERS_VERSION_ID", id)
            .env("PAPERS_HEALTH_FILE", &health)
            .spawn()
            .map_err(|error| format!("Could not start the accepted version: {error}"))?;
        let deadline = Instant::now() + Duration::from_secs(15);
        while Instant::now() < deadline {
            if health.is_file() {
                return Ok(());
            }
            if let Ok(Some(status)) = child.try_wait() {
                return Err(format!(
                    "The accepted version exited before its health check ({status}). The current installed version remains active."
                ));
            }
            thread::sleep(Duration::from_millis(150));
        }
        let _ = child.kill();
        Err(
            "The accepted version did not report healthy within 15 seconds. The current installed version remains active."
                .to_string(),
        )
    }

    fn activate_version(&self, commit: &str, executable: &Path) -> Result<(), String> {
        let mut registry = self.load_registry()?;
        let id = commit.to_string();
        if !registry.versions.iter().any(|version| version.id == id) {
            registry.versions.push(VersionEntry {
                id: id.clone(),
                commit: commit.to_string(),
                executable: executable.to_string_lossy().into_owned(),
                installed_at: Utc::now().to_rfc3339(),
                healthy: true,
            });
        }
        registry.previous = registry.active.take();
        registry.active = Some(id);
        self.save_registry(&registry)
    }

    fn load_registry(&self) -> Result<VersionRegistry, String> {
        let path = self.paths.version_registry();
        if !path.exists() {
            return Ok(VersionRegistry::default());
        }
        serde_json::from_slice(
            &std::fs::read(&path)
                .map_err(|error| format!("Could not read recovery state: {error}"))?,
        )
        .map_err(|error| format!("Recovery state is invalid: {error}"))
    }

    fn save_registry(&self, registry: &VersionRegistry) -> Result<(), String> {
        let path = self.paths.version_registry();
        let temporary = path.with_extension("json.tmp");
        std::fs::write(
            &temporary,
            serde_json::to_vec_pretty(registry).map_err(|error| error.to_string())?,
        )
        .map_err(|error| format!("Could not save recovery state: {error}"))?;
        std::fs::rename(&temporary, &path)
            .map_err(|error| format!("Could not activate recovery state: {error}"))
    }
}

fn candidate_executable(worktree: &Path) -> PathBuf {
    worktree
        .join("src-tauri")
        .join("target")
        .join("debug")
        .join(if cfg!(windows) {
            "papers.exe"
        } else {
            "papers"
        })
}

fn clean_title(title: &str) -> String {
    let cleaned = title
        .lines()
        .next()
        .unwrap_or("Change Papers")
        .trim()
        .chars()
        .take(80)
        .collect::<String>();
    if cleaned.is_empty() {
        "Change Papers".to_string()
    } else {
        cleaned
    }
}

fn git_output(cwd: &Path, args: &[&str]) -> Result<String, String> {
    let output = Command::new("git")
        .args(args)
        .current_dir(cwd)
        .output()
        .map_err(|error| format!("Could not run Git: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "Git {} failed: {}",
            args.join(" "),
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn git_status(cwd: &Path, args: &[&str]) -> Result<(), String> {
    let output = Command::new("git")
        .args(args)
        .current_dir(cwd)
        .output()
        .map_err(|error| format!("Could not run Git: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "Git {} failed: {}",
            args.join(" "),
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    Ok(())
}

fn run_logged(program: &str, args: &[&str], cwd: &Path, log_path: &Path) -> Result<(), String> {
    let mut log = OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_path)
        .map_err(|error| format!("Could not open build log: {error}"))?;
    let _ = writeln!(
        log,
        "\n--- {}: {} {} ---",
        Utc::now().to_rfc3339(),
        program,
        args.join(" ")
    );
    let stderr: File = log.try_clone().map_err(|error| error.to_string())?;
    let status = Command::new(program)
        .args(args)
        .current_dir(cwd)
        .stdout(Stdio::from(log))
        .stderr(Stdio::from(stderr))
        .status()
        .map_err(|error| format!("Could not run {program}: {error}"))?;
    if !status.success() {
        return Err(format!(
            "{program} {} stopped with {status}.",
            args.join(" ")
        ));
    }
    Ok(())
}
