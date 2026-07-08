use std::path::PathBuf;

/// Hardcoded fallback for the creator's dev machine. Override with
/// $PAPERS_REPO_PATH on any other machine. Used only for Git operations in
/// self_edit (rev-parse, push, worktree, rollback). NEVER use this as a
/// runtime process working directory — use `runtime` for that.
pub const DEFAULT_REPOSITORY: &str =
    r"C:\This is Minh\LapSlop brotherhood\Programs\Papers are papers\REAL";

#[derive(Debug, Clone)]
pub struct PapersPaths {
    pub root: PathBuf,
    pub data: PathBuf,
    pub hermes_home: PathBuf,
    pub hermes_install: PathBuf,
    pub runtime: PathBuf,
    pub staging: PathBuf,
    pub versions: PathBuf,
    pub logs: PathBuf,
    /// Git repository root used only by the self-edit/builder flow. Set via
    /// $PAPERS_REPO_PATH at runtime, or falls back to DEFAULT_REPOSITORY.
    /// NEVER use this as a process working directory — the installed Hermes
    /// runtime launches from `runtime`, not from here.
    pub canonical_repo: PathBuf,
}

impl PapersPaths {
    pub fn discover() -> Result<Self, String> {
        let root = std::env::var_os("PAPERS_DATA_HOME")
            .map(PathBuf::from)
            .or_else(|| dirs::data_local_dir().map(|path| path.join("Papers")))
            .ok_or_else(|| {
                "Windows did not provide a local application-data directory".to_string()
            })?;
        let canonical_repo = std::env::var_os("PAPERS_REPO_PATH")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from(DEFAULT_REPOSITORY));

        let paths = Self {
            data: root.join("data"),
            hermes_home: root.join("data").join("hermes"),
            hermes_install: root.join("runtime").join("hermes-agent"),
            runtime: root.join("runtime"),
            staging: root.join("staging"),
            versions: root.join("versions"),
            logs: root.join("logs"),
            canonical_repo,
            root,
        };
        paths.ensure()?;
        Ok(paths)
    }

    fn ensure(&self) -> Result<(), String> {
        for path in [
            &self.root,
            &self.data,
            &self.hermes_home,
            &self.runtime,
            &self.staging,
            &self.versions,
            &self.logs,
        ] {
            std::fs::create_dir_all(path)
                .map_err(|error| format!("Could not create {}: {error}", path.display()))?;
        }
        Ok(())
    }

    pub fn hermes_executable(&self) -> PathBuf {
        self.hermes_install
            .join("venv")
            .join("Scripts")
            .join("hermes.exe")
    }

    pub fn database(&self) -> PathBuf {
        self.data.join("papers.db")
    }

    pub fn version_registry(&self) -> PathBuf {
        self.data.join("versions.json")
    }
}
