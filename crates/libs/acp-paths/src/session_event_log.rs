//! 会话事件日志文件、分段文件与锁路径管理。

use std::path::{Path, PathBuf};

use crate::queue_paths::default_home_dir;

fn push_percent_encoded_byte(output: &mut String, byte: u8) {
    const HEX: &[u8; 16] = b"0123456789ABCDEF";
    output.push('%');
    output.push(HEX[(byte >> 4) as usize] as char);
    output.push(HEX[(byte & 0x0F) as usize] as char);
}

pub fn safe_session_id(session_id: &str) -> String {
    let mut encoded = String::with_capacity(session_id.len());
    for byte in session_id.bytes() {
        if byte.is_ascii_alphanumeric()
            || matches!(byte, b'-' | b'_' | b'.' | b'!' | b'~' | b'*' | b'\'' | b'(' | b')')
        {
            encoded.push(byte as char);
        } else {
            push_percent_encoded_byte(&mut encoded, byte);
        }
    }
    encoded
}

pub fn session_base_dir(home_dir: impl AsRef<Path>) -> PathBuf {
    config_runtime_paths::home_config_dir(home_dir).join("acp").join("sessions")
}

pub fn default_session_base_dir() -> Option<PathBuf> {
    default_home_dir().map(session_base_dir)
}

pub fn session_event_active_path(session_id: &str, home_dir: impl AsRef<Path>) -> PathBuf {
    session_base_dir(home_dir).join(format!("{}.stream.ndjson", safe_session_id(session_id)))
}

pub fn default_session_event_active_path(session_id: &str) -> Option<PathBuf> {
    default_home_dir().map(|home_dir| session_event_active_path(session_id, home_dir))
}

pub fn session_event_segment_path(
    session_id: &str,
    segment: i64,
    home_dir: impl AsRef<Path>,
) -> PathBuf {
    session_base_dir(home_dir)
        .join(format!("{}.stream.{segment}.ndjson", safe_session_id(session_id)))
}

pub fn default_session_event_segment_path(session_id: &str, segment: i64) -> Option<PathBuf> {
    default_home_dir().map(|home_dir| session_event_segment_path(session_id, segment, home_dir))
}

pub fn session_event_lock_path(session_id: &str, home_dir: impl AsRef<Path>) -> PathBuf {
    session_base_dir(home_dir).join(format!("{}.stream.lock", safe_session_id(session_id)))
}

pub fn default_session_event_lock_path(session_id: &str) -> Option<PathBuf> {
    default_home_dir().map(|home_dir| session_event_lock_path(session_id, home_dir))
}

#[cfg(test)]
#[path = "session_event_log_tests.rs"]
mod session_event_log_tests;
