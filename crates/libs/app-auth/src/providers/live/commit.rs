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
use {
    vw_api_app_identity::AppId,
    vw_api_app_auth::AuthRegion,
    vw_api_transport::id::TenantScope,
};

mod state;
pub use state::*;

pub struct ProviderTransactions {
    pub(super) directory: File,
    pub(super) host: ExecutionHostId,
}
impl ProviderTransactions {
    pub fn is_for_host(&self, host: &str) -> bool {
        self.host.0 == host
    }

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
    pub fn execution_host(&self) -> &ExecutionHostId {
        &self.host
    }
    pub fn pending(&self, app: AppId) -> Result<Option<Pending>, WriteError> {
        let bytes = read_from(&self.directory, OsStr::new(&filename(app, "pending")))?;
        let pending: Option<Pending> = bytes
            .map(|b| serde_json::from_slice(&b).map_err(|_| WriteError::InvalidResource))
            .transpose()?;
        if pending.as_ref().is_some_and(|p| {
            !matches!(p.version, 1..=11)
                || (p.version >= 10) != p.provider_record.is_some()
                || (p.version == 11) != p.target.aggregation_recovery.is_some()
                || (p.version >= 7) != p.target.project_scope.is_some()
                || (p.version == 8 && p.target.codex_aggregation_settings.is_none())
                || (p.version < 8 && p.target.codex_aggregation_settings.is_some())
                || (p.version == 9) != p.target.omo.is_some()
                || (p.version == 6 && p.target.claude_managed_login.is_none())
                || (p.version < 6 && p.target.claude_managed_login.is_some())
                || (p.version == 5 && p.target.kimi_binding.is_none())
                || (p.version < 5 && p.target.kimi_binding.is_some())
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
    /// Caller-triggered recovery must belong to the same tenant as the write.
    pub fn recover_owned(
        &self,
        tenant: &TenantScope,
        guard: &mut AppWriteGuard,
    ) -> Result<Option<super::recovery::RecoveryOutcome>, WriteError> {
        let app = guard.app();
        let _lock = self.lock(app)?;
        self.check_pending_owner(app, tenant)?;
        self.check_pending_project(app, None)?;
        self.recover_locked(guard)
    }

    fn check_pending_owner(&self, app: AppId, tenant: &TenantScope) -> Result<(), WriteError> {
        if self.pending(app)?.as_ref().is_some_and(|pending| pending.tenant != *tenant) {
            return Err(WriteError::Conflict);
        }
        Ok(())
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
        self.commit_with_record(tenant, guard, changes, target, validate, None)
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn commit_with_record(
        &self,
        tenant: &TenantScope,
        guard: &mut AppWriteGuard,
        changes: &[FileChange<'_>],
        target: PendingTarget,
        validate: impl Fn() -> Result<(), WriteError>,
        record: Option<(
            &super::record_publication::ProviderRecordChange,
            &dyn super::record_publication::ProviderRecordParticipant,
        )>,
    ) -> Result<(), WriteError> {
        let app = guard.app();
        if target.aggregation_recovery.is_some() && record.is_none() {
            return Err(WriteError::InvalidResource);
        }
        if let Some((change, _)) = record {
            change.validate(&self.host, app, tenant, &target)?;
        }
        validate_target(app, &target)?;
        self.validate_claude_login(tenant, &target)?;
        let _lock = self.lock(app)?;
        self.check_pending_owner(app, tenant)?;
        self.check_pending_project(app, target.project_scope.as_ref())?;
        if self.recover_locked(guard)?.is_some() {
            // Recovery can move state. The caller must re-read and replan.
            return Err(WriteError::Recovered);
        }
        self.register(guard)?;
        let mut state = self.state(app)?;
        validate_stack_update(&state, tenant, &target)?;
        super::project_binding::validate_update(&state, tenant, &target)?;
        let metadata = guard.directory.metadata().map_err(|_| WriteError::Io)?;
        let multi = guard.directories().count() > 1;
        crate::providers::operations::planned(changes.len());
        let mut batch = guard.stage(changes)?;
        validate()?;
        if batch.is_empty() && record.is_none() {
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
            provider_record: record.map(|(change, _)| change.clone()),
            project_preimage: state.project_scope.clone(),
            // Older executors must reject multi-directory intents and actual deletions.
            version: if target.aggregation_recovery.is_some() {
                11
            } else if record.is_some() {
                10
            } else if target.omo.is_some() {
                9
            } else if target.codex_aggregation_settings.is_some() {
                8
            } else if target.project_scope.is_some() {
                7
            } else if target.claude_managed_login.is_some() {
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
        if let Some((change, participant)) = record {
            participant.reserve(change)?;
        }
        validate()?;
        pending.published = true;
        self.save(app, "pending", &pending)?;
        checkpoint(self, app, Phase::Marked)?;
        self.publish_checked(&mut batch, changes, &mut pending)?;
        checkpoint(self, app, Phase::Published)?;
        if let Some((change, participant)) = record {
            participant.finish(change, true)?;
        }
        apply_target(&mut state, tenant, &pending.target);
        self.save(app, "state", &state)?;
        checkpoint(self, app, Phase::Settled)?;
        self.clear_pending(app)?;
        for _ in changes {
            crate::providers::operations::advanced();
        }
        Ok(())
    }

    /// A reference-only change has no native file publication to journal.
    pub fn remove_kimi_binding(
        &self,
        provider: &str,
        mut expected: AppState,
        validate: impl FnOnce() -> bool,
    ) -> Result<(), WriteError> {
        let app = AppId::Kimi;
        let _lock = self.lock(app)?;
        if self.pending(app)?.is_some() || self.state(app)? != expected || !validate() {
            return Err(WriteError::Conflict);
        }
        expected.kimi_bindings.remove(provider);
        self.save(app, "state", &expected)
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
mod target;
pub(super) use target::*;
#[cfg(any(test, feature = "test-support"))]
pub use target::{HOOK, Phase};

pub(super) fn checkpoint(
    store: &ProviderTransactions,
    app: AppId,
    phase: Phase,
) -> Result<(), WriteError> {
    #[cfg(any(test, feature = "test-support"))]
    {
        HOOK.with_borrow_mut(|hook| match hook {
            Some(hook) => hook(store, app, phase),
            None => Ok(()),
        })
    }
    #[cfg(not(any(test, feature = "test-support")))]
    {
        let _ = (store, app, phase);
        Ok(())
    }
}
