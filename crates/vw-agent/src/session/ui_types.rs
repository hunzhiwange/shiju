//! 会话 UI 类型重导出层，向旧调用点暴露共享 crate 中的会话展示模型。

/// 重导出 vw_shared::session::ui_types::*，保持外部调用路径稳定。
pub use vw_shared::session::ui_types::*;

/// An ephemeral, source-bound MCP form. It is never a model tool or persisted message.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct McpFormRequest {
    pub id: String,
    pub session_id: String,
    pub server_name: String,
    pub connection_id: String,
    pub server_request_id: serde_json::Value,
    pub message: String,
    pub requested_schema: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta: Option<serde_json::Value>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum McpFormAction {
    Accept,
    Decline,
    Cancel,
}

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct McpFormResponse {
    pub action: McpFormAction,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<serde_json::Value>,
}

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum McpFormEvent {
    Requested { request: McpFormRequest },
    Finished { request_id: String },
}
