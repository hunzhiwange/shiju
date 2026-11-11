//! Host-local configuration for the OpenCode OMO Slim variant.
use super::omo_read::{OmoReadError, read_candidate, remove_credentials, take_object};
use crate::providers::{
    live::{
        commit::ProviderTransactions,
        patch_json::{object_value, replace_object_preserving_trivia},
        stage::{FileChange, WholeFile},
        write_guard::{FilePrivacy, WriteError, lock_directory},
    },
    opencode::open_config_directory,
    paths::{bounded, open_beneath},
    validate_identity,
};
use serde_json::{Map, Value};
use std::{
    ffi::OsStr,
    fs::File,
    os::unix::fs::MetadataExt,
    path::{Path, PathBuf},
};
use vw_api_types::{agent_apps::AppId, app_providers::paths::ConfigPathState, id::TenantScope};

const NAMES: [&str; 2] = ["oh-my-opencode-slim.jsonc", "oh-my-opencode-slim.json"];

#[derive(Debug, PartialEq, Eq)]
pub struct OmoSlimRead {
    pub current_provider_id: Option<String>,
    pub agents: Option<Map<String, Value>>,
    pub other_fields: Option<Map<String, Value>>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OmoSlimError {
    UnsupportedTarget,
    Forbidden,
    InvalidConfiguration,
    RevisionConflict,
    VariantConflict,
    RecoveryPending,
    TargetFailed,
}

impl From<OmoReadError> for OmoSlimError {
    fn from(error: OmoReadError) -> Self {
        match error {
            OmoReadError::UnsupportedTarget => Self::UnsupportedTarget,
            OmoReadError::Forbidden => Self::Forbidden,
            OmoReadError::InvalidConfiguration => Self::InvalidConfiguration,
            OmoReadError::TargetFailed => Self::TargetFailed,
        }
    }
}

impl From<WriteError> for OmoSlimError {
    fn from(error: WriteError) -> Self {
        match error {
            WriteError::Conflict => Self::RevisionConflict,
            WriteError::Recovered => Self::RecoveryPending,
            WriteError::InvalidResource | WriteError::Io => Self::TargetFailed,
        }
    }
}

#[derive(PartialEq, Eq, serde::Serialize)]
pub struct OmoSlimSnapshot {
    directory: PathBuf,
    identity: Option<(u64, u64)>,
    files: [Option<Vec<u8>>; 2],
}

impl OmoSlimSnapshot {
    /// Existing files are configuration even when they contain an empty object.
    pub fn configuration_present(&self) -> bool {
        self.files.iter().any(Option::is_some)
    }

    /// The authorized caller supplies the saved Slim provider from the same scope.
    pub fn read(&self, current_provider_id: Option<&str>) -> Result<OmoSlimRead, OmoSlimError> {
        if let Some(id) = current_provider_id {
            validate_identity(id).map_err(|_| OmoSlimError::InvalidConfiguration)?;
        }
        let mut config = self
            .files
            .iter()
            .flatten()
            .next()
            .map(|bytes| parse(bytes))
            .transpose()?
            .unwrap_or_default();
        config.remove("categories");
        let agents = take_object(&mut config, "agents")?;
        remove_credentials(&mut config);
        Ok(OmoSlimRead {
            current_provider_id: current_provider_id.map(str::to_owned),
            agents,
            other_fields: (!config.is_empty()).then_some(config),
        })
    }
}

pub enum OmoSlimChange<'a> {
    /// These fields are projected from a saved Slim provider. A saved current
    /// standard OMO provider blocks the apply until it has been disabled.
    Apply {
        fields: &'a Map<String, Value>,
        standard_provider_id: Option<&'a str>,
    },
    Disable,
}

/// Capture both candidates so a newly appearing fallback cannot survive disable.
pub fn snapshot(
    home: &Path,
    directory_override: Option<&str>,
) -> Result<OmoSlimSnapshot, OmoSlimError> {
    if !home.is_absolute() {
        return Err(OmoSlimError::Forbidden);
    }
    let directory = directory_override
        .map(|raw| bounded(home, raw).map_err(|_| OmoSlimError::Forbidden))
        .transpose()?
        .unwrap_or_else(|| home.join(".config/opencode"));
    snapshot_at(home, &directory)
}

fn snapshot_at(home: &Path, directory: &Path) -> Result<OmoSlimSnapshot, OmoSlimError> {
    let identity = open_directory(home, directory)?.as_ref().map(identity).transpose()?;
    let files = if identity.is_some() {
        [
            read_candidate(home, &directory.join(NAMES[0]))?,
            read_candidate(home, &directory.join(NAMES[1]))?,
        ]
    } else {
        [None, None]
    };
    Ok(OmoSlimSnapshot { directory: directory.to_owned(), identity, files })
}

fn open_directory(home: &Path, path: &Path) -> Result<Option<File>, OmoSlimError> {
    match open_beneath(home, path, true) {
        Ok(directory) => Ok(Some(directory)),
        Err(ConfigPathState::Absent) => Ok(None),
        Err(_) => Err(OmoSlimError::TargetFailed),
    }
}

fn identity(directory: &File) -> Result<(u64, u64), OmoSlimError> {
    let metadata = directory.metadata().map_err(|_| OmoSlimError::TargetFailed)?;
    Ok((metadata.dev(), metadata.ino()))
}

