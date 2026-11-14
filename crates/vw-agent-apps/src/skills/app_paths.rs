//! Read-only skill paths on the authorized execution host.
//!
//! HOME, overrides and environment must come from the selected host/installation,
//! never from a client's filesystem. These observations grant no write permission;
//! publishers must recheck path identity under their own transaction guard.
use crate::providers::paths;
use std::{
    collections::BTreeMap,
    path::{Component, Path, PathBuf},
};
use vw_api_types::{agent_apps::AppId, app_providers::paths::ConfigPathState};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkillDistribution {
    /// Claude Desktop has a historical path but no directory synchronization.
    NoDirectorySync,
    /// OpenClaw's parser exists, but no upstream skill install target is exposed.
    PathOnly,
    ManagedBinding,
    /// Pi and OMP derive enabled state from the actual native directory.
    NativeDirectory,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppSkillsPath {
    pub app_id: AppId,
    pub directory: PathBuf,
    pub state: ConfigPathState,
    pub distribution: SkillDistribution,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppPathError {
    UnsupportedPlatform,
    InvalidConfiguration(&'static str),
    UnsafeDirectory,
    OverlappingRoots,
    InvalidSkillDirectory,
    NoDirectorySync,
    NoInstallTarget,
    BindingStateRequired,
}

impl std::fmt::Display for AppPathError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::UnsupportedPlatform => "skill paths require a macOS execution host",
            Self::InvalidConfiguration(reason) => reason,
            Self::UnsafeDirectory => "skill directory is inaccessible or contains a link",
            Self::OverlappingRoots => "skill roots overlap or alias one another",
            Self::InvalidSkillDirectory => "invalid single-segment skill directory",
            Self::NoDirectorySync => "application does not support skill directory synchronization",
            Self::NoInstallTarget => "application skill path has no enabled install target",
            Self::BindingStateRequired => "application enabled state requires its saved binding",
        })
    }
}

impl std::error::Error for AppPathError {}

pub fn distribution(app: AppId) -> SkillDistribution {
    use AppId::*;
    match app {
        ClaudeDesktop => SkillDistribution::NoDirectorySync,
        OpenClaw => SkillDistribution::PathOnly,
        Pi | Omp => SkillDistribution::NativeDirectory,
        Claude | Codex | Gemini | GrokBuild | OpenCode | Hermes | Mcode | Kimi | Dsh | Agy => {
            SkillDistribution::ManagedBinding
        }
    }
}

impl AppSkillsPath {
    pub fn require_distribution(&self) -> Result<(), AppPathError> {
        if self.distribution != distribution(self.app_id) {
            return Err(AppPathError::InvalidConfiguration("inconsistent skill capability"));
        }
        match self.distribution {
            SkillDistribution::NoDirectorySync => Err(AppPathError::NoDirectorySync),
            SkillDistribution::PathOnly => Err(AppPathError::NoInstallTarget),
            SkillDistribution::ManagedBinding | SkillDistribution::NativeDirectory => Ok(()),
        }
    }
}

/// Resolve without creating directories. Pass SSOT, backup and staging roots (and
/// any other publication roots) as exclusions before using the result for writes.
pub fn resolve(
    home: &Path,
    app: AppId,
    override_dir: Option<&str>,
    environment: &BTreeMap<String, String>,
    excluded_roots: &[&Path],
) -> Result<AppSkillsPath, AppPathError> {
    if !cfg!(target_os = "macos") {
        return Err(AppPathError::UnsupportedPlatform);
    }
    // These two skill paths deliberately differ from provider configuration paths.
    let root = match app {
        AppId::ClaudeDesktop | AppId::Agy => {
            if override_dir.is_some() {
                return Err(AppPathError::InvalidConfiguration(
                    "unsupported skill directory override",
                ));
            }
            home.join(if app == AppId::Agy { ".gemini/config" } else { ".claude-desktop" })
        }
        _ => PathBuf::from(
            paths::read(home, app, override_dir, environment)
                .map_err(AppPathError::InvalidConfiguration)?
                .directory,
        ),
    };
    let directory = checked_path(home, &root.join("skills"))?;
    let state = directory_state(home, &directory)?;
    let identity = root_identity(&directory)?;
    for excluded in excluded_roots {
        let excluded = checked_path(home, excluded)?;
        directory_state(home, &excluded)?;
        let excluded = root_identity(&excluded)?;
        if identity.starts_with(&excluded) || excluded.starts_with(&identity) {
            return Err(AppPathError::OverlappingRoots);
        }
    }
    Ok(AppSkillsPath { app_id: app, directory, state, distribution: distribution(app) })
}

