use vw_gateway_client::vw_api_types::app_providers::{
    basic::ProviderBasic,
    claude::{ClaudeFormConfig, ClaudeFormKind, ClaudeFormWrite},
    create::ProviderSecretWrite,
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
    pub api_key_header: bool,
    pub model: String,
    pub apply: bool,
    pub has_saved_key: bool,
}

impl Draft {
    pub fn new() -> Self {
        Self::from_view(ClaudeFormKind::Official {}, None)
    }

    pub fn from_view(config: ClaudeFormKind, model: Option<String>) -> Self {
        let (kind, base_url, api_key_header) = match config {
            ClaudeFormKind::Official {} => (Kind::Official, String::new(), false),
            ClaudeFormKind::ThirdParty { base_url, api_key_header } => {
                (Kind::ThirdParty, base_url, api_key_header)
            }
            ClaudeFormKind::Unsupported {} => (Kind::Unsupported, String::new(), false),
        };
        Self {
            kind,
            base_url,
            api_key_header,
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
    ) -> Option<ClaudeFormWrite> {
        let config = match self.kind {
            Kind::Official => ClaudeFormConfig::Official {},
            Kind::ThirdParty => ClaudeFormConfig::ThirdParty {
                base_url: self.base_url.clone(),
                api_key_header: self.api_key_header,
                api_key: if replace_key {
                    ProviderSecretWrite::Replace { value: key }
                } else {
                    ProviderSecretWrite::Keep
                },
            },
            Kind::Unsupported => return None,
        };
        Some(ClaudeFormWrite {
            target_host_id,
            fields,
            config,
            model: (!self.model.is_empty()).then(|| self.model.clone()),
            apply: self.apply,
        })
    }
}