/// Run on the authorized execution host. The caller rechecks the saved
/// directory override, variant selections, and provider revision before publish.
pub fn write(
    home: &Path,
    transactions: &ProviderTransactions,
    tenant: &TenantScope,
    expected: &OmoSlimSnapshot,
    change: OmoSlimChange<'_>,
    unchanged: impl Fn() -> bool,
) -> Result<bool, OmoSlimError> {
    write_impl(home, transactions, tenant, expected, change, unchanged, None)
}

#[allow(clippy::too_many_arguments)]
pub fn write_project(
    home: &Path,
    transactions: &ProviderTransactions,
    tenant: &TenantScope,
    expected: &OmoSlimSnapshot,
    change: OmoSlimChange<'_>,
    unchanged: impl Fn() -> bool,
    project: &super::omo_scope::ProjectOmoEdit,
) -> Result<bool, OmoSlimError> {
    write_impl(home, transactions, tenant, expected, change, unchanged, Some(project))
}

#[allow(clippy::too_many_arguments)]
fn write_impl(
    home: &Path,
    transactions: &ProviderTransactions,
    tenant: &TenantScope,
    expected: &OmoSlimSnapshot,
    change: OmoSlimChange<'_>,
    unchanged: impl Fn() -> bool,
    project: Option<&super::omo_scope::ProjectOmoEdit>,
) -> Result<bool, OmoSlimError> {
    if !home.is_absolute() || !expected.directory.starts_with(home) {
        return Err(OmoSlimError::Forbidden);
    }
    if snapshot_at(home, &expected.directory)? != *expected || !unchanged() {
        return Err(OmoSlimError::RevisionConflict);
    }
    let after = match change {
        OmoSlimChange::Apply { fields, standard_provider_id } => {
            if standard_provider_id.is_some() {
                return Err(OmoSlimError::VariantConflict);
            }
            let selected = usize::from(expected.files[0].is_none() && expected.files[1].is_some());
            let before = expected.files[selected].as_deref().unwrap_or(b"{}\n");
            let mut desired = parse(before)?;
            validate_agents(&desired)?;
            validate_agents(fields)?;
            desired.remove("agents");
            desired.extend(fields.clone());
            desired.remove("categories");
            let mut after = expected.files.clone();
            after[selected] = Some(
                replace_object_preserving_trivia(before, Value::Object(desired))
                    .map_err(|_| OmoSlimError::InvalidConfiguration)?,
            );
            after
        }
        OmoSlimChange::Disable => {
            for bytes in expected.files.iter().flatten() {
                validate_agents(&parse(bytes)?)?;
            }
            [None, None]
        }
    };
    let changed = after != expected.files;
    let directory = if expected.identity.is_none() {
        if !changed && project.is_none() {
            return Ok(false);
        }
        open_config_directory(home, &expected.directory)?
    } else {
        open_directory(home, &expected.directory)?.ok_or(OmoSlimError::RevisionConflict)?
    };
    let directory_identity = identity(&directory)?;
    let mut guard = lock_directory(AppId::OpenCode, directory)?;
    let current = snapshot_at(home, &expected.directory)?;
    if current.identity != Some(directory_identity)
        || expected.identity.is_some_and(|old| old != directory_identity)
        || current.files != expected.files
        || !unchanged()
    {
        return Err(OmoSlimError::RevisionConflict);
    }

    let pending = transactions.pending(AppId::OpenCode)?;
    if pending.as_ref().is_some_and(|pending| {
        (pending.directory_device, pending.directory_inode) != directory_identity
            || &pending.tenant != tenant
            || pending.files.iter().any(|file| {
                file.directory != 0 || !NAMES.iter().any(|name| name.as_bytes() == file.name)
            })
    }) {
        return Err(OmoSlimError::RecoveryPending);
    }
    if !changed && pending.is_none() && project.is_none() {
        return Ok(false);
    }

    let patches = after
        .into_iter()
        .enumerate()
        .map(|(index, after)| WholeFile::new(current.files[index].as_deref(), after))
        .collect::<Vec<_>>();
    let changes = NAMES
        .iter()
        .zip(&patches)
        .map(|(name, patch)| FileChange {
            name: OsStr::new(name),
            privacy: FilePrivacy::Shared,
            patch,
        })
        .collect::<Vec<_>>();
    super::omo_scope::commit(project, transactions, tenant, &mut guard, &changes, || {
        if unchanged()
            && snapshot_at(home, &current.directory).is_ok_and(|actual| actual == current)
        {
            Ok(())
        } else {
            Err(WriteError::Conflict)
        }
    })?;
    Ok(changed)
}

fn parse(bytes: &[u8]) -> Result<Map<String, Value>, OmoSlimError> {
    match object_value(bytes) {
        Ok(Value::Object(fields)) => Ok(fields),
        _ => Err(OmoSlimError::InvalidConfiguration),
    }
}

fn validate_agents(fields: &Map<String, Value>) -> Result<(), OmoSlimError> {
    if fields.get("agents").is_some_and(|value| !value.is_object() && !value.is_null()) {
        return Err(OmoSlimError::InvalidConfiguration);
    }
    Ok(())
}
