use iced::widget::text_editor::Content;
use vw_gateway_client::vw_api_types::app_providers::opencode::{
    OpenCodeFormConfig, OpenCodeFormat,
};

#[derive(Clone)]
pub(crate) struct Draft {
    pub declaration: Content,
    pub format: OpenCodeFormat,
    pub has_saved_key: bool,
    pub revision: Option<String>,
    pub apply: bool,
}

impl Draft {
    pub fn new() -> Self {
        Self {
            declaration: Content::with_text("{\n  \"models\": {}\n}"),
            format: OpenCodeFormat::V1,
            has_saved_key: false,
            revision: None,
            apply: false,
        }
    }

    pub fn from_view(view: OpenCodeFormConfig, member: bool) -> Self {
        Self {
            declaration: Content::with_text(&view.declaration),
            format: view.format,
            has_saved_key: view.has_saved_key,
            revision: Some(view.revision),
            apply: member,
        }
    }
}
