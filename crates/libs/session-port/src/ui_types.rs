//! 会话 UI 类型重导出层，向旧调用点暴露共享 crate 中的会话展示模型。

/// 重导出 session_ui_types::*，保持外部调用路径稳定。
pub use session_ui_types::*;

/// Read-only projection; upload decoding must reject it as runtime input.
pub use vw_api_chat_identity::SessionAgentIdentity;

pub fn decode_ui_session(value: serde_json::Value) -> anyhow::Result<ChatSession> {
    anyhow::ensure!(value.get("agent_binding").is_none(), "agent_binding is server-owned");
    Ok(serde_json::from_value(value)?)
}
