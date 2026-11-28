//! Read-only presets for the existing application create forms.
use crate::state::{desktop, edit};
use vw_api_app_provider_edit::catalog::ModelDevProviderPreset;

pub mod models_dev;
use app_model_catalog::{
    preset_catalog::{claude_presets, codex_presets, gemini_grok_presets},
    preset_claude::{ClaudePreset, ClaudePresetApiFormat, ClaudePresetCategory},
    preset_codex::{
        CodexPreset, CodexPresetApiFormat, CodexPresetAuthProvider, CodexPresetCategory,
    },
    preset_gemini_grok::GeminiGrokPreset,
    preset_search::PresetSearchEntry,
};
use iced::widget::text_editor::Content;
use vw_api_app_identity::AppId;
use vw_api_app_provider_profiles::{
    codex_catalog::CodexModelMapping, desktop_mapping::DesktopMappingMode,
};

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum Category {
    #[default]
    All,
    Account,
    Vendor,
    ThirdParty,
    Cloud,
    Local,
}
impl Category {
    pub const ORDER: [Self; 6] =
        [Self::All, Self::Account, Self::Vendor, Self::ThirdParty, Self::Cloud, Self::Local];

    pub fn label(self) -> &'static str {
        match self {
            Self::All => "全部",
            Self::Account => "账号登录",
            Self::Vendor => "模型厂商",
            Self::ThirdParty => "第三方平台",
            Self::Cloud => "云服务商",
            Self::Local => "本地",
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct Picker {
    pub query: String,
    pub category: Category,
    pub family: Option<String>,
    pub account_help: bool,
    pub catalog_request: String,
    pub catalog: Option<Result<Vec<ModelDevProviderPreset>, ()>>,
    pub catalog_provider: Option<usize>,
    pub catalog_model_query: String,
}
#[derive(Clone, Debug)]
pub enum Event {
    Open,
    Close,
    Query(String),
    Filter(Category),
    OpenVersions(String),
    BackToCatalog,
    Custom,
    Select(String),
    OpenAuthorization,
    RetryCatalog,
    CatalogLoaded {
        scope: crate::Scope,
        request: String,
        rows: Result<Vec<ModelDevProviderPreset>, ()>,
    },
    ChooseCatalogProvider(usize),
    CatalogModelQuery(String),
    ChooseCatalogModel(usize),
}

#[derive(Clone, Copy)]
pub enum Preset {
    Claude(&'static ClaudePreset),
    Codex(&'static CodexPreset),
    GeminiGrok(&'static GeminiGrokPreset),
}
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Availability {
    Official(&'static str),
    Direct,
    Routing,
    Unavailable,
}

pub fn app_name(app: AppId) -> Option<&'static str> {
    Some(match app {
        AppId::Claude => "Claude Code",
        AppId::ClaudeDesktop => "Claude Desktop",
        AppId::Codex => "Codex",
        AppId::Gemini => "Gemini CLI",
        AppId::GrokBuild => "Grok Build",
        _ => return None,
    })
}

pub fn entries(app: AppId) -> Vec<PresetSearchEntry<'static, Preset>> {
    match app {
        AppId::Claude | AppId::ClaudeDesktop => claude_presets(app)
            .filter(|preset| !preset.provider().hidden)
            .map(Preset::Claude)
            .map(Preset::entry)
            .collect(),
        AppId::Codex => codex_presets(app)
            .filter(|preset| !preset.hidden)
            .map(Preset::Codex)
            .map(Preset::entry)
            .collect(),
        AppId::Gemini | AppId::GrokBuild => {
            gemini_grok_presets(app).map(Preset::GeminiGrok).map(Preset::entry).collect()
        }
        _ => Vec::new(),
    }
}

impl Preset {
    pub fn account_label(self) -> Option<&'static str> {
        match self {
            Self::Claude(p) => match p.provider().oauth_provider.as_deref() {
                Some("codex_oauth") => Some("用 ChatGPT 账号登录"),
                Some("github_copilot") => Some("用 GitHub 账号登录"),
                Some("xai_oauth") => Some("用 xAI 账号登录"),
                None if p.provider().category == ClaudePresetCategory::Official => {
                    Some("用 Claude 账号登录")
                }
                _ => None,
            },
            Self::Codex(p) => match p.auth_provider {
                Some(CodexPresetAuthProvider::GithubCopilot) => Some("用 GitHub 账号登录"),
                Some(CodexPresetAuthProvider::XaiOauth) => Some("用 xAI 账号登录"),
                Some(CodexPresetAuthProvider::CodexOauth) => Some("用 ChatGPT 账号登录"),
                None if p.is_official => Some("用 ChatGPT 账号登录"),
                None => None,
            },
            Self::GeminiGrok(p) if p.official_provider_id.is_some() => match p.app_id {
                AppId::Gemini => Some("用 Google 账号登录"),
                AppId::GrokBuild => Some("用 xAI 账号登录"),
                _ => None,
            },
            Self::GeminiGrok(_) => None,
        }
    }

    fn entry(self) -> PresetSearchEntry<'static, Self> {
        let (name, name_key, website, family, plan, region) = match self {
            Self::Claude(p) => {
                let p = p.provider();
                (
                    &p.name,
                    p.name_key.as_deref(),
                    &p.website_url,
                    p.family.as_deref(),
                    p.plan_key,
                    p.region_key,
                )
            }
            Self::Codex(p) => (
                &p.name,
                p.name_key.as_deref(),
                &p.website_url,
                p.family.as_deref(),
                p.plan_key,
                p.region_key,
            ),
            Self::GeminiGrok(p) => (
                &p.name,
                p.name_key.as_deref(),
                &p.website_url,
                p.family.as_deref(),
                p.plan_key,
                p.region_key,
            ),
        };
        PresetSearchEntry {
            preset: self,
            name,
            name_key,
            website_url: Some(website),
            family,
            plan,
            region,
        }
    }

    pub fn icon(self) -> Option<&'static str> {
        match self.entry().name {
            "Cloudflare AI Gateway" => return Some("cloudflare-ai-gateway"),
            "Fireworks AI" => return Some("fireworks-ai"),
            "Ollama" | "Ollama Cloud" => return Some("ollama-cloud"),
            _ => {}
        }
        match self {
            Self::Claude(p) => p.provider().icon.as_deref(),
            Self::Codex(p) => p.icon.as_deref(),
            Self::GeminiGrok(p) => p.icon.as_deref(),
        }
    }

    pub fn desktop_endpoint_needs_account_id(self) -> bool {
        matches!(self, Self::Claude(ClaudePreset::Desktop { .. }))
            && self.endpoint().is_some_and(desktop::needs_account_id)
    }

    pub fn local_endpoint(self) -> Option<&'static str> {
        (self.category() == Category::Local).then(|| self.endpoint()).flatten()
    }

