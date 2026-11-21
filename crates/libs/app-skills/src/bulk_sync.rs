//! Ordered, non-atomic skill distribution on an authorized execution host.
use serde::{Deserialize, Serialize};
use vw_api_app_identity::{AppId};

use super::{app_paths::AppSkillsPath, local_install::LocalSkill};

/// Resolved by the execution host after installation and path authorization.
#[derive(Debug, Clone)]
pub struct SkillSyncTarget {
    pub installation_id: String,
    pub path: AppSkillsPath,
}

/// A preference identifies an installation, never a client-supplied path.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkillInstallTarget {
    pub installation_id: String,
    pub app_id: AppId,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkillTargetError {
    InvalidInstallation,
    UnsupportedTarget,
    DuplicateTarget,
}

impl std::fmt::Display for SkillTargetError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::InvalidInstallation => "invalid skill application installation",
            Self::UnsupportedTarget => "application has no skill install target",
            Self::DuplicateTarget => "duplicate skill install target",
        })
    }
}
impl std::error::Error for SkillTargetError {}

impl SkillInstallTarget {
    pub fn validate(&self) -> Result<(), SkillTargetError> {
        if self.installation_id.trim().is_empty() || self.installation_id.len() > 255
            || self.installation_id.chars().any(char::is_control) {
            return Err(SkillTargetError::InvalidInstallation);
        }
        use super::app_paths::{distribution, SkillDistribution};
        if !matches!(distribution(self.app_id),
            SkillDistribution::ManagedBinding | SkillDistribution::NativeDirectory) {
            return Err(SkillTargetError::UnsupportedTarget);
        }
        Ok(())
    }
}

impl SkillSyncTarget {
    pub fn identity(&self) -> SkillInstallTarget {
        SkillInstallTarget { installation_id: self.installation_id.clone(), app_id: self.path.app_id }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SkillSyncStatus {
    Enabled,
    Disabled,
    /// Publication or cleanup failed; inspect persisted state before retrying.
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkillSyncOutcome {
    pub target: SkillInstallTarget,
    pub directory: String,
    pub status: SkillSyncStatus,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct BulkSyncResult {
    /// Committed SSOT installations remain visible even if every app fails.
    pub installed: Vec<LocalSkill>,
    pub items: Vec<SkillSyncOutcome>,
}

/// Reject ambiguous preferences before saving, without silently dropping entries.
pub fn validate_targets(targets: &[SkillInstallTarget]) -> Result<(), SkillTargetError> {
    for (index, target) in targets.iter().enumerate() {
        target.validate()?;
        if targets[..index].contains(target) {
            return Err(SkillTargetError::DuplicateTarget);
        }
    }
    Ok(())
}

/// Create the resolved directory beneath the authorized home without following links.
pub fn prepare_default_directory(
    home: &std::path::Path,
    target: &AppSkillsPath,
) -> std::io::Result<()> {
    use std::{ffi::CString, io, os::{fd::AsRawFd, unix::ffi::OsStrExt}, path::Component};

    target.require_distribution().map_err(io::Error::other)?;
    let invalid = || io::Error::new(io::ErrorKind::InvalidInput, "unsafe skill directory");
    let relative = target.directory.strip_prefix(home).map_err(|_| invalid())?;
    let mut path = home.to_path_buf();
    let mut parent = app_config_paths::open_beneath(home, home, true).map_err(|_| invalid())?;
    for part in relative.components() {
        let Component::Normal(name) = part else { return Err(invalid()); };
        let entry = CString::new(name.as_bytes())?;
        if unsafe { libc::mkdirat(parent.as_raw_fd(), entry.as_ptr(), 0o700) } != 0 {
            let error = io::Error::last_os_error();
            if error.kind() != io::ErrorKind::AlreadyExists { return Err(error); }
        }
        path.push(name);
        parent = app_config_paths::open_beneath(home, &path, true).map_err(|_| invalid())?;
    }
    Ok(())
}
