//! MiniMax Code prompt publication on an already authorized execution host.
#[cfg(target_os = "macos")]
use std::ffi::OsStr;

#[cfg(target_os = "macos")]
use crate::providers::live::{
    commit::{PendingTarget, ProviderTransactions},
    stage::{FileChange, WholeFile},
    write_guard::{AppWriteGuard, FilePrivacy, WriteError},
};
#[cfg(target_os = "macos")]
use vw_api_types::{TenantScope, agent_apps::AppId};
#[cfg(not(target_os = "macos"))]
#[derive(Debug, PartialEq, Eq)]
pub enum McodePromptError {
    ContentTooLarge,
}

pub const MAX_PROMPT_BYTES: usize = 32 * 1024;

#[cfg(target_os = "macos")]
#[derive(Debug, PartialEq, Eq)]
pub enum McodePromptError {
    ContentTooLarge,
    File(WriteError),
    Store(WriteError),
    Rollback { store: WriteError, file: WriteError },
}

#[cfg(target_os = "macos")]
impl From<WriteError> for McodePromptError {
    fn from(error: WriteError) -> Self {
        Self::File(error)
    }
}

/// Validate every saved entry, including disabled copies, before persistence.
/// Rust string lengths count UTF-8 bytes; content is never truncated.
pub fn validate_content(content: &str) -> Result<(), McodePromptError> {
    if content.len() > MAX_PROMPT_BYTES {
        return Err(McodePromptError::ContentTooLarge);
    }
    Ok(())
}

/// Publish an active edit, activation, or empty-content deactivation, then save
/// the prompt catalog. The caller retains the same guard across catalog reads,
/// backup preparation and this call, serializing all writes to this app/target.
///
/// `expected` is the observed AGENTS.md snapshot, with missing distinct from
/// empty. Preflight must recheck authorization, ownership and catalog revisions.
/// The store callback must atomically persist all entry/enable/backup changes
/// and leave its state unchanged on error. Disabled-only edits use
/// `validate_content` and the store directly, without publishing a native file.
/// A process crash between file publication and catalog commit is outside this
/// synchronous rollback contract; the file transaction retains its own recovery.
#[cfg(target_os = "macos")]
#[allow(clippy::too_many_arguments)]
pub fn write_and_commit<T>(
    guard: &mut AppWriteGuard,
    transactions: &ProviderTransactions,
    tenant: &TenantScope,
    expected: Option<&[u8]>,
    content: &str,
    preflight: impl Fn() -> Result<(), WriteError>,
    commit_store: impl FnOnce() -> Result<T, WriteError>,
) -> Result<T, McodePromptError> {
    validate_content(content)?;
    if guard.app() != AppId::Mcode {
        return Err(WriteError::InvalidResource.into());
    }
    let after = content.as_bytes().to_vec();
    publish(guard, transactions, tenant, expected, Some(after.clone()), preflight)?;
    match commit_store() {
        Ok(value) => Ok(value),
        Err(store) => {
            if expected != Some(after.as_slice()) {
                // Restore absence as absence. Compare our published bytes so a
                // noncooperating writer's subsequent edit is never overwritten.
                publish(
                    guard,
                    transactions,
                    tenant,
                    Some(&after),
                    expected.map(<[u8]>::to_vec),
                    || Ok(()),
                )
                .map_err(|file| McodePromptError::Rollback { store, file })?;
            }
            Err(McodePromptError::Store(store))
        }
    }
}

#[cfg(target_os = "macos")]
fn publish(
    guard: &mut AppWriteGuard,
    transactions: &ProviderTransactions,
    tenant: &TenantScope,
    expected: Option<&[u8]>,
    after: Option<Vec<u8>>,
    preflight: impl Fn() -> Result<(), WriteError>,
) -> Result<(), WriteError> {
    let patch = WholeFile::new(expected, after);
    transactions.commit_checked(
        tenant,
        guard,
        &[FileChange {
            name: OsStr::new("AGENTS.md"),
            privacy: FilePrivacy::Private,
            patch: &patch,
        }],
        PendingTarget::default(),
        preflight,
    )
}
