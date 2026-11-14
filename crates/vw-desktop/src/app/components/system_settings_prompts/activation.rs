//! Mutations use the server revision and exact native snapshot, including undo.
use vw_gateway_client::{AgentAppPromptEntry, AgentAppPromptsError};

#[derive(Clone, Debug)]
pub enum Action {
    Activate(String),
    Deactivate(String),
    Delete(String),
    Restore(AgentAppPromptEntry),
}

#[derive(Clone, Debug)]
pub(crate) struct Undo {
    pub revision: u64,
    pub content: Option<String>,
    pub action: Action,
}

pub(crate) fn error_text(error: AgentAppPromptsError) -> String {
    match error {
        AgentAppPromptsError::Http { status: 409, body } if body.to_string().contains("active_prompt") =>
            "启用中的条目不能删除，请先停用。".into(),
        AgentAppPromptsError::Http { status: 409, .. } =>
            "提示词或目标文件已变化，操作冲突；请刷新后重试。".into(),
        AgentAppPromptsError::Http { status: 400 | 422, .. } =>
            "服务端拒绝此操作；启用中的条目须先停用才能删除。".into(),
        other => other.to_string(),
    }
}

pub(crate) fn effect(app: vw_gateway_client::vw_api_types::agent_apps::AppId) -> &'static str {
    use vw_gateway_client::vw_api_types::agent_apps::AppId;
    match app {
        AppId::Pi => "操作已保存，请在 Pi 执行 /reload。",
        AppId::Hermes => "操作已保存，Hermes 提示词将在新会话中生效。",
        _ => "提示词操作已保存。",
    }
}
