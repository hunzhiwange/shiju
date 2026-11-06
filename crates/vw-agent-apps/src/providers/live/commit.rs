//! Execution-host state owner. Construct only from a trusted host's private data
//! root (the same source as CredentialFileStore), never a request path or HOME.
//! No application adapters, public file API, recovery or cross-file rollback.
use super::{
    stage::FileChange,
    write_guard::{AppWriteGuard, FilePrivacy, WriteError, prepare, read_from},
};
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use serde::{Deserialize, Serialize};
use std::{
    ffi::{CString, OsStr},
    fs::{File, OpenOptions},
    os::{
        fd::{AsRawFd, FromRawFd},
        unix::fs::{MetadataExt, OpenOptionsExt},
    },
    path::Path,
};
use vw_api_types::{agent_apps::AppId, id::TenantScope};

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExecutionHostId(String);
impl ExecutionHostId {
    pub fn new(id: &str) -> Result<Self, WriteError> {
        crate::providers::validate_identity(id).map_err(|_| WriteError::InvalidResource)?;
        Ok(Self(id.into()))
    }
}
#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Mode {
    Direct,
    Proxy,
}
#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct Contract {
    pub version: u32,
    pub key: String,
    pub exclusive: serde_json::Map<String, serde_json::Value>,
}
#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct ModeState {
    pub mode: Mode,
    pub attached: bool,
    pub proxy_route: Option<String>,
    pub contract: Option<Contract>,
}
/// None preserves that part of state; direct pointer and proxy route are independent.
#[derive(Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct PendingTarget {
    pub pointer: Option<String>,
    pub mode: Option<ModeState>,
}
#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct ScopedProvider {
    pub tenant: TenantScope,
    pub id: String,
}
#[derive(Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct AppState {
    pub pointer: Option<ScopedProvider>,
    pub mode: Option<ModeState>,
    pub mode_tenant: Option<TenantScope>,
}
#[derive(Clone, Serialize, Deserialize)]
pub struct PendingFile {
    pub name: Vec<u8>,
    pub staged: Vec<u8>,
    pub pre: Option<String>,
    pub planned: String,
}
#[derive(Clone, Serialize, Deserialize)]
pub struct Pending {
    pub version: u32,
    pub host: ExecutionHostId,
    pub app: AppId,
    pub tenant: TenantScope,
    pub directory_device: u64,
    pub directory_inode: u64,
    pub target: PendingTarget,
    pub files: Vec<PendingFile>,
    pub published: bool,
}

