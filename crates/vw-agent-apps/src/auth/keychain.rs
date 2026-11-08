//! Generic password entries on the authorized macOS execution host.

use std::{
    io::{self, Read, Write},
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant},
};

const SECURITY: &str = "/usr/bin/security";
const NOT_FOUND: i32 = 44;
const TIMEOUT: Duration = Duration::from_secs(5);
const MAX_LINE: usize = 4032;
const MAX_OUTPUT: u64 = 100_000;

#[derive(Debug, PartialEq, Eq)]
pub(super) enum EntryError {
    InvalidIdentity,
    InvalidPayload,
    InvalidKeychainPath,
    ProcessFailed,
    TimedOut,
    CommandFailed { operation: &'static str, code: i32 },
    WriteRejected,
    VerificationFailed,
}

/// A service and account chosen by the trusted host-side caller.
pub(super) struct KeychainEntry<'a> {
    service: &'a str,
    account: &'a str,
}

impl<'a> KeychainEntry<'a> {
    pub(super) fn new(service: &'a str, account: &'a str) -> Result<Self, EntryError> {
        if [service, account].into_iter().any(|name| {
            name.is_empty()
                || !name
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || matches!(b, b' ' | b'-' | b'_' | b'.'))
        }) {
            return Err(EntryError::InvalidIdentity);
        }
        Ok(Self { service, account })
    }

    /// Returns the password as printed by `security -w`, which hex-encodes nonprintable data.
    pub(super) fn read(&self) -> Result<Option<Vec<u8>>, EntryError> {
        self.read_in(None)
    }

    fn read_in(&self, path: Option<&str>) -> Result<Option<Vec<u8>>, EntryError> {
        let mut args = vec!["find-generic-password", "-a", self.account, "-s", self.service, "-w"];
        args.extend(path);
        let output = run(&args, None, true)?;
        match output.code {
            NOT_FOUND => Ok(None),
            0 => {
                let mut password = output.stdout;
                if password.pop() != Some(b'\n') {
                    return Err(EntryError::ProcessFailed);
                }
                Ok(Some(password))
            }
            code => Err(EntryError::CommandFailed { operation: "read", code }),
        }
    }

    pub(super) fn write(&self, payload: &str) -> Result<(), EntryError> {
        // The interactive command has a fixed-size buffer. Keep the secret off argv.
        if payload.is_empty() || payload.len() > MAX_LINE / 2 {
            return Err(EntryError::InvalidPayload);
        }
        let (path, existing) = self.write_target()?;
        let update = if existing { " -U" } else { "" };
        let prefix = format!(
            "add-generic-password{update} -a \"{}\" -s \"{}\" -X \"",
            self.account, self.service
        );
        let suffix = format!("\" \"{path}\"\n");
        if prefix.len() + payload.len() * 2 + suffix.len() > MAX_LINE {
            return Err(EntryError::InvalidPayload);
        }
        if existing && self.read_in(Some(&path))?.is_none() {
            return Err(EntryError::VerificationFailed);
        }
        let hex: String = payload.bytes().map(|byte| format!("{byte:02x}")).collect();
        let line = format!("{prefix}{hex}{suffix}");
        let output = run(&["-q", "-i"], Some(line.as_bytes()), false)?;
        if output.code != 0 {
            return Err(EntryError::CommandFailed { operation: "write", code: output.code });
        }
        // The interactive shell can exit successfully even if the inner command fails.
        if output.has_diagnostic {
            return Err(EntryError::WriteRejected);
        }
        let expected = if payload.bytes().all(|b| b.is_ascii_graphic() || b == b' ') {
            payload.as_bytes()
        } else {
            hex.as_bytes()
        };
        if self.read_in(Some(&path))?.as_deref() != Some(expected) {
            return Err(EntryError::VerificationFailed);
        }
        Ok(())
    }

    fn write_target(&self) -> Result<(String, bool), EntryError> {
        let found =
            run(&["find-generic-password", "-a", self.account, "-s", self.service], None, true)?;
        // With an implicit keychain, -U can try to add in the default keychain
        // after an update fails in another keychain. Pin the write to one location.
        let (output, prefix, existing) = match found.code {
            0 => (found, "keychain: ", true),
            NOT_FOUND => (run(&["default-keychain"], None, true)?, "    ", false),
            code => return Err(EntryError::CommandFailed { operation: "locate", code }),
        };
        if output.code != 0 {
            return Err(EntryError::CommandFailed { operation: "locate", code: output.code });
        }
        let path = std::str::from_utf8(&output.stdout)
            .ok()
            .and_then(|text| text.lines().next())
            .and_then(|line| line.strip_prefix(prefix))
            .and_then(|line| line.strip_prefix('"'))
            .and_then(|line| line.strip_suffix('"'))
            .filter(|path| {
                path.starts_with('/')
                    && path
                        .bytes()
                        .all(|b| b == b' ' || (b.is_ascii_graphic() && !matches!(b, b'"' | b'\\')))
            })
            .ok_or(EntryError::InvalidKeychainPath)?;
        Ok((path.to_owned(), existing))
    }

    pub(super) fn delete(&self) -> Result<(), EntryError> {
        let output =
            run(&["delete-generic-password", "-a", self.account, "-s", self.service], None, false)?;
        match output.code {
            0 | NOT_FOUND => Ok(()),
            code => Err(EntryError::CommandFailed { operation: "delete", code }),
        }
    }
}

