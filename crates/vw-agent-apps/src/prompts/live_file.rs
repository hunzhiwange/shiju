//! Read-only snapshots of fixed application prompt files on an authorized host.
//! Tenant/action authorization precedes this call; all host paths and environment
//! values must come from authenticated dispatch, never request payloads.
use std::{collections::BTreeMap, io::Read, os::unix::fs::MetadataExt, path::Path};

use sha2::{Digest, Sha256};
use vw_api_types::{
    agent_apps::{AppId, PromptContentSnapshot, PromptResource, PromptRevision, PromptTargetPreview},
    app_providers::paths::ConfigPathState,
};

use crate::providers::paths;
use super::targets;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LiveFileError {
    UnauthorizedHost,
    UnsupportedTarget,
    UnsafePath,
    ChangedDuringRead,
    InvalidUtf8,
    Io,
}

impl std::fmt::Display for LiveFileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::UnauthorizedHost => "prompt executor is not the authorized host",
            Self::UnsupportedTarget => "unsupported prompt target",
            Self::UnsafePath => "unsafe prompt file path",
            Self::ChangedDuringRead => "prompt file changed during preview; retry",
            Self::InvalidUtf8 => "prompt file is not valid UTF-8",
            Self::Io => "prompt file IO failed",
        })
    }
}
impl std::error::Error for LiveFileError {}

/// No arbitrary filename is accepted. Display paths and revisions are observations,
/// not write authority. Mutations must resolve and compare again under their lock.
pub fn read(
    authorized_host: &str,
    executor_host: &str,
    home: &Path,
    app: AppId,
    override_dir: Option<&str>,
    environment: &BTreeMap<String, String>,
) -> Result<PromptTargetPreview, LiveFileError> {
    if authorized_host.is_empty() || authorized_host != executor_host {
        return Err(LiveFileError::UnauthorizedHost);
    }
    let resource = PromptResource::Application { app_id: app };
    resource.validate_application_access().map_err(|_| LiveFileError::UnsupportedTarget)?;
    let target = targets::resolve(home, app, override_dir, environment)
        .map_err(|_| LiveFileError::UnsafePath)?;
    let open = || paths::open_beneath(home, &target.file, false);
    let content = match open() {
        Err(ConfigPathState::Absent) => {
            if !matches!(open(), Err(ConfigPathState::Absent)) {
                return Err(LiveFileError::ChangedDuringRead);
            }
            None
        }
        Err(_) => return Err(LiveFileError::UnsafePath),
        Ok(mut file) => {
            let before = file.metadata().map_err(|_| LiveFileError::Io)?;
            // Hard links could expose another file in the same filesystem.
            if before.nlink() != 1 {
                return Err(LiveFileError::UnsafePath);
            }
            let mut bytes = Vec::new();
            file.read_to_end(&mut bytes).map_err(|_| LiveFileError::Io)?;
            let after = file.metadata().map_err(|_| LiveFileError::Io)?;
            let current = open().map_err(|_| LiveFileError::ChangedDuringRead)?;
            let current = current.metadata().map_err(|_| LiveFileError::Io)?;
            let identity = |m: &std::fs::Metadata| {
                (m.dev(), m.ino(), m.len(), m.mtime(), m.mtime_nsec(),
                 m.ctime(), m.ctime_nsec(), m.nlink())
            };
            if identity(&before) != identity(&after) || identity(&after) != identity(&current) {
                return Err(LiveFileError::ChangedDuringRead);
            }
            Some(String::from_utf8(bytes).map_err(|_| LiveFileError::InvalidUtf8)?)
        }
    };
    let mut hash = Sha256::new();
    hash.update(b"shiju-prompt-live-v1\0");
    match &content {
        None => hash.update(b"missing"),
        Some(content) => { hash.update(b"present\0"); hash.update(content.as_bytes()); }
    }
    let revision = PromptRevision(format!("sha256:{:x}", hash.finalize()));
    let relative = target.file.strip_prefix(home).map_err(|_| LiveFileError::UnsafePath)?;
    Ok(PromptTargetPreview {
        resource: resource.clone(),
        display_path: Some(format!("~/{}", relative.display())),
        // This read operation makes no claim about mutation permissions.
        writable: false,
        snapshot: PromptContentSnapshot { resource, content, revision },
    })
}
