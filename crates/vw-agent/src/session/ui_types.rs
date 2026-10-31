//! 会话 UI 类型重导出层，向旧调用点暴露共享 crate 中的会话展示模型。

/// 重导出 vw_shared::session::ui_types::*，保持外部调用路径稳定。
pub use vw_shared::session::ui_types::*;

pub use vw_api_types::mcp_form::{McpFormAction, McpFormRequest, McpFormResponse};

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum McpFormEvent {
    Requested { request: McpFormRequest },
    Finished { request_id: String },
}
