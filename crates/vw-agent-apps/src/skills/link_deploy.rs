//! Host-local link deployment shares copy publication, conflict checks and rollback.
use serde::{Deserialize, Serialize};
use std::path::Path;
use super::{app_paths::AppSkillsPath, copy_deploy::{CopyBinding, CopyDeploy}, local_install::LocalInstallError};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SyncMethod { Auto, Symlink, Copy }

/// Inspect `binding().link_target` to persist the actual mode selected by Auto.
/// Commit settings after publish, then finish; Drop rolls back failed commits.
pub fn prepare_deploy(ssot: &Path, target: &AppSkillsPath, directory: &str,
    saved: Option<&CopyBinding>, method: SyncMethod) -> Result<CopyDeploy, LocalInstallError> {
    CopyDeploy::prepare_method(ssot, target, directory, saved, method)
}
