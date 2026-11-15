use super::*;

fn components(home: &Path) -> Result<Vec<String>, WriteError> {
    if !home.is_absolute() || std::env::var_os("KIMI_CODE_HOME").is_some_and(|v| !v.is_empty()) {
        return invalid();
    }
    Ok(vec![".kimi-code".into()])
}
pub fn read(home: &Path) -> Result<Option<Vec<u8>>, WriteError> {
    components(home)?;
    super::super::catalog::read_file(home, &[".kimi-code", "config.toml"])
        .map_err(|_| WriteError::InvalidResource)
}
pub struct KimiFiles {
    pub guard: super::super::live::write_guard::AppWriteGuard,
}
impl KimiFiles {
    pub fn open(home: &Path) -> Result<Self, WriteError> {
        use std::os::{
            fd::{AsRawFd, FromRawFd},
            unix::fs::OpenOptionsExt,
        };
        if !home.is_absolute() {
            return invalid();
        }
        let mut dir = std::fs::OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC)
            .open(home)
            .map_err(|_| WriteError::InvalidResource)?;
        for component in components(home)? {
            let name =
                std::ffi::CString::new(component).map_err(|_| WriteError::InvalidResource)?;
            // SAFETY: fixed single child of the retained authorized directory.
            if unsafe { libc::mkdirat(dir.as_raw_fd(), name.as_ptr(), 0o700) } != 0
                && std::io::Error::last_os_error().kind() != std::io::ErrorKind::AlreadyExists
            {
                return Err(WriteError::Io);
            }
            // SAFETY: fixed child; no symlinks; returned fd consumed once.
            let fd = unsafe {
                libc::openat(
                    dir.as_raw_fd(),
                    name.as_ptr(),
                    libc::O_RDONLY | libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC,
                )
            };
            if fd < 0 {
                return invalid();
            }
            dir = unsafe { std::fs::File::from_raw_fd(fd) };
        }
        Ok(Self { guard: super::super::live::write_guard::lock_directory(AppId::Kimi, dir)? })
    }
    pub fn snapshot(&self) -> Result<(&'static str, Option<Vec<u8>>), WriteError> {
        Ok(("config.toml", self.guard.read(OsStr::new("config.toml"))?))
    }
    #[allow(clippy::too_many_arguments)]
    pub fn apply(
        &mut self,
        tx: &super::super::live::commit::ProviderTransactions,
        tenant: &vw_api_transport::TenantScope,
        expected: &super::super::live::commit::AppState,
        name: &str,
        pre: Option<&[u8]>,
        next: Vec<u8>,
        unchanged: impl Fn() -> bool,
    ) -> Result<(), WriteError> {
        self.apply_with_binding(tx, tenant, expected, name, pre, next, None, unchanged)
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn apply_with_binding(
        &mut self,
        tx: &super::super::live::commit::ProviderTransactions,
        tenant: &vw_api_transport::TenantScope,
        expected: &super::super::live::commit::AppState,
        name: &str,
        pre: Option<&[u8]>,
        next: Vec<u8>,
        binding: Option<super::super::live::commit::KimiBinding>,
        unchanged: impl Fn() -> bool,
    ) -> Result<(), WriteError> {
        use super::super::live::{
            commit::*,
            stage::{FileChange, WholeFile},
            write_guard::FilePrivacy,
        };
        if name != "config.toml" {
            return invalid();
        }
        if expected.pointer.is_some()
            || expected.mode_tenant.as_ref().is_some_and(|t| t != tenant)
            || expected.mode.as_ref().is_some_and(|m| m.attached || m.mode != Mode::Direct)
        {
            return Err(WriteError::Conflict);
        }
        if !expected.kimi_bindings.is_empty() {
            let before = document(pre)?;
            let after = document(Some(&next))?;
            for id in expected.kimi_bindings.keys() {
                if before["providers"].get(id) != after["providers"].get(id) {
                    return Err(WriteError::Conflict);
                }
            }
        }
        let after = if pre.is_none() && next.is_empty() { None } else { Some(next) };
        let patch = WholeFile::new(pre, after);
        tx.commit_checked(
            tenant,
            &mut self.guard,
            &[FileChange { name: OsStr::new(name), privacy: FilePrivacy::Private, patch: &patch }],
            PendingTarget {
                pointer: None,
                kimi_binding: binding,
                mode: Some(ModeState {
                    mode: Mode::Direct,
                    attached: false,
                    proxy_route: expected.mode.as_ref().and_then(|m| m.proxy_route.clone()),
                    contract: None,
                }),
                ..Default::default()
            },
            || {
                if unchanged() && tx.state(AppId::Kimi)? == *expected {
                    Ok(())
                } else {
                    Err(WriteError::Conflict)
                }
            },
        )
    }
}
