//! Execution-host state owner. Construct only from a trusted host's private data
//! root (the same source as CredentialFileStore), never a request path or HOME.
//! No application adapters, public file API or cross-file rollback.
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
use vw_api_types::{agent_apps::AppId, agent_apps_auth::AuthRegion, id::TenantScope};

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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub claude_login_operation: Option<String>,
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stack: Option<crate::providers::mode_state::StackState>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kimi_binding: Option<KimiBinding>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub claude_managed_login: Option<ClaudeManagedLogin>,
    pub pointer: Option<String>,
    pub mode: Option<ModeState>,
}
/// Scoped reference to the account whose login occupies the default Claude slot.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ClaudeManagedLogin {
    pub tenant: TenantScope,
    pub principal: String,
    pub host: String,
    pub account_id: String,
}
/// Reference only. Account credentials remain with the managed and native login owners.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KimiBinding {
    pub(crate) provider_id: String,
    pub(crate) principal: String,
    pub(crate) account_id: String,
    pub(crate) region: AuthRegion,
    pub(crate) slot: String,
}
#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct ScopedProvider {
    pub tenant: TenantScope,
    pub id: String,
}
#[derive(Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct AppState {
    #[serde(default, skip_serializing_if = "crate::providers::mode_state::StackState::is_empty")]
    pub stack: crate::providers::mode_state::StackState,
    #[serde(default)]
    pub stack_tenant: Option<TenantScope>,
    #[serde(default, skip_serializing_if = "std::collections::BTreeMap::is_empty")]
    pub kimi_bindings: std::collections::BTreeMap<String, KimiBinding>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub claude_managed_login: Option<ClaudeManagedLogin>,
    pub pointer: Option<ScopedProvider>,
    pub mode: Option<ModeState>,
    pub mode_tenant: Option<TenantScope>,
}
impl AppState {
    pub fn uses_kimi_account(&self, tenant: &TenantScope, principal: &str, account: &str) -> bool {
        self.mode_tenant.as_ref() == Some(tenant)
            && self
                .kimi_bindings
                .values()
                .any(|binding| binding.principal == principal && binding.account_id == account)
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct PendingFile {
    /// Index into the executor registration; legacy intents use the primary directory.
    #[serde(default, skip_serializing_if = "is_zero")]
    pub directory: usize,
    pub name: Vec<u8>,
    pub staged: Vec<u8>,
    pub pre: Option<String>,
    // Required field: a missing digest must never silently become a deletion.
    #[serde(deserialize_with = "Option::<String>::deserialize")]
    pub planned: Option<String>,
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

fn is_zero(index: &usize) -> bool {
    *index == 0
}

pub struct ProviderTransactions {
    pub(super) directory: File,
    pub(super) host: ExecutionHostId,
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
    /// Read-only opening for observations. Missing state is empty; never provisions,
    /// synchronizes, repairs or recovers a directory as a side effect of preview.
    pub fn open_existing(
        private_root: &Path,
        host: ExecutionHostId,
    ) -> Result<Option<Self>, WriteError> {
        if !private_root.is_absolute() {
            return Err(WriteError::InvalidResource);
        }
        let mut directory = match OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC)
            .open(private_root)
        {
            Ok(file) => file,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(_) => return Err(WriteError::InvalidResource),
        };
        check_private(&directory)?;
        for component in ["provider-live".to_string(), URL_SAFE_NO_PAD.encode(&host.0)] {
            let component = CString::new(component).map_err(|_| WriteError::InvalidResource)?;
            // SAFETY: single fixed/base64 child and a live directory descriptor.
            let fd = unsafe {
                libc::openat(
                    directory.as_raw_fd(),
                    component.as_ptr(),
                    libc::O_RDONLY | libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC,
                )
            };
            if fd < 0 {
                return if std::io::Error::last_os_error().kind() == std::io::ErrorKind::NotFound {
                    Ok(None)
                } else {
                    Err(WriteError::InvalidResource)
                };
            }
            // SAFETY: openat returned a fresh descriptor, consumed exactly once.
            directory = unsafe { File::from_raw_fd(fd) };
            check_private(&directory)?;
        }
        Ok(Some(Self { directory, host }))
    }
    pub(crate) fn execution_host(&self) -> &ExecutionHostId {
        &self.host
    }
    pub fn pending(&self, app: AppId) -> Result<Option<Pending>, WriteError> {
        let bytes = read_from(&self.directory, OsStr::new(&filename(app, "pending")))?;
        let pending: Option<Pending> = bytes
            .map(|b| serde_json::from_slice(&b).map_err(|_| WriteError::InvalidResource))
            .transpose()?;
        if pending.as_ref().is_some_and(|p| {
            !matches!(p.version, 1..=6)
                || (p.version == 6) != p.target.claude_managed_login.is_some()
                || (p.version == 5) != p.target.kimi_binding.is_some()
                || (p.version < 4 && p.target.stack.is_some())
                || (p.version < 3 && p.files.iter().any(|f| f.directory != 0))
                || (p.version == 1 && p.files.iter().any(|f| f.planned.is_none()))
                || p.app != app
                || p.host != self.host
        }) {
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
    /// state settlement. Recovery stops this request so the caller can replan.
    pub fn commit(
        &self,
        tenant: &TenantScope,
        guard: &mut AppWriteGuard,
        changes: &[FileChange<'_>],
        target: PendingTarget,
    ) -> Result<(), WriteError> {
        self.commit_checked(tenant, guard, changes, target, || Ok(()))
    }
    /// Executor-owned saved-input validation immediately before publication starts.
    /// After the durable publication marker, recovery forwards the frozen intent.
    pub fn commit_checked(
        &self,
        tenant: &TenantScope,
        guard: &mut AppWriteGuard,
        changes: &[FileChange<'_>],
        target: PendingTarget,
        validate: impl Fn() -> Result<(), WriteError>,
    ) -> Result<(), WriteError> {
        let app = guard.app();
        validate_target(app, &target)?;
        self.validate_claude_login(tenant, &target)?;
        let _lock = self.lock(app)?;
        if self.recover_locked(guard)?.is_some() {
            // Recovery can move state. The caller must re-read and replan.
            return Err(WriteError::Recovered);
        }
        self.register(guard)?;
        let mut state = self.state(app)?;
        validate_stack_update(&state, tenant, &target)?;
        let metadata = guard.directory.metadata().map_err(|_| WriteError::Io)?;
        let multi = guard.directories().count() > 1;
        crate::providers::operations::planned(changes.len());
        let mut batch = guard.stage(changes)?;
        validate()?;
        if batch.is_empty() {
            crate::providers::operations::publication().map_err(|_| WriteError::Conflict)?;
            apply_target(&mut state, tenant, &target);
            self.save(app, "state", &state)?;
            for _ in changes {
                crate::providers::operations::advanced();
            }
            return Ok(());
        }
        // Persist staged directory entries before the intent can reference them.
        for write in &batch.writes {
            write.directory.sync_all().map_err(|_| WriteError::Io)?;
        }
        let mut pending = Pending {
            // Older executors must reject multi-directory intents and actual deletions.
            version: if target.claude_managed_login.is_some() {
                6
            } else if target.kimi_binding.is_some() {
                5
            } else if target.stack.is_some() {
                4
            } else if multi {
                3
            } else if batch.revisions.iter().any(|r| r.after.is_none()) {
                2
            } else {
                1
            },
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
                    directory: revision.directory,
                    name: write.name.as_bytes().to_vec(),
                    staged: write.temp.as_bytes().to_vec(),
                    pre: revision.before.clone(),
                    planned: revision.after.clone(),
                })
                .collect(),
            published: false,
        };
        crate::providers::operations::publication().map_err(|_| WriteError::Conflict)?;
        self.save(app, "pending", &pending)?;
        // Durable intent owns these temporary files from here on. Errors and
        // unwind must retain them for the later recovery owner.
        for write in &mut batch.writes {
            write.retain = true;
        }
        checkpoint(self, app, Phase::Intent)?;
        validate()?;
        pending.published = true;
        self.save(app, "pending", &pending)?;
        checkpoint(self, app, Phase::Marked)?;
        self.publish_checked(&mut batch, changes, &mut pending)?;
        checkpoint(self, app, Phase::Published)?;
        apply_target(&mut state, tenant, &pending.target);
        self.save(app, "state", &state)?;
        checkpoint(self, app, Phase::Settled)?;
        self.clear_pending(app)?;
        for _ in changes {
            crate::providers::operations::advanced();
        }
        Ok(())
    }

    pub(super) fn validate_claude_login(
        &self,
        tenant: &TenantScope,
        target: &PendingTarget,
    ) -> Result<(), WriteError> {
        if let Some(login) = &target.claude_managed_login
            && (&login.tenant != tenant || ExecutionHostId::new(&login.host)? != self.host)
        {
            return Err(WriteError::Conflict);
        }
        Ok(())
    }

    pub(super) fn clear_pending(&self, app: AppId) -> Result<(), WriteError> {
        let name =
            CString::new(filename(app, "pending")).map_err(|_| WriteError::InvalidResource)?;
        // SAFETY: generated single child, descriptor retained through transaction.
        if unsafe { libc::unlinkat(self.directory.as_raw_fd(), name.as_ptr(), 0) } != 0 {
            return Err(WriteError::Io);
        }
        self.directory.sync_all().map_err(|_| WriteError::Io)
    }

    pub(super) fn save(
        &self,
        app: AppId,
        kind: &str,
        value: &impl Serialize,
    ) -> Result<(), WriteError> {
        let bytes = serde_json::to_vec(value).map_err(|_| WriteError::InvalidResource)?;
        if bytes.len() > 1024 * 1024 {
            return Err(WriteError::InvalidResource);
        }
        prepare(&self.directory, OsStr::new(&filename(app, kind)), &bytes, FilePrivacy::Private)?
            .commit()?;
        self.directory.sync_all().map_err(|_| WriteError::Io)
    }
    pub(super) fn lock(&self, app: AppId) -> Result<File, WriteError> {
        self.lock_named(app, "lock")
    }

    pub(super) fn lock_named(&self, app: AppId, kind: &str) -> Result<File, WriteError> {
        let name = CString::new(filename(app, kind)).map_err(|_| WriteError::InvalidResource)?;
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
pub(super) fn filename(app: AppId, kind: &str) -> String {
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
pub(super) fn validate_target(app: AppId, target: &PendingTarget) -> Result<(), WriteError> {
    if let Some(login) = &target.claude_managed_login {
        if app != AppId::Claude
            || target.kimi_binding.is_some()
            || target.stack.is_some()
            || login.principal.trim().is_empty()
            || login.principal.chars().any(char::is_control)
            || !target.mode.as_ref().is_some_and(|mode| mode.mode == Mode::Direct && !mode.attached)
        {
            return Err(WriteError::InvalidResource);
        }
        crate::providers::validate_identity(&login.account_id)
            .map_err(|_| WriteError::InvalidResource)?;
    }
    if let Some(binding) = &target.kimi_binding {
        if app != AppId::Kimi
            || target.pointer.is_some()
            || target.stack.is_some()
            || !target.mode.as_ref().is_some_and(|m| m.mode == Mode::Direct && !m.attached)
            || binding.principal.trim().is_empty()
            || binding.principal.chars().any(char::is_control)
        {
            return Err(WriteError::InvalidResource);
        }
        crate::providers::kimi::validate_identity(&binding.provider_id)?;
        crate::providers::validate_identity(&binding.account_id)
            .map_err(|_| WriteError::InvalidResource)?;
    }
    for id in target.pointer.iter().chain(target.mode.iter().filter_map(|m| m.proxy_route.as_ref()))
    {
        crate::providers::validate_identity(id).map_err(|_| WriteError::InvalidResource)?;
    }
    if let Some(stack) = &target.stack {
        let mut members = std::collections::HashSet::new();
        for id in &stack.members {
            crate::providers::validate_identity(id).map_err(|_| WriteError::InvalidResource)?;
            if !members.insert(id) {
                return Err(WriteError::InvalidResource);
            }
        }
        for (key, id) in &stack.keys {
            crate::providers::validate_identity(key).map_err(|_| WriteError::InvalidResource)?;
            crate::providers::validate_identity(id).map_err(|_| WriteError::InvalidResource)?;
        }
    }
    if serde_json::to_vec(target).map_err(|_| WriteError::InvalidResource)?.len() > 65536 {
        return Err(WriteError::InvalidResource);
    }
    Ok(())
}
/// Stack keys are permanent reservations: removal and reassignment are forbidden.
/// The stack has its own tenant owner, independent of direct-pointer changes.
pub(super) fn validate_stack_update(
    state: &AppState,
    tenant: &TenantScope,
    target: &PendingTarget,
) -> Result<(), WriteError> {
    if let Some(stack) = &target.stack
        && (state.stack_tenant.as_ref().is_some_and(|t| t != tenant)
            || !state.stack.is_empty() && state.stack_tenant.as_ref() != Some(tenant)
            || state.stack.keys.iter().any(|(key, id)| stack.keys.get(key) != Some(id)))
    {
        return Err(WriteError::Conflict);
    }
    Ok(())
}
pub(super) fn apply_target(state: &mut AppState, tenant: &TenantScope, target: &PendingTarget) {
    if let Some(id) = &target.pointer {
        state.pointer = Some(ScopedProvider { tenant: tenant.clone(), id: id.clone() });
    }
    if let Some(stack) = &target.stack {
        state.stack = stack.clone();
        state.stack_tenant = Some(tenant.clone());
    }
    if let Some(login) = &target.claude_managed_login {
        state.claude_managed_login = Some(login.clone());
    }
    if let Some(binding) = &target.kimi_binding {
        state.kimi_bindings.insert(binding.provider_id.clone(), binding.clone());
    }
    if let Some(mode) = &target.mode {
        state.mode = Some(mode.clone());
        state.mode_tenant = Some(tenant.clone());
    }
}
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Phase {
    Intent,
    Marked,
    BeforePublish,
    FilePublished,
    Published,
    Settled,
}
#[cfg(test)]
type CheckpointHook = Box<dyn FnMut(&ProviderTransactions, AppId, Phase) -> Result<(), WriteError>>;
#[cfg(test)]
thread_local! { pub(crate) static HOOK: std::cell::RefCell<Option<CheckpointHook>> = const { std::cell::RefCell::new(None) }; }
pub(super) fn checkpoint(
    store: &ProviderTransactions,
    app: AppId,
    phase: Phase,
) -> Result<(), WriteError> {
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
