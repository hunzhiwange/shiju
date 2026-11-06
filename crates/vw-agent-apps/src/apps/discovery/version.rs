//! Local execution-host version probes, never client command execution.
use std::{
    io::{self, Read},
    os::{fd::AsRawFd, unix::process::CommandExt},
    path::Path,
    process::{Child, Command, Stdio},
    time::{Duration, Instant},
};
use vw_api_types::agent_apps::{
    AppInstallationProbeFailure as Failure, AppInstallationRuntimeStatus as Status,
};

const OUTPUT_LIMIT: usize = 16 * 1024;

#[derive(Debug)]
pub struct VersionProbe {
    pub installed: bool,
    pub version: Option<String>,
    pub runtime_status: Status,
}

impl VersionProbe {
    fn failed(installed: bool, reason: Failure) -> Self {
        Self { installed, version: None, runtime_status: Status::ProbeFailed { reason } }
    }
}

struct ProbeChild(Child);
impl Drop for ProbeChild {
    fn drop(&mut self) {
        // SAFETY: this child owns the fresh process group created below.
        unsafe { libc::kill(-(self.0.id() as i32), libc::SIGKILL) };
        let _ = self.0.wait();
    }
}

/// Runs fixed `--version` on an authorized host's discovered absolute path.
/// This is a blocking POSIX host operation; call outside an async runtime worker.
/// Stderr is discarded, since diagnostic payloads can contain secrets.
/// Windows and WSL execution must use their own authorized host adapters.
pub fn probe_local_version(path: &Path, timeout: Duration) -> io::Result<VersionProbe> {
    if !path.is_absolute() || timeout.is_zero() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "absolute host path and nonzero timeout required",
        ));
    }
    let started = Instant::now();
    let installed = match path.symlink_metadata() {
        Ok(_) => true,
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            return Ok(VersionProbe::failed(false, Failure::Missing));
        }
        Err(error) if error.kind() == io::ErrorKind::PermissionDenied => {
            return Ok(VersionProbe::failed(true, Failure::PermissionDenied));
        }
        Err(error) => return Err(error),
    };
    let child = Command::new(path)
        .arg("--version")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .process_group(0)
        .spawn();
    let mut child = match child {
        Ok(child) => ProbeChild(child),
        Err(error) => {
            let reason = match error.kind() {
                io::ErrorKind::PermissionDenied => Failure::PermissionDenied,
                io::ErrorKind::NotFound => Failure::DamagedInstallation,
                _ if error.raw_os_error() == Some(libc::ENOEXEC) => Failure::DamagedInstallation,
                _ => Failure::SpawnFailed,
            };
            return Ok(VersionProbe::failed(installed, reason));
        }
    };
    let mut stdout = child.0.stdout.take().expect("piped stdout");
    // SAFETY: stdout owns this live descriptor; only O_NONBLOCK is added.
    let flags = unsafe { libc::fcntl(stdout.as_raw_fd(), libc::F_GETFL) };
    if flags < 0
        || unsafe { libc::fcntl(stdout.as_raw_fd(), libc::F_SETFL, flags | libc::O_NONBLOCK) } < 0
    {
        return Err(io::Error::last_os_error());
    }
    let mut bytes = Vec::new();
    let mut exited = None;
    loop {
        if started.elapsed() >= timeout {
            return Ok(VersionProbe::failed(installed, Failure::TimedOut));
        }
        let mut buffer = [0; 4096];
        let eof = match stdout.read(&mut buffer) {
            Ok(0) => true,
            Ok(count) => {
                bytes.extend_from_slice(&buffer[..count]);
                if bytes.len() > OUTPUT_LIMIT {
                    return Ok(VersionProbe::failed(installed, Failure::OutputTooLarge));
                }
                false
            }
            Err(error) if error.kind() == io::ErrorKind::WouldBlock => false,
            Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
            Err(error) => return Err(error),
        };
        if exited.is_none() {
            exited = child.0.try_wait()?;
        }
        if let Some(status) = exited {
            if !status.success() {
                return Ok(match status.code() {
                    Some(exit_code) => VersionProbe {
                        installed,
                        version: None,
                        runtime_status: Status::ExitedWithError { exit_code },
                    },
                    None => VersionProbe::failed(installed, Failure::DamagedInstallation),
                });
            }
            if eof {
                let version = std::str::from_utf8(&bytes).ok().and_then(parse_version);
                return Ok(match version {
                    Some(version) => VersionProbe {
                        installed,
                        version: Some(version),
                        runtime_status: Status::Runnable,
                    },
                    None => VersionProbe::failed(installed, Failure::InvalidVersionOutput),
                });
            }
        }
        std::thread::sleep(Duration::from_millis(2));
    }
}

fn parse_version(output: &str) -> Option<String> {
    output.split_whitespace().find_map(|word| {
        let word = word.trim_matches(|c: char| matches!(c, '(' | ')' | ','));
        let word = word.strip_prefix("omp/").unwrap_or(word);
        let version = word.strip_prefix('v').unwrap_or(word);
        let core = version.split(['-', '+']).next()?;
        let parts: Vec<_> = core.split('.').collect();
        (parts.len() >= 2
            && parts
                .iter()
                .all(|part| !part.is_empty() && part.bytes().all(|b| b.is_ascii_digit()))
            && version.bytes().all(|b| b.is_ascii_alphanumeric() || b".-+".contains(&b)))
        .then(|| version.to_owned())
    })
}