    pub fn category(self) -> Category {
        match self {
            Self::Claude(p) => match p.provider().category {
                ClaudePresetCategory::Official | ClaudePresetCategory::Account => Category::Account,
                ClaudePresetCategory::CnOfficial => Category::Vendor,
                ClaudePresetCategory::CloudProvider => Category::Cloud,
                ClaudePresetCategory::Local => Category::Local,
            },
            Self::Codex(p) => match p.category {
                CodexPresetCategory::Official | CodexPresetCategory::Account => Category::Account,
                CodexPresetCategory::CnOfficial => Category::Vendor,
                CodexPresetCategory::CloudProvider => Category::Cloud,
                CodexPresetCategory::Local => Category::Local,
            },
            Self::GeminiGrok(_) => Category::Account,
        }
    }

    pub fn availability(self) -> Availability {
        match self {
            Self::Claude(p) => {
                let provider = p.provider();
                if provider.category == ClaudePresetCategory::Official {
                    return Availability::Official(match p {
                        ClaudePreset::Code { .. } => "claude-official",
                        ClaudePreset::Desktop { .. } => "claude-desktop-official",
                    });
                }
                if provider.oauth_provider.is_some()
                    || provider.api_format != ClaudePresetApiFormat::Anthropic
                    || matches!(p, ClaudePreset::Desktop { mode: DesktopMappingMode::Proxy, .. })
                {
                    return Availability::Routing;
                }
                if provider.api_key_header && matches!(p, ClaudePreset::Desktop { .. }) {
                    return Availability::Unavailable;
                }
            }
            Self::Codex(p) if p.is_official => return Availability::Official("codex-official"),
            Self::Codex(p)
                if p.requires_oauth
                    || p.auth_provider.is_some()
                    || p.api_format == Some(CodexPresetApiFormat::OpenaiChat) =>
            {
                return Availability::Routing;
            }
            Self::GeminiGrok(p) => {
                return p
                    .official_provider_id
                    .as_deref()
                    .map_or(Availability::Unavailable, Availability::Official);
            }
            _ => {}
        }
        if self.endpoint().is_some() { Availability::Direct } else { Availability::Unavailable }
    }

