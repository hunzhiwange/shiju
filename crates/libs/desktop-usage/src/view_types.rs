#[derive(Debug, Clone)]
pub struct UsageStepRawModal {
    /// 对应的步骤索引
    pub step_index: u32,
    /// 完整原始记录文本
    pub content: String,
}

/// 使用量模型信息
///
/// 包含模型的详细使用信息和定价配置，
/// 用于显示和计算 token 使用量和成本。
#[derive(Debug, Clone)]
pub struct UsageModelInfo {
    /// 提供者标识符
    pub provider_id: String,
    /// 提供者显示名称
    pub provider_name: String,
    /// 模型标识符
    pub model_id: String,
    /// 模型显示名称
    pub model_name: String,
    /// 上下文长度限制（token 数）
    pub context_limit: u64,
    /// 当前请求可用的输入窗口（token 数）
    pub input_limit: u64,
    /// 输出长度限制（token 数）
    pub output_limit: u64,
    /// 是否使用未知模型的保守默认限制
    pub limit_is_default: bool,
    /// 输入 token 每百万的成本（美元）
    pub cost_input_per_million: f64,
    /// 输出 token 每百万的成本（美元）
    pub cost_output_per_million: f64,
    /// 缓存读取 token 每百万的成本（美元）
    pub cost_cache_read_per_million: f64,
    /// 缓存写入 token 每百万的成本（美元）
    pub cost_cache_write_per_million: f64,
}


#[derive(Clone, Debug)]
pub enum Message {
    SessionRightClicked(String, f32, f32), SessionRenamePressed(String),
    SessionCopyPressed(String), SessionArchivePressed(String), SessionDeletePressed(String),
    SessionMenuClose, UsageStepRawClose, CopyCode(String),
    UsageTurnStepRawOpen(String, u32), UsageTurnStepToggled(String, u32),
    ReplaySessionFromSnapshot(String), OpenPathInFinder(String),
}
pub struct ViewInput<'a> {
    pub desktop_locale: config_ui::DesktopLocale,
    pub usage: &'a session_ui_types::TokenUsage,
    pub usage_model_info: Option<&'a UsageModelInfo>,
    pub auto_model: bool,
    pub model: String,
    pub active_session_id: Option<&'a String>,
    pub session_menu_id: Option<&'a String>,
    pub session_menu_anchor: Option<iced::Point>,
    pub usage_step_expanded: &'a std::collections::HashSet<u32>,
    pub usage_step_raw_loading: &'a std::collections::HashSet<(String, String, u32)>,
    pub usage_step_raw_payloads: &'a std::collections::HashMap<(String, String, u32), Result<Option<serde_json::Value>, String>>,
    pub step_llm_raw: &'a std::collections::HashMap<(String, String, u32), serde_json::Value>,
    pub usage_step_raw_modal: Option<&'a UsageStepRawModal>,
    pub last_copied_code_hash: Option<u64>,
}
