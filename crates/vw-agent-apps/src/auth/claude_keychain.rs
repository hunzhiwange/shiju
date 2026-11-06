//! The only replaceable OS boundary. Secrets never enter arguments, errors or logs.
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{
    io::{Read, Write},
    path::Path,
    process::{Command, Stdio},
    time::{Duration, Instant},
};
#[derive(Clone, Copy, Debug)]
pub struct KeychainError;
pub trait ClaudeKeychain: Send + Sync {
    fn authorize_home(&self, home: &Path) -> Result<(), KeychainError>;
    fn read(&self, directory: Option<&Path>) -> Result<Option<Value>, KeychainError>;
    fn write(&self, directory: Option<&Path>, value: Option<&Value>) -> Result<(), KeychainError>;
}
pub struct MacOsKeychain;
fn service(directory: Option<&Path>) -> Result<String, KeychainError> {
    match directory {
        None => Ok("Claude Code-credentials".into()),
        Some(path) => {
            // The CLI hashes NFC text. Reject unreviewed encodings instead of addressing a wrong item.
            let text = path.to_str().filter(|p| p.is_ascii()).ok_or(KeychainError)?;
            Ok(format!(
                "Claude Code-credentials-{}",
                &Sha256::digest(text).iter().map(|b| format!("{b:02x}")).collect::<String>()[..8]
            ))
        }
    }
}
fn account() -> Result<String, KeychainError> {
    std::env::var("USER")
        .ok()
        .filter(|v| {
            !v.is_empty()
                && v.bytes().all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'-'))
        })
        .ok_or(KeychainError)
}
impl ClaudeKeychain for MacOsKeychain {
    fn authorize_home(&self, home: &Path) -> Result<(), KeychainError> {
        let actual = std::env::var_os("HOME").ok_or(KeychainError)?;
        if std::env::var_os("CLAUDE_CONFIG_DIR").is_some()
            || std::fs::canonicalize(home).map_err(|_| KeychainError)?
                != std::fs::canonicalize(actual).map_err(|_| KeychainError)?
        {
            return Err(KeychainError);
        }
        Ok(())
    }
    fn read(&self, directory: Option<&Path>) -> Result<Option<Value>, KeychainError> {
        let (code, bytes) = run(
            &["find-generic-password", "-a", &account()?, "-s", &service(directory)?, "-w"],
            None,
        )?;
        match code {
            44 => Ok(None),
            0 => {
                let text = std::str::from_utf8(&bytes).map_err(|_| KeychainError)?.trim();
                let value = serde_json::from_str(text)
                    .or_else(|_| {
                        let decoded: Option<Vec<u8>> = (text.len() % 2 == 0)
                            .then(|| {
                                (0..text.len())
                                    .step_by(2)
                                    .map(|i| {
                                        text.get(i..i + 2)
                                            .and_then(|s| u8::from_str_radix(s, 16).ok())
                                    })
                                    .collect()
                            })
                            .flatten();
                        serde_json::from_slice(decoded.as_deref().unwrap_or_default())
                    })
                    .map_err(|_| KeychainError)?;
                Ok(Some(value))
            }
            _ => Err(KeychainError),
        }
    }
    fn write(&self, directory: Option<&Path>, value: Option<&Value>) -> Result<(), KeychainError> {
        let service = service(directory)?;
        let account = account()?;
        if let Some(value) = value {
            let bytes = serde_json::to_vec(value).map_err(|_| KeychainError)?;
            let hex: String = bytes.iter().map(|b| format!("{b:02x}")).collect();
            let line =
                format!("add-generic-password -U -a \"{account}\" -s \"{service}\" -X \"{hex}\"\n");
            // Refuse overlong interactive lines: no fallback exposing secrets through ps.
            if line.len() > 4032 {
                return Err(KeychainError);
            }
            if run(&["-i"], Some(line.into_bytes()))?.0 != 0 {
                return Err(KeychainError);
            }
            if self.read(directory)?.as_ref() != Some(value) {
                return Err(KeychainError);
            }
        } else if !matches!(
            run(&["delete-generic-password", "-a", &account, "-s", &service], None)?.0,
            0 | 44
        ) {
            return Err(KeychainError);
        }
        Ok(())
    }
}
fn run(args: &[&str], input: Option<Vec<u8>>) -> Result<(i32, Vec<u8>), KeychainError> {
    let mut child = Command::new("/usr/bin/security")
        .args(args)
        .stdin(if input.is_some() { Stdio::piped() } else { Stdio::null() })
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|_| KeychainError)?;
    let stdin = child.stdin.take();
    let stdout = child.stdout.take().ok_or(KeychainError)?;
    let writer = std::thread::spawn(move || -> Result<(), KeychainError> {
        if let (Some(mut pipe), Some(input)) = (stdin, input) {
            pipe.write_all(&input).map_err(|_| KeychainError)?;
        }
        Ok(())
    });
    let reader = std::thread::spawn(move || {
        let mut bytes = Vec::new();
        stdout.take(100001).read_to_end(&mut bytes).map(|_| bytes)
    });
    let started = Instant::now();
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break Ok(status),
            Ok(None) if started.elapsed() < Duration::from_secs(5) => {
                std::thread::sleep(Duration::from_millis(10))
            }
            _ => {
                let _ = child.kill();
                let _ = child.wait();
                break Err(KeychainError);
            }
        }
    };
    let written = writer.join().map_err(|_| KeychainError)?;
    let output = reader.join().map_err(|_| KeychainError)?.map_err(|_| KeychainError)?;
    written?;
    let code = status?.code().ok_or(KeychainError)?;
    if output.len() > 100000 {
        return Err(KeychainError);
    }
    Ok((code, output))
}
