//! Authenticated, same-host copy import. HTTP cancellation does not stop the blocking
//! copy: callers must refresh after a timeout and must not automatically retry.
use std::net::SocketAddr;
use std::path::{Component, Path, PathBuf};

use super::{ApiError, resolve_file_root};
use axum::{
    Json, Router,
    extract::{ConnectInfo, DefaultBodyLimit},
    http::{HeaderMap, StatusCode},
    routing::post,
};
use vw_api_types::file::{LocalImportItemResult, LocalImportRequest, LocalImportResponse};

pub(super) fn router<S: Clone + Send + Sync + 'static>() -> Router<S> {
    Router::new().route(
        "/files/import-local",
        post(import_local).layer(DefaultBodyLimit::max(crate::app::agent::gateway::MAX_BODY_SIZE)),
    )
}

async fn import_local(
    axum::Extension(tenant): axum::Extension<vw_api_types::id::TenantScope>,
    peer: Option<axum::Extension<ConnectInfo<SocketAddr>>>,
    headers: HeaderMap,
    Json(body): Json<LocalImportRequest>,
) -> Result<Json<LocalImportResponse>, ApiError> {
    if !peer.is_some_and(|peer| peer.0.0.ip().is_loopback()) || !trusted_origin(&headers) {
        return Err(ApiError {
            status: StatusCode::FORBIDDEN,
            message: "local import requires a trusted loopback connection".into(),
        });
    }
    if body.source_paths.is_empty() || body.source_paths.len() > 128 {
        return Err(ApiError::bad_request("expected 1–128 source paths"));
    }
    // Resolve the authorized project before inspecting any external source.
    let root = resolve_file_root(
        &tenant,
        &body.project_id.0,
        body.worktree_id.as_ref().map(|id| id.0.as_str()),
    )
    .await?;
    tokio::task::spawn_blocking(move || {
        let root = PathBuf::from(root).canonicalize().map_err(io_error)?;
        let target = target_directory(&root, &body.target_directory)?;
        let items = body
            .source_paths
            .into_iter()
            .map(|source_path| match import_one(&root, &target, Path::new(&source_path)) {
                Ok(path) => LocalImportItemResult {
                    source_path,
                    destination_path: Some(path),
                    ok: true,
                    error: None,
                },
                Err(error) => LocalImportItemResult {
                    source_path,
                    destination_path: None,
                    ok: false,
                    error: Some(error.to_string()),
                },
            })
            .collect();
        Ok(Json(LocalImportResponse { items }))
    })
    .await
    .map_err(|_| {
        ApiError::internal("local import task failed; result unknown, refresh before retrying")
    })?
}

fn trusted_origin(headers: &HeaderMap) -> bool {
    let mut origins = headers.get_all("origin").iter();
    let Some(origin) = origins.next() else {
        return true;
    };
    if origins.next().is_some() {
        return false;
    }
    let Ok(origin) = origin.to_str() else {
        return false;
    };
    if matches!(origin, "tauri://localhost" | "http://tauri.localhost" | "https://tauri.localhost")
    {
        return true;
    }
    let Ok(url) = url::Url::parse(origin) else {
        return false;
    };
    url.scheme() == "http"
        && url.username().is_empty()
        && url.password().is_none()
        && url.path() == "/"
        && url.query().is_none()
        && url.fragment().is_none()
        && matches!(url.host_str(), Some("localhost" | "127.0.0.1" | "[::1]"))
}

pub(super) fn io_error(error: std::io::Error) -> ApiError {
    ApiError::bad_request(error.to_string())
}

pub(super) fn target_directory(root: &Path, relative: &str) -> Result<PathBuf, ApiError> {
    let mut target = root.to_path_buf();
    for component in Path::new(relative).components() {
        match component {
            Component::Normal(name) => target.push(name),
            Component::CurDir => continue,
            _ => {
                return Err(ApiError::bad_request("target must be a relative workspace directory"));
            }
        }
        let metadata = std::fs::symlink_metadata(&target).map_err(io_error)?;
        if unsafe_link(&metadata) || !metadata.is_dir() {
            return Err(ApiError::bad_request("target contains a link or is not a directory"));
        }
    }
    if !target.is_dir() || !target.canonicalize().map_err(io_error)?.starts_with(root) {
        return Err(ApiError::bad_request("target escapes workspace"));
    }
    Ok(target)
}

fn unsafe_link(metadata: &std::fs::Metadata) -> bool {
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        metadata.file_attributes() & 0x400 != 0
    }
    #[cfg(not(windows))]
    {
        metadata.file_type().is_symlink()
    }
}

