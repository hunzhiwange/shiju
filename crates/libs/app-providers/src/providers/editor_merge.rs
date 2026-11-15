//! Native editor file access and preview dispatch.
use serde_json::Value;
use vw_api_app_provider_edit::editor_merge::*;
pub use app_provider_core::editor_merge::{MergeError, merge};
#[cfg(target_os = "macos")]
pub struct ClaudeFile {
    guard: super::live::write_guard::AppWriteGuard,
    name: &'static str,
    pub bytes: Vec<u8>,
    pub identity: (u64, u64),
}
#[cfg(target_os = "macos")]
impl ClaudeFile {
    pub fn open(home: &std::path::Path) -> Result<Self, super::live::write_guard::WriteError> {
        Self::open_selected(home, None)
    }

    pub fn open_selected(
        home: &std::path::Path,
        selected: Option<&super::live::claude_directory::ClaudeDirectory>,
    ) -> Result<Self, super::live::write_guard::WriteError> {
        use super::live::write_guard::WriteError;
        use std::os::{
            fd::{AsRawFd, FromRawFd},
            unix::fs::{MetadataExt, OpenOptionsExt},
        };
        let directory = if let Some(selected) = selected {
            selected.open(home)?
        } else {
            let home = std::fs::OpenOptions::new()
                .read(true)
                .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC)
                .open(home)
                .map_err(|_| WriteError::InvalidResource)?;
            // SAFETY: fixed child and retained trusted HOME descriptor; fresh descriptor consumed once.
            let fd = unsafe {
                libc::openat(
                    home.as_raw_fd(),
                    c".claude".as_ptr(),
                    libc::O_RDONLY | libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC,
                )
            };
            if fd < 0 {
                return Err(WriteError::InvalidResource);
            }
            unsafe { std::fs::File::from_raw_fd(fd) }
        };
        let meta = directory.metadata().map_err(|_| WriteError::Io)?;
        let guard = super::live::write_guard::lock_directory(
            vw_api_app_identity::AppId::Claude,
            directory,
        )?;
        let (name, bytes) = match guard.read(std::ffi::OsStr::new("settings.json"))? {
            Some(bytes) => ("settings.json", bytes),
            None if selected.is_some() => (
                "claude.json",
                guard
                    .read(std::ffi::OsStr::new("claude.json"))?
                    .ok_or(WriteError::InvalidResource)?,
            ),
            None => return Err(WriteError::InvalidResource),
        };
        Ok(Self { guard, name, bytes, identity: (meta.dev(), meta.ino()) })
    }
    pub fn commit(
        &mut self,
        transactions: &super::live::commit::ProviderTransactions,
        tenant: &vw_api_transport::TenantScope,
        state: &super::live::commit::AppState,
        base: &Value,
        edits: &[EditorEdit],
        choice: EditorChoice,
    ) -> Result<(), super::live::write_guard::WriteError> {
        use super::live::{
            stage::FileChange,
            write_guard::{FilePrivacy, WriteError},
        };
        let expected = &self.bytes;
        let patch = |pre: Option<&[u8]>| {
            if transactions.state(vw_api_app_identity::AppId::Claude)? != *state
                || pre != Some(expected.as_slice())
            {
                return Err(WriteError::Conflict);
            }
            merge(base, pre.ok_or(WriteError::InvalidResource)?, edits, choice, true)
                .map_err(|_| WriteError::Conflict)
        };
        transactions.commit(
            tenant,
            &mut self.guard,
            &[FileChange {
                name: std::ffi::OsStr::new(self.name),
                privacy: FilePrivacy::Private,
                patch: &patch,
            }],
            Default::default(),
        )
    }
}
/// Same projection as preview; unsupported ownership shapes remain explicit failures.
pub fn project(live: &[u8], row: &Value) -> Result<Value, MergeError> {
    let live = serde_json::from_slice(live).map_err(|_| MergeError::Invalid)?;
    super::projection::claude(live, row, Some(row), None)
        .map(|p| p.settings)
        .map_err(|_| MergeError::Invalid)
}
pub fn display(value: Value) -> Value {
    super::preview::redact(value)
}
