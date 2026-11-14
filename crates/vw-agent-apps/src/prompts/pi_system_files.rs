//! Fixed Pi native files on an already authorized execution host.
//! Callers retain the shared Pi AppWriteGuard across read/prepare/commit and
//! supply authorization preflight. No request path or Shiju system prompt is used.
use std::ffi::OsStr;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use vw_api_types::{TenantScope, agent_apps::AppId};

use crate::providers::live::{
    commit::{PendingTarget, ProviderTransactions},
    stage::{FileChange, WholeFile},
    write_guard::{AppWriteGuard, FilePrivacy, WriteError},
};

pub const MAX_PROMPT_FILE_BYTES: usize = 1024 * 1024;
pub const MISSING_REVISION: &str = "missing";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PiPromptFileKind {
    SystemOverride,
    SystemAppend,
}

impl PiPromptFileKind {
    fn filename(self) -> &'static OsStr {
        OsStr::new(match self {
            Self::SystemOverride => "SYSTEM.md",
            Self::SystemAppend => "APPEND_SYSTEM.md",
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PiPromptFileSnapshot {
    pub exists: bool,
    pub revision: String,
    pub content: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PiPromptFileError {
    BlankContent,
    ContentTooLarge,
    InvalidUtf8,
    File(WriteError),
}

impl From<WriteError> for PiPromptFileError {
    fn from(error: WriteError) -> Self { Self::File(error) }
}

pub struct PiPromptFileService;

impl PiPromptFileService {
    /// The guard's directory must be Pi's authorized native agent directory,
    /// resolved on the execution host. Acquire it through the shared host guard.
    pub fn read(
        guard: &AppWriteGuard,
        kind: PiPromptFileKind,
    ) -> Result<PiPromptFileSnapshot, PiPromptFileError> {
        if guard.app() != AppId::Pi {
            return Err(WriteError::InvalidResource.into());
        }
        snapshot(guard.read(kind.filename())?.as_deref())
    }

    #[allow(clippy::too_many_arguments)]
    pub fn replace(
        guard: &mut AppWriteGuard,
        transactions: &ProviderTransactions,
        tenant: &TenantScope,
        kind: PiPromptFileKind,
        expected_revision: &str,
        content: &str,
        preflight: impl Fn() -> Result<(), WriteError>,
    ) -> Result<PiPromptFileSnapshot, PiPromptFileError> {
        if content.trim().is_empty() { return Err(PiPromptFileError::BlankContent); }
        if content.len() > MAX_PROMPT_FILE_BYTES {
            return Err(PiPromptFileError::ContentTooLarge);
        }
        let before = Self::read(guard, kind)?;
        ensure_revision(&before, expected_revision)?;
        publish(guard, transactions, tenant, kind, &before, Some(content.as_bytes().to_vec()), preflight)?;
        Self::read(guard, kind)
    }

    /// Missing + "missing" returns false; a stale revision always conflicts.
    pub fn delete(
        guard: &mut AppWriteGuard,
        transactions: &ProviderTransactions,
        tenant: &TenantScope,
        kind: PiPromptFileKind,
        expected_revision: &str,
        preflight: impl Fn() -> Result<(), WriteError>,
    ) -> Result<bool, PiPromptFileError> {
        let before = Self::read(guard, kind)?;
        ensure_revision(&before, expected_revision)?;
        publish(guard, transactions, tenant, kind, &before, None, preflight)?;
        Ok(before.exists)
    }
}

fn snapshot(bytes: Option<&[u8]>) -> Result<PiPromptFileSnapshot, PiPromptFileError> {
    let Some(bytes) = bytes else {
        return Ok(PiPromptFileSnapshot { exists: false, revision: MISSING_REVISION.into(), content: String::new() });
    };
    if bytes.len() > MAX_PROMPT_FILE_BYTES { return Err(PiPromptFileError::ContentTooLarge); }
    Ok(PiPromptFileSnapshot {
        exists: true,
        revision: format!("{:x}", Sha256::digest(bytes)),
        content: std::str::from_utf8(bytes).map_err(|_| PiPromptFileError::InvalidUtf8)?.into(),
    })
}

fn ensure_revision(before: &PiPromptFileSnapshot, expected: &str) -> Result<(), PiPromptFileError> {
    if before.revision != expected { return Err(WriteError::Conflict.into()); }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn publish(
    guard: &mut AppWriteGuard,
    transactions: &ProviderTransactions,
    tenant: &TenantScope,
    kind: PiPromptFileKind,
    before: &PiPromptFileSnapshot,
    after: Option<Vec<u8>>,
    preflight: impl Fn() -> Result<(), WriteError>,
) -> Result<(), PiPromptFileError> {
    let patch = WholeFile::new(before.exists.then_some(before.content.as_bytes()), after);
    let state = transactions.state(vw_api_types::agent_apps::AppId::Pi)?;
    transactions.commit_checked(
        tenant, guard,
        &[FileChange { name: kind.filename(), privacy: FilePrivacy::Private, patch: &patch }],
        PendingTarget { project_scope: state.project_scope, ..PendingTarget::default() }, preflight,
    )?;
    Ok(())
}