/// Inspect Pi/OMP's real directory, including an intentional skill link into SSOT.
/// A dangling link, foreign target or inaccessible entry is an error, not enabled.
/// This does not infer management ownership and never substitutes for saved bindings.
pub fn native_enabled(
    home: &Path,
    target: &AppSkillsPath,
    directory: &str,
) -> Result<bool, AppPathError> {
    if !cfg!(target_os = "macos") {
        return Err(AppPathError::UnsupportedPlatform);
    }
    target.require_distribution()?;
    if target.distribution != SkillDistribution::NativeDirectory {
        return Err(AppPathError::BindingStateRequired);
    }
    let mut parts = Path::new(directory).components();
    if directory.is_empty()
        || directory.len() > 255
        || directory.chars().any(char::is_control)
        || directory.contains(['/', '\\'])
        || !matches!(parts.next(), Some(Component::Normal(_)))
        || parts.next().is_some()
    {
        return Err(AppPathError::InvalidSkillDirectory);
    }
    let root = checked_path(home, &target.directory)?;
    if directory_state(home, &root)? == ConfigPathState::Absent {
        return Ok(false);
    }
    let child = root.join(directory);
    let metadata = match std::fs::symlink_metadata(&child) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(_) => return Err(AppPathError::UnsafeDirectory),
    };
    if !metadata.is_dir() && !metadata.file_type().is_symlink() {
        return Err(AppPathError::UnsafeDirectory);
    }
    let canonical = child.canonicalize().map_err(|_| AppPathError::UnsafeDirectory)?;
    let canonical_home = home.canonicalize().map_err(|_| AppPathError::UnsafeDirectory)?;
    let relative =
        canonical.strip_prefix(&canonical_home).map_err(|_| AppPathError::UnsafeDirectory)?;
    // Reopen beneath the trusted HOME, rejecting any further link traversal.
    match directory_state(home, &home.join(relative))? {
        ConfigPathState::Present => Ok(true),
        _ => Err(AppPathError::UnsafeDirectory),
    }
}

fn checked_path(home: &Path, path: &Path) -> Result<PathBuf, AppPathError> {
    paths::bounded(home, path.to_str().ok_or(AppPathError::UnsafeDirectory)?)
        .map_err(AppPathError::InvalidConfiguration)
}

fn directory_state(home: &Path, path: &Path) -> Result<ConfigPathState, AppPathError> {
    #[cfg(unix)]
    match paths::open_beneath(home, path, true) {
        Ok(_) => Ok(ConfigPathState::Present),
        Err(ConfigPathState::Absent) => Ok(ConfigPathState::Absent),
        Err(_) => Err(AppPathError::UnsafeDirectory),
    }
    #[cfg(not(unix))]
    {
        let _ = (home, path);
        Err(AppPathError::UnsupportedPlatform)
    }
}

// Resolve existing ancestors too: a missing leaf must not hide a case-insensitive
// filesystem alias. Link traversal has already been rejected by directory_state.
fn root_identity(path: &Path) -> Result<PathBuf, AppPathError> {
    let mut ancestor = path;
    let mut missing = Vec::new();
    loop {
        match ancestor.canonicalize() {
            Ok(mut identity) => {
                for component in missing.iter().rev() {
                    identity.push(component);
                }
                return Ok(identity);
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                missing.push(ancestor.file_name().ok_or(AppPathError::UnsafeDirectory)?);
                ancestor = ancestor.parent().ok_or(AppPathError::UnsafeDirectory)?;
            }
            Err(_) => return Err(AppPathError::UnsafeDirectory),
        }
    }
}
