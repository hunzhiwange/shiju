use serde_json::Value;

/// 模型摘要信息
///
/// 模型的基本信息和能力配置，
/// 用于在模型列表中显示。
#[derive(Debug, Clone)]
pub struct ModelSummary {
    /// 模型标识符
    pub id: String,
    /// 模型显示名称
    pub name: String,
    /// 是否启用
    pub enabled: bool,
    /// 是否支持工具调用
    pub toolcall: bool,
    /// 是否支持附件
    pub attachment: bool,
    /// 上下文长度限制
    pub context_limit: u64,
    /// 模型详细信息（JSON 格式）
    pub detail: Value,
}

/// 提供者模型摘要
///
/// 提供者及其包含的模型列表信息。
#[derive(Debug, Clone)]
pub struct ProviderModelsSummary {
    /// 提供者标识符
    pub id: String,
    /// 提供者显示名称
    pub name: String,
    /// 模型列表
    pub models: Vec<ModelSummary>,
}

/// 模型详情行
///
/// 模型详情对话框中显示的单行信息。
#[derive(Debug, Clone)]
pub struct ModelDetailRow {
    /// 行标签
    pub label: desktop_i18n::settings_models::messages::Feedback,
    /// 行值
    pub value: desktop_i18n::settings_models::messages::Feedback,
}

/// 模型详情对话框状态
///
/// 管理模型详情对话框的显示状态和内容。
#[derive(Debug, Clone)]
pub struct ModelDetailModalState {
    /// 提供者标识符
    pub provider_id: String,
    /// 提供者显示名称
    pub provider_name: String,
    /// 模型标识符
    pub model_id: String,
    /// 模型显示名称
    pub model_name: String,
    /// 详情行列表
    pub rows: Vec<ModelDetailRow>,
    /// 原始 JSON 数据
    pub raw_json: String,
    /// 是否显示原始 JSON
    pub show_raw: bool,
}

/// 模型配置内的子标签。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ModelConfigTab {
    /// 已连接模型目录。
    #[default]
    Models,
    /// 按请求提示选择模型的路由规则。
    ModelRoutes,
    /// 将查询匹配到模型路由的分类规则。
    QueryClassification,
    /// 图片输入、远程抓取与 OCR 配置。
    Multimodal,
}

/// 模型设置面板状态
///
/// 管理模型设置页面的状态，包括模型列表、
/// 搜索过滤和详情对话框。
#[derive(Debug, Clone)]
#[derive(Default)]
pub struct ModelSettingsState {
    /// 当前显示的模型配置子标签。
    pub active_tab: ModelConfigTab,
    /// 是否正在加载
    pub loading: bool,
    /// 是否正在保存模型启用状态
    pub saving: bool,
    /// 当前展开的提供者模型分组；默认全部折叠
    pub expanded_provider_ids: std::collections::HashSet<String>,
    /// 输入面板模型选择器中已折叠的提供商，独立于系统设置分组。
    pub input_collapsed_provider_ids: std::collections::BTreeSet<String>,
    /// 搜索查询
    pub query: String,
    /// 提供者模型列表
    pub providers: Vec<ProviderModelsSummary>,
    /// 保存错误信息
    pub save_error: Option<desktop_i18n::settings_models::messages::Feedback>,
    /// 模型详情对话框状态
    pub detail_modal: Option<ModelDetailModalState>,
}

/// 模型路由单条规则输入状态。
#[derive(Debug, Clone, Default)]
pub struct ModelRoute {
    /// 路由模式/提示。
    pub pattern: String,
    /// 提供者名称。
    pub provider: String,
    /// 模型名称。
    pub model: String,
    /// 优先级输入。
    pub priority_input: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CustomProviderModelTab {
    #[default]
    Info,
    Limits,
    Capabilities,
}

#[derive(Debug, Clone, Default)]
pub struct ModelCapabilityDefaults {
    pub reasoning: bool,
    pub tool_call: bool,
    pub temperature: bool,
    pub attachment: bool,
    pub interleaved: bool,
    pub input_modalities: Vec<String>,
    pub output_modalities: Vec<String>,
}

#[derive(Debug, Clone, Default)]
pub struct CustomProviderModelModalState {
    /// 当前选中的模型配置 Tab。
    pub active_tab: CustomProviderModelTab,
    /// 正在编辑的模型索引（None 表示新增）
    pub edit_index: Option<usize>,
    /// 模型标识符输入
    pub model_id: String,
    /// 模型显示名称输入
    pub display_name: String,
    /// 模型家族输入
    pub family: String,
    /// 上下文限制输入文本
    pub context_limit: String,
    /// 最大输入限制输入文本
    pub input_limit: String,
    /// 最大输出限制输入文本
    pub output_limit: String,
    /// 显式推理能力覆盖
    pub reasoning: Option<bool>,
    /// 显式工具调用能力覆盖
    pub tool_call: Option<bool>,
    /// 显式温度能力覆盖
    pub temperature: Option<bool>,
    /// 显式附件能力覆盖
    pub attachment: Option<bool>,
    /// 显式交错输入输出能力覆盖
    pub interleaved: Option<bool>,
    /// 显式输入模态覆盖
    pub input_modalities: Option<Vec<String>>,
    /// 显式输出模态覆盖
    pub output_modalities: Option<Vec<String>>,
    /// 模型目录提供的能力默认值，不直接写入用户配置。
    pub inherited_capabilities: Option<ModelCapabilityDefaults>,
    /// 编辑时原始模型 ID
    pub original_model_id: Option<String>,
}
