use iced::widget::text_editor::Content;
use vw_gateway_client::vw_api_types::app_providers::{
    basic::ProviderBasic,
    codex_catalog::{CodexFormConfig, CodexFormWrite, CodexModelMapping},
    create::ProviderSecretWrite,
};

pub(crate) struct Draft {
    pub toml: Content,
    pub has_saved_key: bool,
    pub inactive: Vec<Vec<String>>,
    pub models: Option<Vec<CodexModelMapping>>,
    pub apply: bool,
}

impl Clone for Draft {
    fn clone(&self) -> Self {
        Self {
            toml: Content::with_text(&self.toml.text()),
            has_saved_key: self.has_saved_key,
            inactive: self.inactive.clone(),
            models: self.models.clone(),
            apply: self.apply,
        }
    }
}

impl Draft {
    pub fn new() -> Self {
        Self::from_view(CodexFormConfig {
            toml: concat!(
                "model_provider = \"vendor\"\n",
                "model = \"\"\n\n",
                "[model_providers.vendor]\n",
                "base_url = \"\"\n",
                "wire_api = \"responses\"\n",
            )
            .into(),
            has_saved_key: false,
            inactive: Vec::new(),
            models: Some(Vec::new()),
        })
    }

    pub fn from_view(config: CodexFormConfig) -> Self {
        Self {
            toml: Content::with_text(&config.toml),
            has_saved_key: config.has_saved_key,
            inactive: config.inactive,
            models: config.models,
            apply: false,
        }
    }

    pub fn request(
        &self,
        host: String,
        fields: ProviderBasic,
        api_key: ProviderSecretWrite,
    ) -> CodexFormWrite {
        CodexFormWrite {
            target_host_id: host,
            fields,
            toml: self.toml.text(),
            api_key,
            models: self.models.clone(),
            apply: self.apply,
        }
    }
}
