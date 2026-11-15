use iced::widget::text_editor::Content;
use vw_api_app_provider_terminal::pi::PiFormConfig;

#[derive(Clone)]
pub struct Draft {
    pub declaration: Content,
    pub has_saved_key: bool,
    pub revision: Option<String>,
    pub apply: bool,
}

impl Draft {
    pub fn new() -> Self {
        Self {
            declaration: Content::with_text(
                "{\n  \"baseUrl\": \"\",\n  \"api\": \"openai-completions\",\n  \"models\": []\n}",
            ),
            has_saved_key: false,
            revision: None,
            apply: false,
        }
    }

    pub fn from_view(view: PiFormConfig, member: bool) -> Self {
        Self {
            declaration: Content::with_text(&view.declaration),
            has_saved_key: view.has_saved_key,
            revision: Some(view.revision),
            apply: member,
        }
    }
}
