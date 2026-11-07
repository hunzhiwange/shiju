use super::claude::{Draft, Kind};
use vw_gateway_client::vw_api_types::app_providers::{
    basic::*,
    create::{AppProviderCreate, ProviderSecretWrite},
};
#[derive(Clone, Debug)]
pub(crate) enum Mode {
    Create,
    Update,
    Copy { source_id: String },
    Delete,
}
#[derive(Clone)]
pub(crate) struct EditForm {
    pub mode: Mode,
    pub id: String,
    pub fields: ProviderBasic,
    pub key: String,
    pub replace_key: bool,
    pub clear_key: bool,
    pub saving: bool,
    pub draft: Option<AppProviderCreate>,
    pub claude: Option<Draft>,
    pub gemini: Option<super::gemini::Draft>,
}
impl std::fmt::Debug for EditForm {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("EditForm([redacted])")
    }
}
impl EditForm {
    pub fn new(
        mode: Mode,
        id: String,
        fields: ProviderBasic,
        draft: Option<AppProviderCreate>,
    ) -> Self {
        Self {
            mode,
            id,
            fields,
            key: String::new(),
            replace_key: false,
            clear_key: false,
            saving: false,
            draft,
            claude: None,
            gemini: None,
        }
    }
    pub fn claude_editable(&self) -> bool {
        self.claude.as_ref().is_some_and(|draft| draft.kind != Kind::Unsupported)
    }

    pub fn claude_request(
        &self,
        host: String,
    ) -> Option<vw_gateway_client::vw_api_types::app_providers::claude::ClaudeFormWrite> {
        let draft = self.claude.as_ref()?;
        draft.request(
            host,
            self.fields.clone(),
            self.key.clone(),
            self.replace_key || !draft.has_saved_key,
        )
    }

    pub fn gemini_editable(&self) -> bool {
        self.gemini.as_ref().is_some_and(|draft| draft.kind != super::gemini::Kind::Unsupported)
    }

    pub fn gemini_request(
        &self,
        host: String,
    ) -> Option<vw_gateway_client::vw_api_types::app_providers::gemini::GeminiFormWrite> {
        let draft = self.gemini.as_ref()?;
        draft.request(
            host,
            self.fields.clone(),
            self.key.clone(),
            self.replace_key || !draft.has_saved_key,
        )
    }

    pub fn identified(&self) -> bool {
        !self.id.trim().is_empty() && !self.fields.name.trim().is_empty()
    }

    pub fn draft_ready(&self) -> bool {
        self.draft.as_ref().is_some_and(|draft| {
            !draft.model.trim().is_empty()
                && !draft.base_url.trim().is_empty()
                && !draft.add_to_live
        })
    }
    pub fn request(&self) -> Result<ProviderBasicWrite, String> {
        if !self.identified() {
            return Err("请填写标识与名称".into());
        }
        if matches!(self.mode, Mode::Create) && !self.draft_ready() {
            return Err("待补完整应用配置，请在应用表单中配置后保存".into());
        }
        let action = match &self.mode {
            Mode::Create => {
                let mut draft =
                    self.draft.clone().ok_or("待补完整应用配置，请在应用表单中配置后保存")?;
                draft.api_key = ProviderSecretWrite::Keep;
                draft.name = self.fields.name.clone();
                BasicOperation::Create { draft: Box::new(draft) }
            }
            Mode::Update => BasicOperation::Update,
            Mode::Copy { source_id } => BasicOperation::Copy { source_id: source_id.clone() },
            Mode::Delete => return Err("请使用删除确认".into()),
        };
        let marker = self.key.trim();
        if (self.replace_key || matches!(self.mode, Mode::Create))
            && (marker.is_empty()
                || (marker.eq_ignore_ascii_case("[redacted]")
                    || marker.eq_ignore_ascii_case("<redacted>"))
                || marker.chars().all(|c| c == '*' || c == '•'))
        {
            return Err("请输入新的完整密钥，不能提交脱敏占位值".into());
        }
        let api_key = if self.clear_key {
            ProviderSecretWrite::Clear
        } else if self.replace_key || matches!(self.mode, Mode::Create) {
            ProviderSecretWrite::Replace { value: self.key.clone() }
        } else {
            ProviderSecretWrite::Keep
        };
        Ok(ProviderBasicWrite { fields: self.fields.clone(), api_key, action })
    }
    pub fn finish(&mut self, result: &Result<(), String>) -> bool {
        self.saving = false;
        result.is_ok()
    }
}
