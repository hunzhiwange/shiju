//! Unix host file operations for the credential store.
use super::{
    AccountCredential, ClaudeAccountDirectory, CredentialKind, DIRECTORY_MARKER, INVALID_MATERIAL,
    MAX_BYTES, managed_config_path,
};
#[cfg(target_os = "macos")]
use std::ffi::OsStr;
use std::{
    ffi::{CStr, CString},
    fs::{self, File},
    io::{self, Read, Write},
    os::{
        fd::{AsRawFd, FromRawFd},
        unix::{
            ffi::OsStrExt,
            fs::{MetadataExt, OpenOptionsExt, PermissionsExt},
        },
    },
    path::{Component, Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
};
use vw_api_types::agent_apps::AppId;

const UNAVAILABLE: &str = "official credential storage unavailable";
const UNSAFE_PATH: &str = "unsafe official credential path";
const NOT_FOUND: &str = "official account not found";

pub(in crate::auth) struct ScopedCredentialPath {
    root: PathBuf,
    anchor: Arc<PrivateDirectory>,
    pub(in crate::auth) path: PathBuf,
}

impl ScopedCredentialPath {
    pub(super) fn new(root: PathBuf, anchor: Arc<PrivateDirectory>, path: PathBuf) -> Self {
        Self { root, anchor, path }
    }

    pub(in crate::auth) fn directory(&self, create: bool) -> Result<PrivateDirectory, String> {
        let relative =
            self.path.parent().and_then(|p| p.strip_prefix(&self.root).ok()).ok_or(UNSAFE_PATH)?;
        let mut directory = open_root(&self.root, &self.anchor)?;
        for part in relative.components() {
            let Component::Normal(part) = part else {
                return Err(UNSAFE_PATH.into());
            };
            let name = CString::new(part.as_bytes()).map_err(|_| UNSAFE_PATH)?;
            directory = directory.child_directory(&name, create)?;
        }
        Ok(directory)
    }
}

pub(super) fn open_root(
    path: &Path,
    anchor: &PrivateDirectory,
) -> Result<PrivateDirectory, String> {
    let file = crate::providers::paths::open_beneath(Path::new("/"), path, true)
        .map_err(|_| UNSAFE_PATH)?;
    check_private(&file, 0o700)?;
    let directory = PrivateDirectory(file);
    if !same_file(&directory.0, &anchor.0)? {
        return Err(UNSAFE_PATH.into());
    }
    Ok(directory)
}

pub(in crate::auth) struct PrivateDirectory(pub(in crate::auth) File);

impl PrivateDirectory {
    pub(super) fn open(path: &Path) -> Result<Self, String> {
        let file = File::options()
            .read(true)
            .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC)
            .open(path)
            .map_err(path_error)?;
        check_private(&file, 0o700)?;
        Ok(Self(file))
    }

    fn child_directory(&self, name: &CStr, create: bool) -> Result<Self, String> {
        if create {
            // SAFETY: name is a single NUL-terminated component and self owns the descriptor.
            let result = unsafe { libc::mkdirat(self.0.as_raw_fd(), name.as_ptr(), 0o700) };
            if result != 0 && io::Error::last_os_error().kind() != io::ErrorKind::AlreadyExists {
                return Err(UNAVAILABLE.into());
            }
        }
        let file =
            self.open_child(name, libc::O_RDONLY | libc::O_DIRECTORY, 0).map_err(path_error)?;
        check_private(&file, 0o700)?;
        Ok(Self(file))
    }

    pub(in crate::auth) fn open_child(
        &self,
        name: &CStr,
        flags: libc::c_int,
        mode: libc::c_uint,
    ) -> io::Result<File> {
        // SAFETY: owned directory descriptor and validated direct child name.
        let fd = unsafe {
            libc::openat(
                self.0.as_raw_fd(),
                name.as_ptr(),
                flags | libc::O_NOFOLLOW | libc::O_NONBLOCK | libc::O_CLOEXEC,
                mode,
            )
        };
        if fd < 0 {
            return Err(io::Error::last_os_error());
        }
        // SAFETY: openat returned a fresh descriptor, consumed exactly once.
        let file = unsafe { File::from_raw_fd(fd) };
        #[cfg(target_os = "macos")]
        if flags & libc::O_EXCL == 0 {
            check_child_spelling(&file, name)?;
        }
        Ok(file)
    }

    fn file(&self, name: &CStr) -> Result<Option<File>, String> {
        match self.open_child(name, libc::O_RDONLY, 0) {
            Ok(file) => {
                check_private(&file, 0o600)?;
                Ok(Some(file))
            }
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(path_error(e).into()),
        }
    }
}

