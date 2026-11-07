use iced::widget::text_editor::Content;
use vw_gateway_client::vw_api_types::app_providers::grok::GrokFormConfig;

#[derive(Clone)]
pub(crate) struct Draft {
    pub toml: Content,
    pub has_saved_key: bool,
    pub inactive: Vec<Vec<String>>,
    pub apply: bool,
}
impl Draft {
    pub fn new() -> Self {
        Self {
            toml: Content::with_text(
                "[models]\ndefault = \"vendor\"\n\n[model.vendor]\nmodel = \"\"\nname = \"\"\nbase_url = \"\"\napi_backend = \"responses\"\ncontext_window = \n",
            ),
            has_saved_key: false,
            inactive: Vec::new(),
            apply: false,
        }
    }

    pub fn from_view(view: GrokFormConfig) -> Self {
        Self {
            toml: Content::with_text(&view.toml),
            has_saved_key: view.has_saved_key,
            inactive: view.inactive,
            apply: false,
        }
    }
}
