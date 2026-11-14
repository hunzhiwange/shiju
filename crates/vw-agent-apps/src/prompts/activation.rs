//! Ordinary prompt publication. Callers hold the fixed target's guard across
//! catalog reads and commit, and authorize the physical host/project beforehand.
use std::ffi::OsStr;
use vw_api_types::{agent_apps::{AppId, PromptResource}, id::ProjectScope};
use crate::providers::live::{
    commit::{PendingTarget, ProviderTransactions},
    stage::{FileChange, WholeFile},
    write_guard::{AppWriteGuard, FilePrivacy, WriteError},
};
use super::listing::PromptEntry;

#[derive(Debug)]
pub enum ActivationError {
    InvalidTarget,
    MissingEntry,
    InvalidContent,
    InvalidSelection,
    File(WriteError),
    Store,
    Rollback(WriteError),
}
impl std::fmt::Display for ActivationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::InvalidTarget => "unsupported ordinary prompt target",
            Self::MissingEntry => "prompt not found",
            Self::InvalidContent => "invalid prompt content",
            Self::InvalidSelection => "invalid stored prompt selection",
            Self::File(_) => "prompt file transaction failed",
            Self::Store => "prompt catalog commit failed; file restored",
            Self::Rollback(_) => "prompt catalog commit and file rollback failed; reload required",
        })
    }
}
impl std::error::Error for ActivationError {}

pub fn filename(app: AppId) -> Result<&'static OsStr, ActivationError> {
    PromptResource::Application { app_id: app }.validate_application_access()
        .map_err(|_| ActivationError::InvalidTarget)?;
    let name = match app {
        AppId::Pi | AppId::Mcode => return Err(ActivationError::InvalidTarget),
        AppId::Claude => "CLAUDE.md",
        AppId::Gemini => "GEMINI.md",
        AppId::Hermes => "SOUL.md",
        _ => "AGENTS.md",
    };
    Ok(OsStr::new(name))
}

/// Prepare all catalog changes without side effects. Capture the requested body
/// before backfilling, including reactivating the current selection.
pub fn prepare(
    entries: &[PromptEntry], active: Option<&str>, id: &str,
    edit: Option<&PromptEntry>, live: Option<&[u8]>, now: i64,
) -> Result<(Vec<PromptEntry>, String), ActivationError> {
    let target = entries.iter().find(|entry| entry.id == id)
        .ok_or(ActivationError::MissingEntry)?;
    let target = edit.unwrap_or(target);
    if target.id != id || target.name.trim().is_empty() {
        return Err(ActivationError::InvalidContent);
    }
    let content = target.content.clone();
    let mut next = entries.to_vec();
    let active_index = match active {
        Some(active) => Some(next.iter().position(|entry| entry.id == active)
            .ok_or(ActivationError::InvalidSelection)?),
        None => None,
    };
    let live = live.map(std::str::from_utf8).transpose()
        .map_err(|_| ActivationError::InvalidContent)?;
    if let Some(live) = live.filter(|text| !text.trim().is_empty()) {
        // Replacing the active entry itself must preserve its live preimage too.
        if let Some(index) = active_index.filter(|_| active != Some(id)) {
            next[index].content = live.to_owned();
        } else if !next.iter().any(|entry| entry.content == live && entry.id != id)
            && (live != content || active.is_none())
        {
            next.push(PromptEntry {
                id: uuid::Uuid::new_v4().to_string(), name: "原始提示词备份".into(),
                content: live.to_owned(), description: Some("激活前自动备份".into()),
                created_at: Some(now),
            });
        }
    }
    if let Some(edit) = edit {
        let entry = next.iter_mut().find(|entry| entry.id == id).unwrap();
        let created_at = entry.created_at;
        *entry = edit.clone();
        entry.created_at = created_at;
    }
    Ok((next, content))
}

/// Store callback atomically commits entries and the unique selected ID via CAS,
/// leaving storage unchanged on error. Synchronous rollback compares our bytes;
/// external edits are reported as conflicts, never overwritten. Crash recovery of
/// the file transaction does not provide atomicity with the separate catalog DB.
#[allow(clippy::too_many_arguments)]
pub fn publish<T>(
    guard: &mut AppWriteGuard, transactions: &ProviderTransactions,
    scope: &ProjectScope, expected: Option<&[u8]>, content: &str,
    validate: impl Fn() -> Result<(), WriteError>,
    commit: impl FnOnce() -> Result<T, ()>,
) -> Result<T, ActivationError> {
    let name = filename(guard.app())?;
    let state = transactions.state(guard.app()).map_err(ActivationError::File)?;
    state.require_project(scope).map_err(ActivationError::File)?;
    let write = |guard: &mut AppWriteGuard, before: Option<&[u8]>, after: Option<Vec<u8>>| {
        let patch = WholeFile::new(before, after);
        transactions.commit_project_checked(scope, guard, &state, false,
            &[FileChange { name, privacy: FilePrivacy::Private, patch: &patch }],
            PendingTarget::default(), &validate)
    };
    write(guard, expected, Some(content.as_bytes().to_vec())).map_err(ActivationError::File)?;
    match commit() {
        Ok(value) => Ok(value),
        Err(()) => {
            // Publication increments transaction state; rollback uses its new snapshot.
            let state = transactions.state(guard.app()).map_err(ActivationError::Rollback)?;
            let patch = WholeFile::new(Some(content.as_bytes()), expected.map(<[u8]>::to_vec));
            transactions.commit_project_checked(scope, guard, &state, false,
                &[FileChange { name, privacy: FilePrivacy::Private, patch: &patch }],
                PendingTarget::default(), || Ok(())).map_err(ActivationError::Rollback)?;
            Err(ActivationError::Store)
        }
    }
}
