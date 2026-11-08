//! Fixed Dsh paths, retained directory handles and native locks.
use super::{Snapshot, invalid};
use crate::providers::live::write_guard::WriteError;

use crate::providers::live::write_guard::{AppWriteGuard, DirectoryTarget};
use std::{
    ffi::{CString, OsStr},
    fs::{File, OpenOptions},
    os::{
        fd::{AsRawFd, FromRawFd},
        unix::fs::{MetadataExt, OpenOptionsExt},
    },
    path::Path,
};
use vw_api_types::agent_apps::AppId;
/// All target directories are retained and locked before any bytes are prepared.
pub struct DshFiles {
    pub guard: AppWriteGuard,
    keys: Vec<String>,
    _native_locks: Vec<NativeLock>,
}
fn child(parent: &File, name: &str, create: bool) -> Result<Option<File>, WriteError> {
    let name = CString::new(name).map_err(|_| WriteError::InvalidResource)?;
    if create
        && unsafe { libc::mkdirat(parent.as_raw_fd(), name.as_ptr(), 0o700) } != 0
        && std::io::Error::last_os_error().kind() != std::io::ErrorKind::AlreadyExists
    {
        return Err(WriteError::Io);
    }
    let fd = unsafe {
        libc::openat(
            parent.as_raw_fd(),
            name.as_ptr(),
            libc::O_RDONLY | libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC,
        )
    };
    if fd < 0 {
        return if !create && std::io::Error::last_os_error().kind() == std::io::ErrorKind::NotFound
        {
            Ok(None)
        } else {
            invalid()
        };
    }
    // SAFETY: new descriptor, owned once; every component was opened without following links.
    let f = unsafe { File::from_raw_fd(fd) };
    let m = f.metadata().map_err(|_| WriteError::Io)?;
    if m.uid() != unsafe { libc::geteuid() } || m.mode() & 0o022 != 0 {
        return invalid();
    }
    Ok(Some(f))
}
fn components(home: &Path) -> Result<Vec<String>, WriteError> {
    if !home.is_absolute() {
        return invalid();
    }
    let raw = std::env::var_os("DSH_HOME").filter(|v| !v.is_empty());
    let path = raw.map(std::path::PathBuf::from).unwrap_or_else(|| home.join(".dsh"));
    let relative = path.strip_prefix(home).map_err(|_| WriteError::InvalidResource)?;
    let parts = relative
        .components()
        .map(|c| match c {
            std::path::Component::Normal(v) => {
                v.to_str().map(str::to_owned).ok_or(WriteError::InvalidResource)
            }
            _ => invalid(),
        })
        .collect::<Result<Vec<_>, _>>()?;
    if parts.is_empty() {
        return invalid();
    }
    Ok(parts)
}
fn open_root(home: &Path, create: bool) -> Result<Option<File>, WriteError> {
    let components = components(home)?;
    let mut root = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC)
        .open(home)
        .map_err(|_| WriteError::InvalidResource)?;
    for component in components {
        let Some(directory) = child(&root, &component, create)? else {
            return Ok(None);
        };
        root = directory;
    }
    Ok(Some(root))
}

