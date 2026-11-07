use iced::widget::text_editor::Content;
use vw_gateway_client::vw_api_types::app_providers::openclaw::OpenClawFormConfig;

#[derive(Clone)]
pub(crate) struct Draft {
    pub declaration: Content,
    pub user_agent: bool,
    pub has_saved_key: bool,
    pub revision: Option<String>,
    pub apply: bool,
}

impl Draft {
    pub fn new() -> Self {
        Self {
            declaration: Content::with_text("{\n  \"models\": []\n}"),
            user_agent: false,
            has_saved_key: false,
            revision: None,
            apply: false,
        }
    }

    pub fn from_view(view: OpenClawFormConfig, member: bool) -> Self {
        Self {
            declaration: Content::with_text(&view.declaration),
            user_agent: view.user_agent,
            has_saved_key: view.has_saved_key,
            revision: Some(view.revision),
            apply: member,
        }
    }
}