fn import_one(root: &Path, target: &Path, source: &Path) -> Result<String, ApiError> {
    if !source.is_absolute() {
        return Err(ApiError::bad_request("source must be absolute"));
    }
    // Strip trailing separators and dot components before testing the final entry.
    let source: PathBuf = source.components().collect();
    let metadata = std::fs::symlink_metadata(&source).map_err(io_error)?;
    if unsafe_link(&metadata) {
        return Err(ApiError::bad_request("symbolic links are unsupported"));
    }
    let name = source
        .file_name()
        .filter(|name| name.to_str().is_some())
        .ok_or_else(|| ApiError::bad_request("source requires a UTF-8 basename"))?;
    let canonical_source = source.canonicalize().map_err(io_error)?;
    let destination = target.join(name);
    if target.starts_with(&canonical_source) || destination == canonical_source {
        return Err(ApiError::bad_request("cannot copy into the source or its descendants"));
    }
    match std::fs::symlink_metadata(&destination) {
        Ok(_) => return Err(ApiError::conflict("destination already exists")),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(io_error(error)),
    }
    let staging =
        tempfile::Builder::new().prefix(".shiju-import-").tempdir_in(target).map_err(io_error)?;
    let staged = staging.path().join("content");
    let result = (|| {
        copy_entry(&source, &staged, false)?;
        publish_no_replace(&staged, &destination).map_err(io_error)
    })();
    // Explicit close reports cleanup failures on normal error paths as well as success.
    let cleanup = staging.close().map_err(io_error);
    result?;
    cleanup?;
    let relative = destination
        .strip_prefix(root)
        .map_err(|_| ApiError::bad_request("invalid destination"))?
        .to_string_lossy()
        .into_owned();
    #[cfg(windows)]
    let relative = relative.replace('\\', "/");
    Ok(relative)
}

