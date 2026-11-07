use iced::widget::text_editor::Content;
use vw_gateway_client::vw_api_types::app_providers::kimi::KimiFormConfig;

#[derive(Clone)]
pub(crate) struct Draft {
    pub declaration: Content,
    pub has_saved_key: bool,
    pub revision: Option<String>,
    pub apply: bool,
}

impl Draft {
    pub fn new() -> Self {
        Self {
            declaration: Content::with_text("[provider]\ntype = 'openai'\nbase_url = ''\n[models]"),
            has_saved_key: false,
            revision: None,
            apply: false,
        }
    }

    pub fn from_view(view: KimiFormConfig, member: bool) -> Self {
        Self {
            declaration: Content::with_text(&view.declaration),
            has_saved_key: view.has_saved_key,
            revision: Some(view.revision),
            apply: member,
        }
    }
}
