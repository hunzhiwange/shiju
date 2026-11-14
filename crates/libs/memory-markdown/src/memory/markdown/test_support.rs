//! 为每个测试独占并清理派生的用户态数据目录。

pub(super) struct TempWorkspace {
    workspace: tempfile::TempDir,
    storage: std::path::PathBuf,
}

impl TempWorkspace {
    pub(super) fn new() -> std::io::Result<Self> {
        let workspace = tempfile::TempDir::new()?;
        let storage = super::paths::project_data_dir_best_effort(workspace.path());
        if storage.exists() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::AlreadyExists,
                "test fixture requires a fresh project data directory",
            ));
        }
        Ok(Self { workspace, storage })
    }

    pub(super) fn path(&self) -> &std::path::Path {
        self.workspace.path()
    }
}

impl Drop for TempWorkspace {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.storage);
    }
}