// Unix traversal keeps directory handles open and opens children relative to them.
// Replacing a path with a symlink during traversal cannot redirect source reads.
#[cfg(any(target_os = "linux", target_os = "macos"))]
pub(super) fn copy_entry(
    source: &Path,
    destination: &Path,
    preserve_mode: bool,
) -> Result<(), ApiError> {
    use std::os::unix::fs::OpenOptionsExt;
    let input = std::fs::OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
        .open(source)
        .map_err(io_error)?;
    copy_open_entry(input, destination, preserve_mode)
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
fn copy_open_entry(
    mut input: std::fs::File,
    destination: &Path,
    preserve_mode: bool,
) -> Result<(), ApiError> {
    use std::os::fd::{AsRawFd, FromRawFd, IntoRawFd};
    use std::os::unix::ffi::OsStrExt;
    use std::os::unix::fs::{DirBuilderExt, PermissionsExt};
    let metadata = input.metadata().map_err(io_error)?;
    if metadata.is_file() {
        let mut output = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(destination)
            .map_err(io_error)?;
        std::io::copy(&mut input, &mut output).map_err(io_error)?;
        if preserve_mode {
            output
                .set_permissions(std::fs::Permissions::from_mode(
                    metadata.permissions().mode() & 0o777,
                ))
                .map_err(io_error)?;
        }
        return Ok(());
    }
    if !metadata.is_dir() {
        return Err(ApiError::bad_request("special files are unsupported"));
    }
    let mut directory = std::fs::DirBuilder::new();
    if preserve_mode {
        // Keep existing group and world restrictions while allowing staging cleanup.
        directory.mode((metadata.permissions().mode() & 0o777) | 0o700);
    }
    directory.create(destination).map_err(io_error)?;
    let descriptor = input.try_clone().map_err(io_error)?.into_raw_fd();
    // fdopendir takes ownership only on success.
    let stream = unsafe { libc::fdopendir(descriptor) };
    if stream.is_null() {
        let error = std::io::Error::last_os_error();
        unsafe {
            libc::close(descriptor);
        }
        return Err(io_error(error));
    }
    struct DirectoryStream(*mut libc::DIR);
    impl Drop for DirectoryStream {
        fn drop(&mut self) {
            unsafe {
                libc::closedir(self.0);
            }
        }
    }
    let stream = DirectoryStream(stream);
    loop {
        // errno distinguishes end-of-directory from read failure.
        #[cfg(target_os = "linux")]
        let errno = unsafe { libc::__errno_location() };
        #[cfg(target_os = "macos")]
        let errno = unsafe { libc::__error() };
        unsafe {
            *errno = 0;
        }
        let entry = unsafe { libc::readdir(stream.0) };
        if entry.is_null() {
            if unsafe { *errno } != 0 {
                return Err(io_error(std::io::Error::last_os_error()));
            }
            break;
        }
        // Copy the name before the next readdir invalidates its buffer.
        let name = unsafe { std::ffi::CStr::from_ptr((*entry).d_name.as_ptr()) };
        if matches!(name.to_bytes(), b"." | b"..") {
            continue;
        }
        let descriptor = unsafe {
            libc::openat(
                input.as_raw_fd(),
                name.as_ptr(),
                libc::O_RDONLY | libc::O_CLOEXEC | libc::O_NOFOLLOW | libc::O_NONBLOCK,
            )
        };
        if descriptor < 0 {
            return Err(io_error(std::io::Error::last_os_error()));
        }
        let child = unsafe { std::fs::File::from_raw_fd(descriptor) };
        copy_open_entry(
            child,
            &destination.join(std::ffi::OsStr::from_bytes(name.to_bytes())),
            preserve_mode,
        )?;
    }
    Ok(())
}

#[cfg(windows)]
pub(super) fn copy_entry(
    source: &Path,
    destination: &Path,
    _preserve_mode: bool,
) -> Result<(), ApiError> {
    use std::os::windows::fs::OpenOptionsExt;
    // Open reparse points themselves and disallow replacement while reading.
    let mut input = std::fs::OpenOptions::new()
        .read(true)
        .share_mode(1)
        .custom_flags(0x0020_0000 | 0x0200_0000)
        .open(source)
        .map_err(io_error)?;
    let metadata = input.metadata().map_err(io_error)?;
    if unsafe_link(&metadata) {
        return Err(ApiError::bad_request("recursive links are unsupported"));
    }
    if metadata.is_dir() {
        std::fs::create_dir(destination).map_err(io_error)?;
        for entry in std::fs::read_dir(source).map_err(io_error)? {
            let entry = entry.map_err(io_error)?;
            copy_entry(&entry.path(), &destination.join(entry.file_name()), false)?;
        }
    } else if metadata.is_file() {
        let mut output = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(destination)
            .map_err(io_error)?;
        std::io::copy(&mut input, &mut output).map_err(io_error)?;
    } else {
        return Err(ApiError::bad_request("special files are unsupported"));
    }
    Ok(())
}

#[cfg(not(any(target_os = "linux", target_os = "macos", windows)))]
pub(super) fn copy_entry(
    _source: &Path,
    _destination: &Path,
    _preserve_mode: bool,
) -> Result<(), ApiError> {
    Err(ApiError::bad_request("safe local copy unsupported on this platform"))
}

pub(super) fn publish_no_replace(source: &Path, destination: &Path) -> std::io::Result<()> {
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    {
        use std::os::unix::ffi::OsStrExt;
        let source = std::ffi::CString::new(source.as_os_str().as_bytes())?;
        let destination = std::ffi::CString::new(destination.as_os_str().as_bytes())?;
        // Both C strings live through the syscall; exclusive rename preserves raced targets.
        #[cfg(target_os = "macos")]
        let result =
            unsafe { libc::renamex_np(source.as_ptr(), destination.as_ptr(), libc::RENAME_EXCL) };
        #[cfg(target_os = "linux")]
        let result = unsafe {
            libc::syscall(
                libc::SYS_renameat2,
                libc::AT_FDCWD,
                source.as_ptr(),
                libc::AT_FDCWD,
                destination.as_ptr(),
                libc::RENAME_NOREPLACE,
            )
        };
        if result == 0 { Ok(()) } else { Err(std::io::Error::last_os_error()) }
    }
    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStrExt;
        #[link(name = "kernel32")]
        unsafe extern "system" {
            fn MoveFileExW(existing: *const u16, new: *const u16, flags: u32) -> i32;
        }
        let source: Vec<u16> = source.as_os_str().encode_wide().chain(Some(0)).collect();
        let destination: Vec<u16> = destination.as_os_str().encode_wide().chain(Some(0)).collect();
        // Zero flags explicitly excludes MOVEFILE_REPLACE_EXISTING for files and directories.
        if unsafe { MoveFileExW(source.as_ptr(), destination.as_ptr(), 0) } != 0 {
            Ok(())
        } else {
            Err(std::io::Error::last_os_error())
        }
    }
    #[cfg(not(any(target_os = "linux", target_os = "macos", windows)))]
    {
        let _ = (source, destination);
        Err(std::io::Error::new(
            std::io::ErrorKind::Unsupported,
            "exclusive publication unavailable on this platform",
        ))
    }
}