#[cfg(target_os = "macos")]
fn check_child_spelling(file: &File, name: &CStr) -> io::Result<()> {
    // Existing launch paths use case-sensitive base64 even on case-insensitive APFS.
    let mut path = [0; libc::PATH_MAX as usize];
    // SAFETY: F_GETPATH writes at most PATH_MAX bytes into the caller's buffer.
    if unsafe { libc::fcntl(file.as_raw_fd(), libc::F_GETPATH, path.as_mut_ptr()) } == -1 {
        return Err(io::Error::last_os_error());
    }
    let actual = CStr::from_bytes_until_nul(&path)
        .ok()
        .and_then(|path| Path::new(OsStr::from_bytes(path.to_bytes())).file_name());
    if actual == Some(OsStr::from_bytes(name.to_bytes())) {
        Ok(())
    } else {
        Err(io::ErrorKind::InvalidData.into())
    }
}

fn path_error(error: io::Error) -> &'static str {
    if error.kind() == io::ErrorKind::NotFound { NOT_FOUND } else { UNSAFE_PATH }
}

pub(in crate::auth) fn child_name(path: &Path) -> Result<CString, String> {
    CString::new(path.file_name().ok_or(UNSAFE_PATH)?.as_bytes()).map_err(|_| UNSAFE_PATH.into())
}

pub(in crate::auth) fn check_private(file: &File, mode: u32) -> Result<(), String> {
    let metadata = file.metadata().map_err(|_| UNAVAILABLE)?;
    if (mode == 0o700 && !metadata.is_dir())
        // A concurrent atomic replacement can leave an already open reader with zero links.
        || (mode == 0o600 && (!metadata.is_file() || metadata.nlink() > 1))
    {
        return Err(UNSAFE_PATH.into());
    }
    // SAFETY: geteuid has no pointer arguments or preconditions.
    if metadata.uid() != unsafe { libc::geteuid() } || metadata.mode() & 0o7777 != mode {
        return Err("official credential permissions are not private".into());
    }
    #[cfg(target_os = "macos")]
    check_macos_acl(file)?;
    Ok(())
}

#[cfg(target_os = "macos")]
fn check_macos_acl(file: &File) -> Result<(), String> {
    // Darwin ACLs can grant access beyond the Unix mode bits.
    const ACL_TYPE_EXTENDED: libc::c_uint = 0x100;
    unsafe extern "C" {
        fn acl_get_fd_np(fd: libc::c_int, kind: libc::c_uint) -> *mut libc::c_void;
        fn acl_free(acl: *mut libc::c_void) -> libc::c_int;
    }
    // SAFETY: file owns a live descriptor; any returned ACL is released below.
    let acl = unsafe { acl_get_fd_np(file.as_raw_fd(), ACL_TYPE_EXTENDED) };
    if acl.is_null() {
        return if io::Error::last_os_error().kind() == io::ErrorKind::NotFound {
            Ok(())
        } else {
            Err(UNAVAILABLE.into())
        };
    }
    // SAFETY: the non-null pointer was allocated by acl_get_fd_np.
    unsafe { acl_free(acl) };
    Err("official credential permissions are not private".into())
}

pub(in crate::auth) fn same_file(a: &File, b: &File) -> Result<bool, String> {
    let a = a.metadata().map_err(|_| UNAVAILABLE)?;
    let b = b.metadata().map_err(|_| UNAVAILABLE)?;
    Ok((a.dev(), a.ino()) == (b.dev(), b.ino()))
}

pub(in crate::auth) fn read_from(
    directory: &PrivateDirectory,
    path: &Path,
    kind: CredentialKind,
) -> Result<AccountCredential, String> {
    let file = directory.file(&child_name(path)?)?.ok_or(NOT_FOUND)?;
    decode(directory, path, kind, &file)
}

fn decode(
    directory: &PrivateDirectory,
    path: &Path,
    kind: CredentialKind,
    file: &File,
) -> Result<AccountCredential, String> {
    let mut bytes = Vec::new();
    file.take((MAX_BYTES + 1) as u64).read_to_end(&mut bytes).map_err(|_| UNAVAILABLE)?;
    if bytes.len() > MAX_BYTES {
        return Err(INVALID_MATERIAL.into());
    }
    let credential = match kind {
        CredentialKind::App(AppId::Claude) if bytes == DIRECTORY_MARKER => {
            let path = managed_config_path(path);
            directory.child_directory(&child_name(&path)?, false)?;
            return Ok(AccountCredential::ClaudeDirectory(ClaudeAccountDirectory { path }));
        }
        CredentialKind::App(AppId::Claude) => {
            AccountCredential::ClaudeToken(String::from_utf8(bytes).map_err(|_| INVALID_MATERIAL)?)
        }
        CredentialKind::App(AppId::Codex) => AccountCredential::CodexAuth(
            serde_json::from_slice(&bytes).map_err(|_| INVALID_MATERIAL)?,
        ),
        CredentialKind::Copilot => {
            AccountCredential::CopilotToken(String::from_utf8(bytes).map_err(|_| INVALID_MATERIAL)?)
        }
        _ => return Err("official account service is not implemented".into()),
    };
    validate_kind(kind, &credential)?;
    Ok(credential)
}