    fn endpoint(self) -> Option<&'static str> {
        let endpoint = match self {
            Self::Claude(p) => p.provider().base_url.as_deref(),
            Self::Codex(p) => p.base_url.as_deref(),
            Self::GeminiGrok(p) => p.base_url.as_deref(),
        }?;
        (!endpoint.contains(['{', '}', '<', '>'])
            && app_provider_validation::validation::endpoint(endpoint).is_ok())
        .then_some(endpoint)
    }

    pub fn fill_desktop(self, form: &mut desktop::Form) {
        if let (Self::Claude(ClaudePreset::Desktop { provider, .. }), Some(endpoint)) =
            (self, self.endpoint())
        {
            form.name.clone_from(&provider.name);
            form.base_url = endpoint.into();
        }
    }

    pub fn fill_editor(self, form: &mut edit::EditForm) {
        let Some(endpoint) = self.endpoint() else { return };
        let entry = self.entry();
        form.fields.name = entry.name.into();
        form.fields.website_url = entry.website_url.map(str::to_owned);
        match self {
            Self::Claude(ClaudePreset::Code { provider, models, .. }) => {
                if let Some(draft) = &mut form.claude {
                    draft.kind = crate::state::claude::Kind::ThirdParty;
                    draft.base_url = endpoint.into();
                    draft.api_key_header = provider.api_key_header;
                    draft.model = models.model.clone().unwrap_or_default();
                }
            }
            Self::Codex(p) => {
                if let Some(draft) = &mut form.codex {
                    let models = p
                        .model_catalog
                        .iter()
                        .map(|row| CodexModelMapping {
                            slug: row.model.clone(),
                            display_name: row.display_name.as_ref().unwrap_or(&row.model).clone(),
                        })
                        .collect();
                    fill_codex(
                        draft,
                        endpoint,
                        p.default_model.as_deref(),
                        p.default_reasoning_effort.as_deref(),
                        p.requires_openai_auth,
                        models,
                    );
                }
            }
            Self::Claude(_) | Self::GeminiGrok(_) => {}
        }
    }
}

fn fill_codex(
    draft: &mut crate::state::codex::Draft,
    endpoint: &str,
    model: Option<&str>,
    effort: Option<&str>,
    requires_auth: Option<bool>,
    models: Vec<CodexModelMapping>,
) {
    let mut config = toml::Table::new();
    config.insert("model_provider".into(), "vendor".into());
    if let Some(model) = model {
        config.insert("model".into(), model.into());
    }
    if let Some(effort) = effort {
        config.insert("model_reasoning_effort".into(), effort.into());
    }
    let mut route = toml::Table::from_iter([
        ("base_url".into(), endpoint.into()),
        ("wire_api".into(), "responses".into()),
    ]);
    if let Some(auth) = requires_auth {
        route.insert("requires_openai_auth".into(), auth.into());
    }
    config.insert(
        "model_providers".into(),
        toml::Table::from_iter([("vendor".into(), route.into())]).into(),
    );
    draft.toml = Content::with_text(&toml::to_string(&config).expect("serializable TOML table"));
    draft.models = Some(models);
}

fn fill_grok(
    draft: &mut crate::state::grok::Draft,
    endpoint: &str,
    name: &str,
    model: &str,
    context: i64,
) {
    let mut config = toml::Table::new();
    config.insert(
        "models".into(),
        toml::Table::from_iter([("default".into(), "vendor".into())]).into(),
    );
    let table = toml::Table::from_iter([
        ("model".into(), model.into()),
        ("name".into(), name.into()),
        ("base_url".into(), endpoint.into()),
        ("api_backend".into(), "responses".into()),
        ("context_window".into(), context.into()),
    ]);
    config.insert("model".into(), toml::Table::from_iter([("vendor".into(), table.into())]).into());
    draft.toml = Content::with_text(&toml::to_string(&config).expect("serializable TOML table"));
}

pub fn new_id() -> String {
    let mut random = [0; 8];
    rand::fill(&mut random);
    format!("provider-{}", hex::encode(random))
}