impl DshFiles {
    pub fn open(home: &Path) -> Result<Self, WriteError> {
        let root = open_root(home, true)?.ok_or(WriteError::Io)?;
        let profiles = child(&root, "profiles", true)?.unwrap();
        let mut dirs = vec![];
        let mut targets = vec![
            DirectoryTarget {
                key: "credentials".into(),
                directory: 0,
                name: ".credentials.yaml".into(),
            },
            DirectoryTarget { key: "home".into(), directory: 0, name: "cordis.patch.yml".into() },
        ];
        let mut locks = vec![
            NativeLock::acquire(&root, ".credentials.yaml.lock")?,
            NativeLock::acquire(&root, "cordis.patch.yml.lock")?,
        ];
        let mut keys = vec!["credentials".into(), "home".into()];
        for name in ["web", "desktop", "headless", "acp"] {
            let Some(dir) = child(&profiles, name, matches!(name, "web" | "desktop"))? else {
                continue;
            };
            if matches!(name, "headless" | "acp") {
                let rc = unsafe {
                    libc::faccessat(
                        dir.as_raw_fd(),
                        c"cordis.patch.yml".as_ptr(),
                        libc::F_OK,
                        libc::AT_SYMLINK_NOFOLLOW,
                    )
                };
                if rc != 0 {
                    if std::io::Error::last_os_error().kind() == std::io::ErrorKind::NotFound {
                        continue;
                    }
                    return invalid();
                }
            }
            // Symlink aliases are rejected by child; actual duplicate directory identities are rejected by the shared guard.
            locks.push(NativeLock::acquire(&dir, "cordis.patch.yml.lock")?);
            targets.push(DirectoryTarget {
                key: name.into(),
                directory: dirs.len() + 1,
                name: "cordis.patch.yml".into(),
            });
            keys.push(name.into());
            dirs.push(dir);
        }
        let guard =
            crate::providers::live::write_guard::lock_directories(AppId::Dsh, root, dirs, targets)?;
        Ok(Self { guard, keys, _native_locks: locks })
    }
    pub fn snapshot(&self) -> Result<Snapshot, WriteError> {
        self.keys.iter().map(|key| Ok((key.clone(), self.guard.read(OsStr::new(key))?))).collect()
    }
    pub fn apply(
        &mut self,
        tx: &crate::providers::live::commit::ProviderTransactions,
        tenant: &vw_api_types::id::TenantScope,
        expected: &crate::providers::live::commit::AppState,
        pre: &Snapshot,
        next: &Snapshot,
        unchanged: impl Fn() -> bool,
    ) -> Result<(), WriteError> {
        use crate::providers::live::{
            commit::*,
            stage::{FileChange, WholeFile},
            write_guard::FilePrivacy,
        };
        if expected.pointer.is_some()
            || expected.mode_tenant.as_ref().is_some_and(|t| t != tenant)
            || expected.mode.as_ref().is_some_and(|m| m.attached || m.mode != Mode::Direct)
        {
            return Err(WriteError::Conflict);
        }
        if pre.keys().ne(next.keys()) || pre.len() != self.keys.len() {
            return invalid();
        }
        let patches: Vec<_> = pre
            .iter()
            .map(|(key, bytes)| (key, WholeFile::new(bytes.as_deref(), next[key].clone())))
            .collect();
        let changes: Vec<_> = patches
            .iter()
            .map(|(key, patch)| FileChange {
                name: OsStr::new(key),
                privacy: FilePrivacy::Private,
                patch,
            })
            .collect();
        tx.commit_checked(
            tenant,
            &mut self.guard,
            &changes,
            PendingTarget {
                pointer: None,
                mode: Some(ModeState {
                    mode: Mode::Direct,
                    attached: false,
                    proxy_route: expected.mode.as_ref().and_then(|m| m.proxy_route.clone()),
                    contract: None,
                }),
                ..Default::default()
            },
            || {
                if unchanged()
                    && self._native_locks.iter().all(NativeLock::is_current)
                    && tx.state(AppId::Dsh)? == *expected
                {
                    Ok(())
                } else {
                    Err(WriteError::Conflict)
                }
            },
        )
    }
}
/// The account slot shares the provider adapter's physical application guard
/// and the native DSH lock, without opening or provisioning any profile files.
#[cfg(target_os = "macos")]
pub(crate) struct DshCredentials<'a> {
    home: &'a Path,
    root: File,
    guard: AppWriteGuard,
    lock: NativeLock,
}

#[cfg(target_os = "macos")]
impl<'a> DshCredentials<'a> {
    const FILE: &'static str = ".credentials.yaml";

    pub(crate) fn open(home: &'a Path, create: bool) -> Result<Option<Self>, WriteError> {
        let Some(root) = open_root(home, create)? else {
            return Ok(None);
        };
        let lock = NativeLock::acquire(&root, ".credentials.yaml.lock")?;
        let guard = crate::providers::live::write_guard::lock_directory(
            AppId::Dsh,
            root.try_clone().map_err(|_| WriteError::Io)?,
        )?;
        Ok(Some(Self { home, root, guard, lock }))
    }

