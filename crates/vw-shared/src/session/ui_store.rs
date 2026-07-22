use std::path::{Path, PathBuf};

#[cfg(not(target_arch = "wasm32"))]
use std::io;

#[cfg(target_arch = "wasm32")]
use crate::session::info::Info;
pub use crate::session::ui_types::{
    ChatMessage, ChatRole, ChatSession, ChatSessionMeta, ChatSessionStep, SessionTodoItem,
    ThinkTiming, TokenUsage,
};

use super::path;

#[cfg(not(target_arch = "wasm32"))]
fn json_to_io_error(error: serde_json::Error) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, error.to_string())
}

/// 保存某个步骤的会话快照文件。
pub fn save_session_step_snapshot(
    data_dir: &Path,
    session: &ChatSession,
    step_index: u32,
    kind: &str,
    scope: Option<&str>,
) -> Option<PathBuf> {
    let path =
        path::session_step_snapshot_file_path(data_dir, &session.id, step_index, kind, scope)?;
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    let content = serde_json::to_string_pretty(session).ok()?;
    let _ = std::fs::write(&path, content);
    Some(path)
}

/// 持久化某一步的原始 LLM 输出。
pub fn persist_llm_raw_step(
    data_dir: &Path,
    session_id: &str,
    step_index: u32,
    payload: &serde_json::Value,
    scope: Option<&str>,
) -> Option<PathBuf> {
    let path =
        path::session_step_llm_raw_file_path_scoped(data_dir, session_id, step_index, scope)?;
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    let content = serde_json::to_string_pretty(payload).ok()?;
    let _ = std::fs::write(&path, content);
    Some(path)
}

/// 读取某一步的原始 LLM 输出。
#[cfg(not(target_arch = "wasm32"))]
pub fn load_llm_raw_step(
    data_dir: &Path,
    session_id: &str,
    step_index: u32,
    scope: Option<&str>,
) -> io::Result<Option<serde_json::Value>> {
    let Some(path) =
        path::session_step_llm_raw_file_path_scoped(data_dir, session_id, step_index, scope)
    else {
        return Ok(None);
    };
    if !path.is_file() {
        return Ok(None);
    }
    let content = std::fs::read_to_string(path)?;
    serde_json::from_str(&content).map(Some).map_err(json_to_io_error)
}

#[cfg(target_arch = "wasm32")]
pub fn load_session_todos_scoped(
    _data_dir: &Path,
    _session_id: &str,
    _scope: Option<&str>,
) -> Vec<SessionTodoItem> {
    Vec::new()
}

#[cfg(target_arch = "wasm32")]
pub fn save_session_todos_scoped(
    _data_dir: &Path,
    _session_id: &str,
    _todos: &[SessionTodoItem],
    _scope: Option<&str>,
) -> Option<PathBuf> {
    None
}

#[cfg(target_arch = "wasm32")]
pub fn load_sessions_scoped(_data_dir: &Path, _scope: Option<&str>) -> Vec<ChatSessionMeta> {
    Vec::new()
}

#[cfg(target_arch = "wasm32")]
pub fn save_sessions_scoped(_data_dir: &Path, _sessions: &[ChatSessionMeta], _scope: Option<&str>) {
}

/// 在 WASM 场景下读取指定 scope 的会话 JSON。
#[cfg(target_arch = "wasm32")]
pub fn load_session_scoped(data_dir: &Path, id: &str, scope: Option<&str>) -> Option<ChatSession> {
    let path = path::session_file_path(data_dir, id, scope)?;
    let content = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&content).ok()
}

#[cfg(target_arch = "wasm32")]
pub fn load_session_any(data_dir: &Path, id: &str, scope: Option<&str>) -> Option<ChatSession> {
    load_session_scoped(data_dir, id, scope)
}

/// 在 WASM 场景下保存指定 scope 的会话 JSON。
#[cfg(target_arch = "wasm32")]
pub fn save_session_scoped(
    data_dir: &Path,
    session: &ChatSession,
    scope: Option<&str>,
) -> Option<PathBuf> {
    let path = path::session_file_path(data_dir, &session.id, scope)?;
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let content = serde_json::to_string_pretty(session).ok()?;
    std::fs::write(&path, content).ok()?;
    Some(path)
}

#[cfg(target_arch = "wasm32")]
pub fn delete_session_scoped(data_dir: &Path, id: &str, scope: Option<&str>) {
    let Some(path) = path::session_file_path(data_dir, id, scope) else {
        return;
    };
    let _ = std::fs::remove_file(path);
}

#[cfg(target_arch = "wasm32")]
pub fn load_agent_session_scoped(
    _data_dir: &Path,
    _session_id: &str,
    _scope: Option<&str>,
) -> Option<Info> {
    None
}

#[cfg(target_arch = "wasm32")]
pub fn load_agent_sessions_scoped(_data_dir: &Path, _scope: Option<&str>) -> Vec<Info> {
    Vec::new()
}

#[cfg(target_arch = "wasm32")]
pub fn save_agent_session_scoped(
    _data_dir: &Path,
    _info: &Info,
    _scope: Option<&str>,
) -> Option<PathBuf> {
    None
}

#[cfg(target_arch = "wasm32")]
pub fn delete_agent_session_scoped(_data_dir: &Path, _id: &str, _scope: Option<&str>) {}

#[cfg(target_arch = "wasm32")]
pub fn load_agent_sessions_all(_data_dir: &Path, _scope: Option<&str>) -> Vec<Info> {
    Vec::new()
}

#[cfg(target_arch = "wasm32")]
pub fn load_agent_session_any(
    _data_dir: &Path,
    _session_id: &str,
    _scope: Option<&str>,
) -> Option<Info> {
    None
}

#[cfg(target_arch = "wasm32")]
pub fn session_preview_meta(
    data_dir: &Path,
    id: &str,
    scope: Option<&str>,
) -> Option<ChatSessionMeta> {
    load_sessions_scoped(data_dir, scope).into_iter().find(|meta| meta.id == id)
}

#[cfg(test)]
#[path = "ui_store_tests.rs"]
mod ui_store_tests;
