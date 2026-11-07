use super::{
    commit::{ProviderTransactions, filename},
    write_guard::{WriteError, read_from},
};
use crate::providers::{claude_login::LoginHost, paths};
use serde::{Deserialize, Serialize};
use std::{
    ffi::OsStr,
    fs::File,
    os::unix::fs::MetadataExt,
    path::{Path, PathBuf},
};
use vw_api_types::{agent_apps::AppId, id::TenantScope};

const APP: AppId = AppId::Claude;
const RECORD: &str = "config-directory";

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClaudeDirectory {
    owner: TenantScope,
    path: PathBuf,
    device: u64,
    inode: u64,
}

impl ClaudeDirectory {
    fn validate(home: &Path, raw: &str, tenant: &TenantScope) -> Result<Option<Self>, WriteError> {
        let path: PathBuf = paths::bounded(home, raw)
            .map_err(|_| WriteError::InvalidResource)?
            .components()
            .collect();
        if path == home.join(".claude") {
            return Ok(None);
        }
        if path == home {
            return Err(WriteError::InvalidResource);
        }
        let directory =
            paths::open_beneath(home, &path, true).map_err(|_| WriteError::InvalidResource)?;
        let metadata = directory.metadata().map_err(|_| WriteError::Io)?;
        // SAFETY: geteuid reads the current process identity.
        if metadata.uid() != unsafe { libc::geteuid() } || metadata.mode() & 0o022 != 0 {
            return Err(WriteError::InvalidResource);
        }
        Ok(Some(Self {
            owner: tenant.clone(),
            path,
            device: metadata.dev(),
            inode: metadata.ino(),
        }))
    }

    pub fn validated_path(&self, home: &Path) -> Result<&str, WriteError> {
        self.open(home)?;
        self.path.to_str().ok_or(WriteError::InvalidResource)
    }

    pub(crate) fn open(&self, home: &Path) -> Result<File, WriteError> {
        let directory =
            paths::open_beneath(home, &self.path, true).map_err(|_| WriteError::InvalidResource)?;
        let metadata = directory.metadata().map_err(|_| WriteError::Io)?;
        // SAFETY: geteuid reads the current process identity.
        if (metadata.dev(), metadata.ino()) != (self.device, self.inode)
            || metadata.uid() != unsafe { libc::geteuid() }
            || metadata.mode() & 0o022 != 0
        {
            return Err(WriteError::InvalidResource);
        }
        Ok(directory)
    }
}

impl ProviderTransactions {
    /// Acquired before opening the physical config file. The transaction lock
    /// follows the physical lock and is never held while acquiring this one.
    pub fn lock_claude_directory(&self) -> Result<File, WriteError> {
        self.lock_named(APP, "config-directory-lock")
    }

    pub fn claude_directory(
        &self,
        tenant: &TenantScope,
    ) -> Result<Option<ClaudeDirectory>, WriteError> {
        let saved: Option<ClaudeDirectory> =
            read_from(&self.directory, OsStr::new(&filename(APP, RECORD)))?
                .map(|value| {
                    serde_json::from_slice(&value).map_err(|_| WriteError::InvalidResource)
                })
                .transpose()?
                .flatten();
        if saved.as_ref().is_some_and(|directory| directory.owner != *tenant) {
            return Err(WriteError::Conflict);
        }
        Ok(saved)
    }

    pub fn set_claude_directory(
        &self,
        home: &Path,
        tenant: &TenantScope,
        requested: Option<&str>,
    ) -> Result<Option<ClaudeDirectory>, WriteError> {
        let _directory_lock = self.lock_claude_directory()?;
        let old = self.claude_directory(tenant)?;
        let next = requested
            .map(|raw| ClaudeDirectory::validate(home, raw, tenant))
            .transpose()?
            .flatten();
        if old == next {
            return Ok(next);
        }
        let state = self.state(APP)?;
        if state.pointer.is_some() || state.mode.is_some() || self.pending(APP)?.is_some() {
            return Err(WriteError::Conflict);
        }
        match home.join(".claude").symlink_metadata() {
            Ok(_) if LoginHost::open(home)?.has_journal()? => return Err(WriteError::Conflict),
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(_) => return Err(WriteError::Io),
        }
        self.save(APP, RECORD, &next)?;
        Ok(next)
    }
}