fn validate_kind(kind: CredentialKind, credential: &AccountCredential) -> Result<(), String> {
    match (kind, credential) {
        (CredentialKind::App(app), _) => credential.validate(app),
        (CredentialKind::Copilot, AccountCredential::CopilotToken(token))
            if !token.is_empty()
                && token.len() <= 16_384
                && token.bytes().all(|byte| byte.is_ascii_graphic()) =>
        {
            Ok(())
        }
        (CredentialKind::Copilot, _) => Err(INVALID_MATERIAL.into()),
    }
}

pub(in crate::auth) fn remove_credential_file(
    directory: &PrivateDirectory,
    path: &Path,
) -> Result<(), String> {
    let name = child_name(path)?;
    if directory.file(&name)?.is_some() {
        // SAFETY: name is a single validated child of the leased directory.
        if unsafe { libc::unlinkat(directory.0.as_raw_fd(), name.as_ptr(), 0) } != 0 {
            return Err(UNAVAILABLE.into());
        }
    }
    Ok(())
}

pub(in crate::auth) fn write_to(
    directory: &PrivateDirectory,
    path: &Path,
    kind: CredentialKind,
    credential: &AccountCredential,
    recheck: impl FnOnce() -> Result<(), String>,
) -> Result<(), String> {
    if let AccountCredential::ClaudeDirectory(config) = credential {
        if !matches!(kind, CredentialKind::App(AppId::Claude))
            || config.path() != managed_config_path(path)
        {
            return Err("managed Claude directory scope mismatch".into());
        }
        directory.child_directory(&child_name(config.path())?, false)?;
    } else {
        validate_kind(kind, credential)?;
    }

    let name = child_name(path)?;
    let previous = directory.file(&name)?;
    if let Some(file) = &previous {
        match (decode(directory, path, kind, file)?, credential) {
            (AccountCredential::ClaudeToken(_), AccountCredential::ClaudeToken(_))
            | (AccountCredential::CopilotToken(_), AccountCredential::CopilotToken(_))
            | (AccountCredential::ClaudeDirectory(_), AccountCredential::ClaudeDirectory(_)) => {}
            (AccountCredential::CodexAuth(before), AccountCredential::CodexAuth(after))
                if before["tokens"]["account_id"] == after["tokens"]["account_id"] => {}
            _ => return Err("official account identity or representation changed".into()),
        }
    }
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let temp = loop {
        let temp = CString::new(format!(
            ".shiju-credential-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ))
        .expect("ASCII staging name");
        match directory.open_child(&temp, libc::O_WRONLY | libc::O_CREAT | libc::O_EXCL, 0o600) {
            Ok(file) => break (temp, file),
            Err(e) if e.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(_) => return Err(UNAVAILABLE.into()),
        }
    };
    let (temp, mut file) = temp;
    let fd = directory.0.as_raw_fd();
    let result = (|| {
        file.set_permissions(fs::Permissions::from_mode(0o600)).map_err(|_| UNAVAILABLE)?;
        check_private(&file, 0o600)?;
        file.write_all(&credential.bytes())
            .and_then(|_| file.sync_all())
            .map_err(|_| UNAVAILABLE)?;
        recheck()?;
        match (previous, directory.file(&name)?) {
            (Some(before), Some(now)) if same_file(&before, &now)? => {}
            (None, None) => {}
            _ => return Err("official credential changed before publication".into()),
        }
        // SAFETY: both names are validated children of the live directory descriptor.
        if unsafe { libc::renameat(fd, temp.as_ptr(), fd, name.as_ptr()) } != 0 {
            return Err(UNAVAILABLE.into());
        }
        Ok(())
    })();
    if result.is_err() {
        // SAFETY: unlink only the staging entry created exclusively by this call.
        unsafe { libc::unlinkat(fd, temp.as_ptr(), 0) };
    }
    result
}

pub(in crate::auth) fn register_claude_directory(
    directory: &PrivateDirectory,
    path: &Path,
    recheck: impl FnOnce() -> Result<(), String>,
) -> Result<(), String> {
    let config = managed_config_path(path);
    let name = child_name(&config)?;
    if let Some(file) = directory.file(&child_name(path)?)? {
        return match decode(directory, path, CredentialKind::App(AppId::Claude), &file)? {
            AccountCredential::ClaudeDirectory(_) => Ok(()),
            _ => Err("official account already exists".into()),
        };
    }

    let fd = directory.0.as_raw_fd();
    // SAFETY: fd is retained and name is a NUL-terminated direct child.
    if unsafe { libc::mkdirat(fd, name.as_ptr(), 0o700) } != 0 {
        return Err("cannot create managed Claude directory".into());
    }
    let credential = AccountCredential::ClaudeDirectory(ClaudeAccountDirectory { path: config });
    let result =
        write_to(directory, path, CredentialKind::App(AppId::Claude), &credential, recheck);
    if result.is_err() {
        // SAFETY: remove only the empty directory created by this call.
        unsafe { libc::unlinkat(fd, name.as_ptr(), libc::AT_REMOVEDIR) };
    }
    result
}
