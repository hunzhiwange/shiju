use vw_gateway_client::vw_api_types::app_providers::{
    basic::ProviderBasic,
    create::ProviderSecretWrite,
    gemini::{GeminiFormConfig, GeminiFormKind, GeminiFormWrite},
};

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Kind {
    Official,
    ThirdParty,
    Unsupported,
}

#[derive(Clone)]
pub(crate) struct Draft {
    pub kind: Kind,
    pub base_url: String,
    pub model: String,
    pub apply: bool,
    pub has_saved_key: bool,
}

impl Draft {
    pub fn new() -> Self {
        Self::from_view(GeminiFormKind::Official {}, None)
    }

    pub fn from_view(config: GeminiFormKind, model: Option<String>) -> Self {
        let (kind, base_url) = match config {
            GeminiFormKind::Official {} => (Kind::Official, String::new()),
            GeminiFormKind::ThirdParty { base_url } => (Kind::ThirdParty, base_url),
            GeminiFormKind::Unsupported {} => (Kind::Unsupported, String::new()),
        };
        Self {
            kind,
            base_url,
            model: model.unwrap_or_default(),
            apply: false,
            has_saved_key: kind == Kind::ThirdParty,
        }
    }

    pub fn request(
        &self,
        target_host_id: String,
        fields: ProviderBasic,
        key: String,
        replace_key: bool,
    ) -> Option<GeminiFormWrite> {
        let config = match self.kind {
            Kind::Official => GeminiFormConfig::Official {},
            Kind::ThirdParty => GeminiFormConfig::ThirdParty {
                base_url: self.base_url.clone(),
                api_key: if replace_key {
                    ProviderSecretWrite::Replace { value: key }
                } else {
                    ProviderSecretWrite::Keep
                },
            },
            Kind::Unsupported => return None,
        };
        Some(GeminiFormWrite {
            target_host_id,
            fields,
            config,
            model: (!self.model.is_empty()).then(|| self.model.clone()),
            apply: self.apply,
        })
    }
}
