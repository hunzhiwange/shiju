pub const SHIJU_OFFICIAL_PROVIDER_ID: &str = "shiju";
pub const SHIJU_OFFICIAL_PROVIDER_NAME: &str = "识矩";

pub fn is_shiju_official_provider_id(provider_id: &str) -> bool {
    provider_id.trim().eq_ignore_ascii_case(SHIJU_OFFICIAL_PROVIDER_ID)
}

/// 提供者摘要信息
///
/// 提供者的基本信息和连接状态，
/// 用于在提供者列表中显示。
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct ProviderSummary {
    /// 提供者标识符
    pub id: String,
    /// 提供者显示名称
    pub name: String,
    /// 来源标签（如配置文件、内置等）
    pub source_label: desktop_i18n::settings_models::messages::Feedback,
    /// 是否已连接
    pub connected: bool,
}

/// 提供者连接状态
///
/// 存储提供者连接对话框的输入状态，
/// 包括提供者信息和 API 密钥。
#[derive(Debug, Clone, Default)]
pub struct ProviderConnectState {
    /// 提供者标识符
    pub provider_id: String,
    /// 提供者显示名称
    pub provider_name: String,
    /// API 密钥输入
    pub api_key: String,
}

/// 提供者请求头草稿
///
/// 用于自定义提供者的 HTTP 请求头配置。
#[derive(Debug, Clone, Default)]
pub struct ProviderHeaderDraft {
    /// 请求头键名
    pub key: String,
    /// 请求头值
    pub value: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CustomProviderTab {
    #[default]
    Basic,
    Authentication,
    Models,
}

pub use crate::state::CustomProviderModelTab;

/// 模型目录提供的能力默认值，仅用于编辑表单展示与交互。
///
/// 这些值不属于用户显式覆盖；保存模型时仍只写入对应的 `Option` 覆盖字段。
pub use crate::state::ModelCapabilityDefaults;

/// 编辑提供商模型列表的启用状态过滤。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CustomProviderModelFilter {
    All,
    #[default]
    Enabled,
    Disabled,
}

/// 自定义提供者模型草稿
///
/// 用于添加自定义模型时的输入状态。
#[derive(Debug, Clone)]
pub struct CustomProviderModelDraft {
    /// 模型标识符
    pub model_id: String,
    /// 模型显示名称
    pub display_name: String,
    /// 模型家族
    pub family: String,
    /// 上下文限制输入文本；空值表示沿用基线或安全默认值。
    pub context_limit: String,
    /// 最大输入限制输入文本；空值表示沿用基线或安全默认值。
    pub input_limit: String,
    /// 最大输出限制输入文本；空值表示沿用基线或安全默认值。
    pub output_limit: String,
    /// 显式推理能力覆盖；None 表示沿用基线或安全默认值。
    pub reasoning: Option<bool>,
    /// 显式工具调用能力覆盖；None 表示沿用基线或安全默认值。
    pub tool_call: Option<bool>,
    /// 显式温度能力覆盖；None 表示沿用基线或安全默认值。
    pub temperature: Option<bool>,
    /// 显式附件能力覆盖；None 表示沿用基线或安全默认值。
    pub attachment: Option<bool>,
    /// 显式交错输入输出能力覆盖；复杂 field 形式保留在原始配置中。
    pub interleaved: Option<bool>,
    /// 显式输入模态覆盖；None 表示沿用基线或安全默认值。
    pub input_modalities: Option<Vec<String>>,
    /// 显式输出模态覆盖；None 表示沿用基线或安全默认值。
    pub output_modalities: Option<Vec<String>>,
    /// 模型目录提供的能力默认值，不直接写入用户配置。
    pub inherited_capabilities: Option<ModelCapabilityDefaults>,
    /// 是否启用；对应配置中的 `status: active|disabled`。
    pub enabled: bool,
    /// 是否来自系统默认模型目录；用于保存时避免把未改动的禁用目录模型写入配置。
    pub from_catalog: bool,
    /// 编辑时原始模型 ID，用于模型改名时迁移旧配置。
    pub original_model_id: Option<String>,
}

impl Default for CustomProviderModelDraft {
    fn default() -> Self {
        Self {
            model_id: String::new(),
            display_name: String::new(),
            family: String::new(),
            context_limit: String::new(),
            input_limit: String::new(),
            output_limit: String::new(),
            reasoning: None,
            tool_call: None,
            temperature: None,
            attachment: None,
            interleaved: None,
            input_modalities: None,
            output_modalities: None,
            inherited_capabilities: None,
            enabled: true,
            from_catalog: false,
            original_model_id: None,
        }
    }
}

