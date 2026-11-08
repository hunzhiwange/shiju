//! The only replaceable OS boundary. Secrets never enter arguments, errors or logs.
use super::keychain::KeychainEntry;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::path::Path;

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
        let service = service(directory)?;
        let account = account()?;
        let entry = KeychainEntry::new(&service, &account).map_err(|_| KeychainError)?;
        let Some(bytes) = entry.read().map_err(|_| KeychainError)? else {
            return Ok(None);
        };
        let text = std::str::from_utf8(&bytes).map_err(|_| KeychainError)?.trim();
        let value = serde_json::from_str(text)
            .or_else(|_| {
                let decoded: Option<Vec<u8>> = (text.len() % 2 == 0)
                    .then(|| {
                        (0..text.len())
                            .step_by(2)
                            .map(|i| {
                                text.get(i..i + 2).and_then(|s| u8::from_str_radix(s, 16).ok())
                            })
                            .collect()
                    })
                    .flatten();
                serde_json::from_slice(decoded.as_deref().unwrap_or_default())
            })
            .map_err(|_| KeychainError)?;
        Ok(Some(value))
    }

    fn write(&self, directory: Option<&Path>, value: Option<&Value>) -> Result<(), KeychainError> {
        let service = service(directory)?;
        let account = account()?;
        let entry = KeychainEntry::new(&service, &account).map_err(|_| KeychainError)?;
        match value {
            Some(value) => {
                let payload = serde_json::to_string(value).map_err(|_| KeychainError)?;
                entry.write(&payload).map_err(|_| KeychainError)
            }
            None => entry.delete().map_err(|_| KeychainError),
        }
    }
}