struct Output {
    code: i32,
    stdout: Vec<u8>,
    has_diagnostic: bool,
}

fn run(args: &[&str], input: Option<&[u8]>, read_stdout: bool) -> Result<Output, EntryError> {
    let mut child = Command::new(SECURITY)
        .args(args)
        .stdin(if input.is_some() { Stdio::piped() } else { Stdio::null() })
        .stdout(if read_stdout { Stdio::piped() } else { Stdio::null() })
        .stderr(if input.is_some() { Stdio::piped() } else { Stdio::null() })
        .spawn()
        .map_err(|_| EntryError::ProcessFailed)?;
    let stdin = child.stdin.take();
    let stdout = child.stdout.take();
    let stderr = child.stderr.take();

    thread::scope(|scope| {
        let writer = scope.spawn(move || {
            if let (Some(mut pipe), Some(input)) = (stdin, input) {
                pipe.write_all(input)?;
            }
            Ok::<_, io::Error>(())
        });
        let reader = scope.spawn(move || {
            let mut bytes = Vec::new();
            if let Some(pipe) = stdout {
                pipe.take(MAX_OUTPUT + 1).read_to_end(&mut bytes)?;
            }
            Ok::<_, io::Error>(bytes)
        });
        let diagnostics = scope.spawn(move || match stderr {
            Some(mut pipe) => io::copy(&mut pipe, &mut io::sink()).map(|size| size != 0),
            None => Ok(false),
        });

        let started = Instant::now();
        let status = loop {
            let failure = match child.try_wait() {
                Ok(Some(status)) => break status,
                Ok(None) if started.elapsed() < TIMEOUT => {
                    thread::sleep(Duration::from_millis(10));
                    continue;
                }
                Ok(None) => EntryError::TimedOut,
                Err(_) => EntryError::ProcessFailed,
            };
            let _ = child.kill();
            let _ = child.wait();
            return Err(failure);
        };
        writer
            .join()
            .map_err(|_| EntryError::ProcessFailed)?
            .map_err(|_| EntryError::ProcessFailed)?;
        let stdout = reader
            .join()
            .map_err(|_| EntryError::ProcessFailed)?
            .map_err(|_| EntryError::ProcessFailed)?;
        let has_diagnostic = diagnostics
            .join()
            .map_err(|_| EntryError::ProcessFailed)?
            .map_err(|_| EntryError::ProcessFailed)?;
        if stdout.len() as u64 > MAX_OUTPUT {
            return Err(EntryError::ProcessFailed);
        }
        let code = status.code().ok_or(EntryError::ProcessFailed)?;
        Ok(Output { code, stdout, has_diagnostic })
    })
}