/// 自定义提供者草稿
///
/// 存储创建或编辑自定义提供者时的所有输入状态，
/// 包括基本信息、认证和模型配置。
#[derive(Debug, Clone)]
pub struct CustomProviderDraft {
    /// 提供者标识符
    pub provider_id: String,
    /// 提供者显示名称
    pub display_name: String,
    /// API 基础 URL
    pub base_url: String,
    /// API 密钥
    pub api_key: String,
    /// 自定义请求头列表
    pub headers: Vec<ProviderHeaderDraft>,
    /// 模型列表
    pub models: Vec<CustomProviderModelDraft>,
}

impl Default for CustomProviderDraft {
    fn default() -> Self {
        Self {
            provider_id: String::new(),
            display_name: String::new(),
            base_url: String::new(),
            api_key: String::new(),
            headers: vec![ProviderHeaderDraft::default()],
            models: vec![CustomProviderModelDraft::default()],
        }
    }
}

/// 自定义提供者模型对话框状态
///
/// 管理添加或编辑自定义提供者模型时的对话框状态。
pub use crate::state::CustomProviderModelModalState;

/// 模型目录条目
///
/// “更多模型”目录中显示的模型及其所属提供者信息。
#[derive(Debug, Clone)]
pub struct ModelCatalogEntry {
    /// 提供者标识符
    pub provider_id: String,
    /// 提供者显示名称
    pub provider_name: String,
    /// 模型标识符
    pub model_id: String,
    /// 模型显示名称
    pub model_name: String,
}

/// 提供者设置面板状态
///
/// 管理提供者设置页面的完整状态，包括提供者列表、
/// 目录搜索、连接对话框和自定义提供者配置。
#[derive(Debug, Clone)]
pub struct ProviderSettingsState {
    /// 是否正在加载提供者列表
    pub loading: bool,
    /// 是否正在从远端同步模型目录
    pub models_syncing: bool,
    /// 模型目录同步进度
    pub models_sync_progress: f32,
    /// 模型目录同步状态文案
    pub models_sync_label: desktop_i18n::settings_models::messages::Feedback,
    /// 提供者摘要列表
    pub providers: Vec<ProviderSummary>,
    /// 热门提供者模式名称列表
    pub popular_patterns: Vec<String>,
    /// 目录是否正在加载
    pub catalog_loading: bool,
    /// 目录对话框是否打开
    pub catalog_open: bool,
    /// 目录搜索查询
    pub catalog_query: String,
    /// 目录条目列表
    pub catalog_items: Vec<ModelCatalogEntry>,
    /// 连接对话框状态
    pub connect_modal: Option<ProviderConnectState>,
    /// 连接错误信息
    pub connect_error: Option<desktop_i18n::settings_models::messages::Feedback>,
    /// 等待确认断开连接的提供者 ID
    pub disconnect_confirm_provider_id: Option<String>,
    /// 自定义提供者对话框是否打开
    pub custom_provider_modal_open: bool,
    /// 自定义提供者对话框当前选中的 Tab
    pub custom_provider_tab: CustomProviderTab,
    /// 当前是否正在编辑已有提供者
    pub custom_editing_provider_id: Option<String>,
    /// 是否正在发现当前提供者的远端模型
    pub custom_models_discovering: bool,
    /// 自定义提供者草稿
    pub custom: CustomProviderDraft,
    /// 编辑提供商模型列表的启用状态过滤
    pub custom_models_filter: CustomProviderModelFilter,
    /// 自定义模型对话框状态
    pub custom_model_modal: Option<CustomProviderModelModalState>,
    /// 保存错误信息
    pub save_error: Option<desktop_i18n::settings_models::messages::Feedback>,
}

impl Default for ProviderSettingsState {
    fn default() -> Self {
        Self {
            loading: false,
            models_syncing: false,
            models_sync_progress: 0.0,
            models_sync_label: desktop_i18n::settings_models::messages::Feedback::default(),
            providers: Vec::new(),
            popular_patterns: vec![
                "OpenCode Zen".to_string(),
                "Anthropic".to_string(),
                "GitHub Copilot".to_string(),
                "OpenAI".to_string(),
                "Google".to_string(),
                "OpenRouter".to_string(),
                "Vercel AI Gateway".to_string(),
            ],
            catalog_loading: false,
            catalog_open: false,
            catalog_query: String::new(),
            catalog_items: Vec::new(),
            connect_modal: None,
            connect_error: None,
            disconnect_confirm_provider_id: None,
            custom_provider_modal_open: false,
            custom_provider_tab: CustomProviderTab::default(),
            custom_editing_provider_id: None,
            custom_models_discovering: false,
            custom: CustomProviderDraft::default(),
            custom_models_filter: CustomProviderModelFilter::default(),
            custom_model_modal: None,
            save_error: None,
        }
    }
}


#[cfg(test)]
#[path = "provider_state_tests.rs"]
mod tests;
