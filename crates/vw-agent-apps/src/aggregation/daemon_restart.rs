//! Restricted host-side restart. Never accepts command, PID or environment from an API request.
use super::catalog_generation;
use crate::providers::{codex_catalog::CodexFiles, live::commit::ProviderTransactions};
use serde::Deserialize;
use std::{
    ffi::OsStr,
    fs::File,
    io::Read,
    os::{
        fd::{AsRawFd, FromRawFd},
        unix::{ffi::OsStrExt, fs::MetadataExt},
    },
    path::{Path, PathBuf},
    process::Stdio,
    time::Duration,
};
use tokio::{io::AsyncReadExt, process::Command, task::JoinHandle};
use vw_api_types::id::ProjectScope;

const ARGS: [&str; 3] = ["app-server", "daemon", "restart"];
const LIMIT: usize = 64 * 1024;
const TIMEOUT: Duration = Duration::from_secs(375);
#[derive(Debug, PartialEq, Eq)]
pub enum RestartOutcome {
    Restarted,
    NotRunning,
}
#[derive(Debug, PartialEq, Eq)]
pub enum RestartError {
    ConfirmationRequired,
    Forbidden,
    RevisionConflict,
    UnsupportedTarget,
    TargetUnavailable,
    TargetFailed,
    TargetTimeout,
}
/// Host policy supplies this absolute trusted installation path, never request JSON.
pub struct RestartRequest<'a> {
    pub expected_revision: &'a str,
    pub confirmed: bool,
    pub executable: &'a Path,
}
/// Retain this operation until terminal, even when the HTTP caller stops waiting.
/// Dropping a wait future does not kill the command or report rollback/cancellation.
pub struct RestartOperation {
    task: JoinHandle<Result<RestartOutcome, RestartError>>,
}
impl RestartOperation {
    pub async fn wait(&mut self) -> Result<RestartOutcome, RestartError> {
        (&mut self.task).await.map_err(|_| RestartError::TargetFailed)?
    }
}
/// Caller holds the Codex physical guard and rechecks independent restart authorization.
/// The closure must authorize this host/project/app/subject on every invocation.
pub fn restart_daemon(
    files: &CodexFiles,
    tx: &ProviderTransactions,
    scope: &ProjectScope,
    request: RestartRequest<'_>,
    authorized: impl Fn() -> bool,
) -> Result<RestartOperation, RestartError> {
    restart_with(files, tx, scope, request, authorized, |executable, home| {
        run_command(executable, home, TIMEOUT)
    })
}
fn restart_with(
    files: &CodexFiles,
    tx: &ProviderTransactions,
    scope: &ProjectScope,
    request: RestartRequest<'_>,
    authorized: impl Fn() -> bool,
    run: impl FnOnce(
        PathBuf,
        PathBuf,
    ) -> Result<JoinHandle<Result<RestartOutcome, RestartError>>, RestartError>,
) -> Result<RestartOperation, RestartError> {
    if !authorized() {
        return Err(RestartError::Forbidden);
    }
    if !request.confirmed {
        return Err(RestartError::ConfirmationRequired);
    }
    let check_revision = || {
        let snapshot = catalog_generation::read(files, tx, scope)
            .map_err(|_| RestartError::TargetUnavailable)?;
        if snapshot.revision() != request.expected_revision {
            return Err(RestartError::RevisionConflict);
        }
        Ok(())
    };
    check_revision()?;
    let executable = trusted_executable(request.executable)?;
    let directory = files.guard.retained_directory();
    let home = directory_path(directory)?;
    let stamp = executable_stamp(&executable)?;
    let before = read_record(directory)?;
    let Some(record) = before else {
        return Ok(RestartOperation {
            task: tokio::spawn(async { Ok(RestartOutcome::NotRunning) }),
        });
    };
    if let Err(error) = verify_process(&record, &executable, &home) {
        // ESRCH alone proves this recorded PID is absent; EPERM/unknown remain errors.
        if unsafe { libc::kill(record.pid as i32, 0) } == -1
            && std::io::Error::last_os_error().raw_os_error() == Some(libc::ESRCH)
        {
            return Ok(RestartOperation {
                task: tokio::spawn(async { Ok(RestartOutcome::NotRunning) }),
            });
        }
        return Err(error);
    }
    // Check all capabilities again immediately before handing off the fixed command.
    if !authorized() {
        return Err(RestartError::Forbidden);
    }
    check_revision()?;
    if read_record(directory)?.as_ref() != Some(&record)
        || trusted_executable(request.executable)? != executable
        || executable_stamp(&executable)? != stamp
        || directory_path(directory)? != home
    {
        return Err(RestartError::RevisionConflict);
    }
    verify_process(&record, &executable, &home)?;
    Ok(RestartOperation { task: run(executable, home)? })
}
fn executable_stamp(path: &Path) -> Result<(u64, u64, i64, i64, u64), RestartError> {
    let m = path.metadata().map_err(|_| RestartError::UnsupportedTarget)?;
    Ok((m.dev(), m.ino(), m.ctime(), m.ctime_nsec(), m.len()))
}
fn trusted_executable(path: &Path) -> Result<PathBuf, RestartError> {
    if !path.is_absolute() {
        return Err(RestartError::UnsupportedTarget);
    }
    let canonical = path.canonicalize().map_err(|_| RestartError::UnsupportedTarget)?;
    let metadata = canonical.metadata().map_err(|_| RestartError::UnsupportedTarget)?;
    if canonical != path
        || canonical.file_name() != Some(OsStr::new("codex"))
        || !metadata.is_file()
        || metadata.mode() & 0o022 != 0
        || metadata.mode() & 0o111 == 0
        || (metadata.uid() != 0 && metadata.uid() != unsafe { libc::geteuid() })
    {
        return Err(RestartError::UnsupportedTarget);
    }
    Ok(canonical)
}
fn directory_path(directory: &File) -> Result<PathBuf, RestartError> {
    let mut buffer = [0u8; libc::PATH_MAX as usize];
    // SAFETY: F_GETPATH writes at most PATH_MAX bytes to this retained descriptor's buffer.
    if unsafe { libc::fcntl(directory.as_raw_fd(), libc::F_GETPATH, buffer.as_mut_ptr()) } != 0 {
        return Err(RestartError::TargetUnavailable);
    }
    let end = buffer.iter().position(|b| *b == 0).ok_or(RestartError::TargetUnavailable)?;
    let path = PathBuf::from(OsStr::from_bytes(&buffer[..end]));
    let expected = directory.metadata().map_err(|_| RestartError::TargetUnavailable)?;
    if expected.uid() != unsafe { libc::geteuid() } || expected.mode() & 0o022 != 0 {
        return Err(RestartError::TargetUnavailable);
    }
    let actual = path.symlink_metadata().map_err(|_| RestartError::TargetUnavailable)?;
    if !path.is_absolute()
        || actual.dev() != expected.dev()
        || actual.ino() != expected.ino()
        || !actual.is_dir()
    {
        return Err(RestartError::TargetUnavailable);
    }
    Ok(path)
}
#[derive(Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
struct Record {
    pid: u32,
    process_identity: Identity,
}
#[derive(Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
struct Identity {
    start_seconds: u64,
    start_microseconds: u64,
}
fn read_record(directory: &File) -> Result<Option<Record>, RestartError> {
    // SAFETY: fixed no-follow children relative to the retained authorized directory.
    let fd = unsafe {
        libc::openat(
            directory.as_raw_fd(),
            c"app-server-daemon".as_ptr(),
            libc::O_RDONLY | libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC,
        )
    };
    if fd < 0 {
        return if std::io::Error::last_os_error().kind() == std::io::ErrorKind::NotFound {
            Ok(None)
        } else {
            Err(RestartError::TargetUnavailable)
        };
    }
    let daemon = unsafe { File::from_raw_fd(fd) };
    let metadata = daemon.metadata().map_err(|_| RestartError::TargetUnavailable)?;
    if metadata.uid() != unsafe { libc::geteuid() } || metadata.mode() & 0o022 != 0 {
        return Err(RestartError::TargetUnavailable);
    }
    let fd = unsafe {
        libc::openat(
            daemon.as_raw_fd(),
            c"daemon.pid".as_ptr(),
            libc::O_RDONLY | libc::O_NOFOLLOW | libc::O_CLOEXEC | libc::O_NONBLOCK,
        )
    };
    if fd < 0 {
        return if std::io::Error::last_os_error().kind() == std::io::ErrorKind::NotFound {
            Ok(None)
        } else {
            Err(RestartError::TargetUnavailable)
        };
    }
    let file = unsafe { File::from_raw_fd(fd) };
    let metadata = file.metadata().map_err(|_| RestartError::TargetUnavailable)?;
    if !metadata.is_file()
        || metadata.uid() != unsafe { libc::geteuid() }
        || metadata.mode() & 0o022 != 0
    {
        return Err(RestartError::TargetUnavailable);
    }
    let mut bytes = Vec::new();
    file.take((LIMIT + 1) as u64)
        .read_to_end(&mut bytes)
        .map_err(|_| RestartError::TargetUnavailable)?;
    if bytes.len() > LIMIT {
        return Err(RestartError::TargetUnavailable);
    }
    let record: Record =
        serde_json::from_slice(&bytes).map_err(|_| RestartError::UnsupportedTarget)?;
    if record.pid <= 1
        || record.pid > i32::MAX as u32
        || record.process_identity.start_microseconds >= 1_000_000
    {
        return Err(RestartError::TargetUnavailable);
    }
    Ok(Some(record))
}
fn native_identity(pid: u32) -> Result<Identity, RestartError> {
    let mut info = std::mem::MaybeUninit::<libc::proc_bsdinfo>::uninit();
    let size = std::mem::size_of::<libc::proc_bsdinfo>() as i32;
    // SAFETY: exact-sized aligned buffer, read only after complete initialization.
    let read = unsafe {
        libc::proc_pidinfo(pid as i32, libc::PROC_PIDTBSDINFO, 1, info.as_mut_ptr().cast(), size)
    };
    if read != size {
        return Err(RestartError::TargetUnavailable);
    }
    let info = unsafe { info.assume_init() };
    if info.pbi_uid != unsafe { libc::geteuid() } || info.pbi_status == libc::SZOMB {
        return Err(RestartError::TargetUnavailable);
    }
    Ok(Identity { start_seconds: info.pbi_start_tvsec, start_microseconds: info.pbi_start_tvusec })
}
fn verify_process(record: &Record, executable: &Path, home: &Path) -> Result<(), RestartError> {
    if native_identity(record.pid)? != record.process_identity {
        return Err(RestartError::TargetUnavailable);
    }
    let mut buffer = [0u8; libc::PROC_PIDPATHINFO_MAXSIZE as usize];
    // SAFETY: correctly-sized buffer; never interpret bytes outside the returned C string.
    let n = unsafe {
        libc::proc_pidpath(record.pid as i32, buffer.as_mut_ptr().cast(), buffer.len() as u32)
    };
    if n <= 0 {
        return Err(RestartError::TargetUnavailable);
    }
    let end = buffer.iter().position(|b| *b == 0).ok_or(RestartError::TargetUnavailable)?;
    if Path::new(OsStr::from_bytes(&buffer[..end])) != executable {
        return Err(RestartError::TargetUnavailable);
    }
    let (argv, process_home) = process_arguments(record.pid)?;
    if process_home.canonicalize().map_err(|_| RestartError::TargetUnavailable)? != home {
        return Err(RestartError::TargetUnavailable);
    }
    if argv.get(1).map(Vec::as_slice) != Some(b"app-server")
        || !argv.iter().any(|a| a == b"--managed-daemon")
        || argv.iter().any(|a| a == b"daemon" || a == b"proxy")
    {
        return Err(RestartError::TargetUnavailable);
    }
    if native_identity(record.pid)? != record.process_identity {
        return Err(RestartError::TargetUnavailable);
    }
    Ok(())
}
fn process_arguments(pid: u32) -> Result<(Vec<Vec<u8>>, PathBuf), RestartError> {
    let mut mib = [libc::CTL_KERN, libc::KERN_PROCARGS2, pid as i32];
    let mut bytes = vec![0u8; 256 * 1024];
    let mut len = bytes.len();
    // SAFETY: read-only sysctl with owned bounded output buffer and valid size pointer.
    if unsafe {
        libc::sysctl(
            mib.as_mut_ptr(),
            3,
            bytes.as_mut_ptr().cast(),
            &mut len,
            std::ptr::null_mut(),
            0,
        )
    } != 0
        || len < 5
        || len > bytes.len()
    {
        return Err(RestartError::TargetUnavailable);
    }
    bytes.truncate(len);
    let argc = i32::from_ne_bytes(bytes[..4].try_into().unwrap());
    if !(1..=4096).contains(&argc) {
        return Err(RestartError::TargetUnavailable);
    }
    let mut offset =
        4 + bytes[4..].iter().position(|b| *b == 0).ok_or(RestartError::TargetUnavailable)?;
    while bytes.get(offset) == Some(&0) {
        offset += 1;
    }
    let mut args = Vec::new();
    for _ in 0..argc {
        let rest = bytes.get(offset..).ok_or(RestartError::TargetUnavailable)?;
        let end = rest.iter().position(|b| *b == 0).ok_or(RestartError::TargetUnavailable)?;
        args.push(rest[..end].to_vec());
        offset += end + 1;
    }
    let mut homes =
        bytes[offset..].split(|b| *b == 0).filter_map(|entry| entry.strip_prefix(b"CODEX_HOME="));
    let home = homes.next().filter(|p| !p.is_empty()).ok_or(RestartError::TargetUnavailable)?;
    if homes.next().is_some() {
        return Err(RestartError::TargetUnavailable);
    }
    let home = PathBuf::from(OsStr::from_bytes(home));
    if !home.is_absolute() {
        return Err(RestartError::TargetUnavailable);
    }
    Ok((args, home))
}
fn run_command(
    executable: PathBuf,
    home: PathBuf,
    timeout: Duration,
) -> Result<JoinHandle<Result<RestartOutcome, RestartError>>, RestartError> {
    let child = Command::new(executable)
        .args(ARGS)
        .env_clear()
        .env("HOME", home.parent().ok_or(RestartError::TargetUnavailable)?)
        .env("CODEX_HOME", &home)
        .env("LC_ALL", "C")
        .env("PATH", "/usr/bin:/bin")
        .current_dir(&home)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .kill_on_drop(true)
        .spawn()
        .map_err(|_| RestartError::TargetFailed)?;
    Ok(tokio::spawn(wait_command(child, timeout)))
}
async fn wait_command(
    mut child: tokio::process::Child,
    timeout: Duration,
) -> Result<RestartOutcome, RestartError> {
    let result = tokio::time::timeout(timeout, async {
        let mut bytes = Vec::new();
        child
            .stdout
            .take()
            .ok_or(RestartError::TargetFailed)?
            .take((LIMIT + 1) as u64)
            .read_to_end(&mut bytes)
            .await
            .map_err(|_| RestartError::TargetFailed)?;
        if bytes.len() > LIMIT {
            return Err(RestartError::TargetFailed);
        }
        let status = child.wait().await.map_err(|_| RestartError::TargetFailed)?;
        lifecycle_result(status.success(), &bytes)
    })
    .await
    .unwrap_or(Err(RestartError::TargetTimeout));
    if result.is_err() {
        let _ = child.kill().await;
        let _ = child.wait().await;
    }
    result
}
pub(super) fn lifecycle_result(
    success: bool,
    stdout: &[u8],
) -> Result<RestartOutcome, RestartError> {
    if !success {
        return Err(RestartError::TargetFailed);
    }
    let line = std::str::from_utf8(stdout)
        .ok()
        .and_then(|s| s.lines().rev().find(|l| !l.trim().is_empty()))
        .ok_or(RestartError::TargetFailed)?;
    let value: serde_json::Value =
        serde_json::from_str(line).map_err(|_| RestartError::TargetFailed)?;
    match value.get("status").and_then(|v| v.as_str()) {
        Some("restarted") => Ok(RestartOutcome::Restarted),
        Some("notRunning") => Ok(RestartOutcome::NotRunning),
        _ => Err(RestartError::TargetFailed),
    }
}

#[cfg(test)]
#[path = "daemon_restart_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "daemon_restart_desktop_tests.rs"]
mod desktop_tests;