    pub(crate) fn read(&self) -> Result<Option<Vec<u8>>, WriteError> {
        self.guard.read(OsStr::new(Self::FILE))
    }

    pub(crate) fn replace(&self, before: Option<&[u8]>, after: &[u8]) -> Result<(), WriteError> {
        if after.len() > super::MAX_YAML_BYTES {
            return Err(WriteError::InvalidResource);
        }
        let staged = crate::providers::live::write_guard::prepare_private_child(
            &self.root,
            OsStr::new(Self::FILE),
            after,
        )?;
        let current = open_root(self.home, false)?.ok_or(WriteError::Conflict)?;
        let retained = self.root.metadata().map_err(|_| WriteError::Io)?;
        let current = current.metadata().map_err(|_| WriteError::Io)?;
        if (current.dev(), current.ino()) != (retained.dev(), retained.ino())
            || !self.lock.is_current()
            || self.read()?.as_deref() != before
        {
            return Err(WriteError::Conflict);
        }
        staged.commit()
    }
}

/// Dsh's exclusive-create `<file>.lock` protocol; never remove an unknown or live owner's lock.
struct NativeLock {
    parent: File,
    name: CString,
    dev: u64,
    ino: u64,
}
impl NativeLock {
    fn acquire(parent: &File, name: &str) -> Result<Self, WriteError> {
        use std::io::Write;
        let name = CString::new(name).map_err(|_| WriteError::InvalidResource)?;
        let parent = parent.try_clone().map_err(|_| WriteError::Io)?;
        let fd = unsafe {
            libc::openat(
                parent.as_raw_fd(),
                name.as_ptr(),
                libc::O_WRONLY | libc::O_CREAT | libc::O_EXCL | libc::O_NOFOLLOW | libc::O_CLOEXEC,
                0o600,
            )
        };
        if fd < 0 {
            return Err(WriteError::Conflict);
        }
        // SAFETY: newly created exclusive descriptor, consumed once.
        let mut f = unsafe { File::from_raw_fd(fd) };
        let m = f.metadata().map_err(|_| WriteError::Io)?;
        let lock = Self { parent, name, dev: m.dev(), ino: m.ino() };
        writeln!(f, "{}", std::process::id()).map_err(|_| WriteError::Io)?;
        f.sync_all().map_err(|_| WriteError::Io)?;
        Ok(lock)
    }
    fn is_current(&self) -> bool {
        let mut m = std::mem::MaybeUninit::<libc::stat>::uninit();
        if unsafe {
            libc::fstatat(
                self.parent.as_raw_fd(),
                self.name.as_ptr(),
                m.as_mut_ptr(),
                libc::AT_SYMLINK_NOFOLLOW,
            )
        } != 0
        {
            return false;
        }
        let m = unsafe { m.assume_init() };
        m.st_dev as u64 == self.dev && m.st_ino == self.ino
    }
}
impl Drop for NativeLock {
    fn drop(&mut self) {
        if self.is_current() {
            unsafe {
                libc::unlinkat(self.parent.as_raw_fd(), self.name.as_ptr(), 0);
            }
        }
    }
}
/// Read observations without creating directories or taking native write locks.
pub fn read(home: &Path) -> Result<Snapshot, WriteError> {
    let parts = components(home)?;
    let mut result = Snapshot::new();
    for (key, tail) in [
        ("credentials", vec![".credentials.yaml"]),
        ("home", vec!["cordis.patch.yml"]),
        ("web", vec!["profiles", "web", "cordis.patch.yml"]),
        ("desktop", vec!["profiles", "desktop", "cordis.patch.yml"]),
        ("headless", vec!["profiles", "headless", "cordis.patch.yml"]),
        ("acp", vec!["profiles", "acp", "cordis.patch.yml"]),
    ] {
        let path: Vec<_> = parts.iter().map(String::as_str).chain(tail).collect();
        let bytes = crate::providers::catalog::read_file(home, &path)
            .map_err(|_| WriteError::InvalidResource)?;
        if bytes.is_some() || !matches!(key, "headless" | "acp") {
            result.insert(key.into(), bytes);
        }
    }
    Ok(result)
}