pub struct ProviderTransactions {
    directory: File,
    host: ExecutionHostId,
}
impl ProviderTransactions {
    /// Executor initialization factory. Caller supplies the authorized host's
    /// existing private root; no discovery, client path or tenant root fallback.
    pub fn new(private_root: &Path, host: ExecutionHostId) -> Result<Self, WriteError> {
        crate::providers::validate_identity(&host.0).map_err(|_| WriteError::InvalidResource)?;
        if !private_root.is_absolute() {
            return Err(WriteError::InvalidResource);
        }
        let root = OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC)
            .open(private_root)
            .map_err(|_| WriteError::InvalidResource)?;
        check_private(&root)?;
        let directory = private_child(&root, "provider-live")?;
        let directory = private_child(&directory, &URL_SAFE_NO_PAD.encode(&host.0))?;
        Ok(Self { directory, host })
    }
    pub fn pending(&self, app: AppId) -> Result<Option<Pending>, WriteError> {
        let bytes = read_from(&self.directory, OsStr::new(&filename(app, "pending")))?;
        let pending: Option<Pending> = bytes
            .map(|b| serde_json::from_slice(&b).map_err(|_| WriteError::InvalidResource))
            .transpose()?;
        if pending.as_ref().is_some_and(|p| p.version != 1 || p.app != app || p.host != self.host) {
            return Err(WriteError::InvalidResource);
        }
        Ok(pending)
    }
    /// Internal host observation; callers must authorize before exposing any scope.
    pub fn state(&self, app: AppId) -> Result<AppState, WriteError> {
        read_from(&self.directory, OsStr::new(&filename(app, "state")))?
            .map(|b| serde_json::from_slice(&b).map_err(|_| WriteError::InvalidResource))
            .transpose()
            .map(Option::unwrap_or_default)
    }
    /// Holds the physical application guard for staging, intent, publication and
    /// state settlement. Existing pending rejects every tenant until recovery.
    pub fn commit(
        &self,
        tenant: &TenantScope,
        guard: &mut AppWriteGuard,
        changes: &[FileChange<'_>],
        target: PendingTarget,
    ) -> Result<(), WriteError> {
        validate_target(&target)?;
        let app = guard.app();
        let _lock = self.lock(app)?;
        if self.pending(app)?.is_some() {
            return Err(WriteError::InvalidResource);
        }
        let mut state = self.state(app)?;
        let mut batch = guard.stage(changes)?;
        if batch.is_empty() {
            apply_target(&mut state, tenant, &target);
            return self.save(app, "state", &state);
        }
        // Persist staged directory entries before the intent can reference them.
        batch.writes[0].directory.sync_all().map_err(|_| WriteError::Io)?;
        let metadata = batch.writes[0].directory.metadata().map_err(|_| WriteError::Io)?;
        let mut pending = Pending {
            version: 1,
            host: self.host.clone(),
            app,
            tenant: tenant.clone(),
            directory_device: metadata.dev(),
            directory_inode: metadata.ino(),
            target,
            files: batch
                .writes
                .iter()
                .zip(&batch.revisions)
                .map(|(write, revision)| PendingFile {
                    name: write.name.as_bytes().to_vec(),
                    staged: write.temp.as_bytes().to_vec(),
                    pre: revision.before.clone(),
                    planned: revision.after.clone(),
                })
                .collect(),
            published: false,
        };
        self.save(app, "pending", &pending)?;
        // Durable intent owns these temporary files from here on. Errors and
        // unwind must retain them for the later recovery owner.
        for write in &mut batch.writes {
            write.retain = true;
        }
        checkpoint(self, app, Phase::Intent)?;
        pending.published = true;
        self.save(app, "pending", &pending)?;
        checkpoint(self, app, Phase::Marked)?;
        for write in &batch.writes {
            write.publish()?;
            write.directory.sync_all().map_err(|_| WriteError::Io)?;
        }
        checkpoint(self, app, Phase::Published)?;
        apply_target(&mut state, tenant, &pending.target);
        self.save(app, "state", &state)?;
        checkpoint(self, app, Phase::Settled)?;
        let name =
            CString::new(filename(app, "pending")).map_err(|_| WriteError::InvalidResource)?;
        // SAFETY: generated single child, descriptor retained through transaction.
        if unsafe { libc::unlinkat(self.directory.as_raw_fd(), name.as_ptr(), 0) } != 0 {
            return Err(WriteError::Io);
        }
        self.directory.sync_all().map_err(|_| WriteError::Io)
    }

    fn save(&self, app: AppId, kind: &str, value: &impl Serialize) -> Result<(), WriteError> {
        let bytes = serde_json::to_vec(value).map_err(|_| WriteError::InvalidResource)?;
        if bytes.len() > 1024 * 1024 {
            return Err(WriteError::InvalidResource);
        }
        prepare(&self.directory, OsStr::new(&filename(app, kind)), &bytes, FilePrivacy::Private)?
            .commit()?;
        self.directory.sync_all().map_err(|_| WriteError::Io)
    }
    fn lock(&self, app: AppId) -> Result<File, WriteError> {
        let name = CString::new(filename(app, "lock")).map_err(|_| WriteError::InvalidResource)?;
        // SAFETY: retained directory and validated generated name; owns returned fd.
        let fd = unsafe {
            libc::openat(
                self.directory.as_raw_fd(),
                name.as_ptr(),
                libc::O_RDWR
                    | libc::O_CREAT
                    | libc::O_NOFOLLOW
                    | libc::O_NONBLOCK
                    | libc::O_CLOEXEC,
                0o600 as libc::c_uint,
            )
        };
        if fd < 0 {
            return Err(WriteError::Io);
        }
        // SAFETY: newly owned descriptor.
        let file = unsafe { File::from_raw_fd(fd) };
        check_private(&file)?;
        if !file.metadata().map_err(|_| WriteError::Io)?.is_file() {
            return Err(WriteError::InvalidResource);
        }
        // SAFETY: valid descriptor; lock released by close on every exit path.
        if unsafe { libc::flock(fd, libc::LOCK_EX) } != 0 {
            return Err(WriteError::Io);
        }
        Ok(file)
    }
}
fn filename(app: AppId, kind: &str) -> String {
    format!(
        "{}.{kind}.json",
        serde_json::to_value(app)
            .expect("AppId serializes as a string")
            .as_str()
            .expect("AppId string")
    )
}
fn check_private(file: &File) -> Result<(), WriteError> {
    let metadata = file.metadata().map_err(|_| WriteError::Io)?;
    // SAFETY: geteuid has no preconditions.
    if metadata.mode() & 0o077 != 0 || metadata.uid() != unsafe { libc::geteuid() } {
        return Err(WriteError::InvalidResource);
    }
    Ok(())
}
fn private_child(parent: &File, name: &str) -> Result<File, WriteError> {
    let name = CString::new(name).map_err(|_| WriteError::InvalidResource)?;
    // SAFETY: fixed/base64 single child and valid parent descriptor.
    if unsafe { libc::mkdirat(parent.as_raw_fd(), name.as_ptr(), 0o700) } != 0
        && std::io::Error::last_os_error().kind() != std::io::ErrorKind::AlreadyExists
    {
        return Err(WriteError::Io);
    }
    // SAFETY: no-follow directory open; owns result.
    let fd = unsafe {
        libc::openat(
            parent.as_raw_fd(),
            name.as_ptr(),
            libc::O_RDONLY | libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC,
        )
    };
    if fd < 0 {
        return Err(WriteError::InvalidResource);
    }
    // SAFETY: newly owned descriptor.
    let file = unsafe { File::from_raw_fd(fd) };
    check_private(&file)?;
    parent.sync_all().map_err(|_| WriteError::Io)?;
    Ok(file)
}
fn validate_target(target: &PendingTarget) -> Result<(), WriteError> {
    for id in target.pointer.iter().chain(target.mode.iter().filter_map(|m| m.proxy_route.as_ref()))
    {
        crate::providers::validate_identity(id).map_err(|_| WriteError::InvalidResource)?;
    }
    if serde_json::to_vec(target).map_err(|_| WriteError::InvalidResource)?.len() > 65536 {
        return Err(WriteError::InvalidResource);
    }
    Ok(())
}
fn apply_target(state: &mut AppState, tenant: &TenantScope, target: &PendingTarget) {
    if let Some(id) = &target.pointer {
        state.pointer = Some(ScopedProvider { tenant: tenant.clone(), id: id.clone() });
    }
    if let Some(mode) = &target.mode {
        state.mode = Some(mode.clone());
        state.mode_tenant = Some(tenant.clone());
    }
}
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Phase {
    Intent,
    Marked,
    Published,
    Settled,
}
#[cfg(test)]
type CheckpointHook = Box<dyn FnMut(&ProviderTransactions, AppId, Phase) -> Result<(), WriteError>>;
#[cfg(test)]
thread_local! { pub(super) static HOOK: std::cell::RefCell<Option<CheckpointHook>> = const { std::cell::RefCell::new(None) }; }
fn checkpoint(store: &ProviderTransactions, app: AppId, phase: Phase) -> Result<(), WriteError> {
    #[cfg(test)]
    {
        HOOK.with_borrow_mut(|hook| match hook {
            Some(hook) => hook(store, app, phase),
            None => Ok(()),
        })
    }
    #[cfg(not(test))]
    {
        let _ = (store, app, phase);
        Ok(())
    }
}
