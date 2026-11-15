//! 运行时会话标识提取的轻量包装层。

use serde_json::Value;

use crate::agent_session_id::{
    AGENT_SESSION_ID_META_KEYS, extract_agent_session_id, normalize_agent_session_id,
};

pub const RUNTIME_SESSION_ID_META_KEYS: &[&str] = AGENT_SESSION_ID_META_KEYS;

pub fn normalize_runtime_session_id(value: &Value) -> Option<String> {
    normalize_agent_session_id(value)
}

pub fn extract_runtime_session_id(meta: &Value) -> Option<String> {
    extract_agent_session_id(meta)
}

#[cfg(test)]
#[path = "runtime_session_id_tests.rs"]
mod runtime_session_id_tests;
