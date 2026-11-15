//! 会话事件日志的类型相关默认值。

use std::path::Path;

use acp_paths::queue_paths::default_home_dir;
use crate::types::SessionEventLog;

pub use acp_paths::session_event_log::*;

pub const DEFAULT_EVENT_SEGMENT_MAX_BYTES: i64 = 64 * 1024 * 1024;
pub const DEFAULT_EVENT_MAX_SEGMENTS: i64 = 5;

pub fn session_event_log(session_id: &str, home_dir: impl AsRef<Path>) -> SessionEventLog {
    SessionEventLog {
        active_path: session_event_active_path(session_id, home_dir).to_string_lossy().into_owned(),
        segment_count: DEFAULT_EVENT_MAX_SEGMENTS,
        max_segment_bytes: DEFAULT_EVENT_SEGMENT_MAX_BYTES,
        max_segments: DEFAULT_EVENT_MAX_SEGMENTS,
        last_write_at: None,
        last_write_error: None,
    }
}

pub fn default_session_event_log(session_id: &str) -> Option<SessionEventLog> {
    default_home_dir().map(|home_dir| session_event_log(session_id, home_dir))
}

#[cfg(test)]
#[path = "session_event_log_tests.rs"]
mod session_event_log_tests;
