//! Host-local edits of standard OMO's section in an existing unified document.
use crate::providers::{
    live::{
        commit::ProviderTransactions,
        patch_json::{JsonPatch, object_value, replace_member_preserving_trivia},
        stage::{FileChange, WholeFile},
        write_guard::{FilePrivacy, WriteError, lock_directory},
    },
    paths::open_beneath,
};
use serde_json::{Map, Value};
use std::{ffi::OsStr, fs::File, io::Read, os::unix::fs::MetadataExt, path::Path};
use vw_api_types::{agent_apps::AppId, app_providers::paths::ConfigPathState, id::TenantScope};

const SECTION: &str = "[opencode]";
const DIRECTORY: &str = ".omo";
const NAMES: [&str; 2] = ["omo.jsonc", "omo.json"];
const MAX_BYTES: u64 = 1024 * 1024;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OmoWriteError {
    UnsupportedTarget,
    Forbidden,
    InvalidConfiguration,
    RevisionConflict,
    RecoveryPending,
    TargetFailed,
}

#[derive(PartialEq, Eq)]
pub struct OmoSnapshot {
    directory: (u64, u64),
    name: &'static str,
    bytes: Vec<u8>,
}

pub enum OmoChange<'a> {
    /// The authorized caller projects these fields from a saved provider.
    Apply(&'a Map<String, Value>),
    Disable,
}

impl From<WriteError> for OmoWriteError {
    fn from(error: WriteError) -> Self {
        match error {
            WriteError::Conflict => Self::RevisionConflict,
            WriteError::Recovered => Self::RecoveryPending,
            WriteError::InvalidResource | WriteError::Io => Self::TargetFailed,
        }
    }
}

/// Capture the exact selected file from the authorized host's home.
pub fn snapshot(home: &Path) -> Result<OmoSnapshot, OmoWriteError> {
    let directory = directory(home)?;
    let identity = directory_identity(&directory)?;
    for name in NAMES {
        let file = match open_beneath(home, &home.join(DIRECTORY).join(name), false) {
            Ok(file) => file,
            Err(ConfigPathState::Absent) => continue,
            Err(_) => return Err(OmoWriteError::TargetFailed),
        };
        let mut bytes = Vec::new();
        file.take(MAX_BYTES + 1)
            .read_to_end(&mut bytes)
            .map_err(|_| OmoWriteError::TargetFailed)?;
        if bytes.len() as u64 > MAX_BYTES {
            return Err(OmoWriteError::InvalidConfiguration);
        }
        return Ok(OmoSnapshot { directory: identity, name, bytes });
    }
    Err(OmoWriteError::UnsupportedTarget)
}

/// Run only on the authorized execution host. The caller checks scope and
/// supplies a recheck of the saved provider and other inputs to the revision.
pub fn write(
    home: &Path,
    transactions: &ProviderTransactions,
    tenant: &TenantScope,
    expected: &OmoSnapshot,
    change: OmoChange<'_>,
    unchanged: impl Fn() -> bool,
) -> Result<bool, OmoWriteError> {
    let check_target = |error| match error {
        OmoWriteError::UnsupportedTarget => OmoWriteError::RevisionConflict,
        other => other,
    };
    let directory = directory(home).map_err(check_target)?;
    if directory_identity(&directory)? != expected.directory {
        return Err(OmoWriteError::RevisionConflict);
    }
    let mut guard = lock_directory(AppId::OpenCode, directory)?;
    let current = snapshot(home).map_err(check_target)?;
    if &current != expected || !unchanged() {
        return Err(OmoWriteError::RevisionConflict);
    }

    // OpenCode also journals writes in its native config directory. That owner
    // must recover its own resource before a unified OMO edit can proceed.
    if transactions.pending(AppId::OpenCode)?.is_some_and(|pending| {
        (pending.directory_device, pending.directory_inode) != expected.directory
            || &pending.tenant != tenant
    }) {
        return Err(OmoWriteError::RecoveryPending);
    }

    let after = edit(&expected.bytes, change)?;
    let changed = after != expected.bytes;
    let patch = WholeFile::new(Some(&expected.bytes), Some(after));
    transactions.commit_checked(
        tenant,
        &mut guard,
        &[FileChange {
            name: OsStr::new(expected.name),
            privacy: FilePrivacy::Shared,
            patch: &patch,
        }],
        Default::default(),
        || {
            if unchanged() && snapshot(home).is_ok_and(|current| current == *expected) {
                Ok(())
            } else {
                Err(WriteError::Conflict)
            }
        },
    )?;
    Ok(changed)
}

fn directory(home: &Path) -> Result<File, OmoWriteError> {
    if !home.is_absolute() {
        return Err(OmoWriteError::Forbidden);
    }
    open_beneath(home, &home.join(DIRECTORY), true).map_err(|state| match state {
        ConfigPathState::Absent => OmoWriteError::UnsupportedTarget,
        _ => OmoWriteError::TargetFailed,
    })
}

fn directory_identity(directory: &File) -> Result<(u64, u64), OmoWriteError> {
    let metadata = directory.metadata().map_err(|_| OmoWriteError::TargetFailed)?;
    Ok((metadata.dev(), metadata.ino()))
}

fn edit(before: &[u8], change: OmoChange<'_>) -> Result<Vec<u8>, OmoWriteError> {
    let root = object_value(before).map_err(|_| OmoWriteError::InvalidConfiguration)?;
    if let Some(section) = root.get(SECTION) {
        validate_fields(section.as_object().ok_or(OmoWriteError::InvalidConfiguration)?)?;
    }
    let result = match change {
        OmoChange::Apply(fields) => {
            validate_fields(fields)?;
            replace_member_preserving_trivia(before, SECTION, Value::Object(fields.clone()))
        }
        OmoChange::Disable => {
            JsonPatch { remove: vec![vec![SECTION.to_owned()]], ..Default::default() }
                .apply(Some(before))
        }
    };
    result.map_err(|_| OmoWriteError::InvalidConfiguration)
}

fn validate_fields(fields: &Map<String, Value>) -> Result<(), OmoWriteError> {
    for key in ["agents", "categories"] {
        if fields.get(key).is_some_and(|value| !value.is_object() && !value.is_null()) {
            return Err(OmoWriteError::InvalidConfiguration);
        }
    }
    Ok(())
}
