use iced::widget::text_editor::Content;
use vw_gateway_client::vw_api_types::{agent_apps::AppId, app_providers::snippets::ConfigSnippet};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Operation {
    Read,
    Extract,
    Preview,
    Save,
}

#[derive(Clone, Default)]
pub(crate) struct Form {
    pub draft: Content,
    pub loaded: bool,
    pub pending: Option<Operation>,
    pub error: Option<String>,
    pub provider_id: Option<String>,
    pub preview: Option<(Operation, ConfigSnippet)>,
}

impl std::fmt::Debug for Form {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("SnippetForm([redacted])")
    }
}

pub(crate) fn available(app: AppId) -> bool {
    matches!(app, AppId::Claude | AppId::Codex | AppId::Gemini | AppId::OpenCode | AppId::OpenClaw)
}
