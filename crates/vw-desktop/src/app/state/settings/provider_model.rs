use super::*;

pub(crate) const SHIJU_OFFICIAL_PROVIDER_ID: &str = "shiju";
pub(crate) const SHIJU_OFFICIAL_PROVIDER_NAME: &str = "识矩";

pub(crate) fn is_shiju_official_provider_id(provider_id: &str) -> bool {
    provider_id.trim().eq_ignore_ascii_case(SHIJU_OFFICIAL_PROVIDER_ID)
}

/// 提供者摘要信息
///
/// 提供者的基本信息和连接状态，
/// 用于在提供者列表中显示。
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub(crate) struct ProviderSummary {
    /// 提供者标识符
    pub(crate) id: String,
    /// 提供者显示名称
    pub(crate) name: String,
    /// 来源标签（如配置文件、内置等）
    pub(crate) source_label: String,
    /// 是否已连接
    pub(crate) connected: bool,
}

/// 提供者连接状态
///
/// 存储提供者连接对话框的输入状态，
/// 包括提供者信息和 API 密钥。
#[derive(Debug, Clone, Default)]
pub(crate) struct ProviderConnectState {
    /// 提供者标识符
    pub(crate) provider_id: String,
    /// 提供者显示名称
    pub(crate) provider_name: String,
    /// API 密钥输入
    pub(crate) api_key: String,
}

/// 提供者请求头草稿
///
/// 用于自定义提供者的 HTTP 请求头配置。
#[derive(Debug, Clone, Default)]
pub(crate) struct ProviderHeaderDraft {
    /// 请求头键名
    pub(crate) key: String,
    /// 请求头值
    pub(crate) value: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CustomProviderTab {
    #[default]
    Basic,
    Authentication,
    Models,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CustomProviderModelTab {
    #[default]
    Info,
    Limits,
    Capabilities,
}

/// 模型目录提供的能力默认值，仅用于编辑表单展示与交互。
///
/// 这些值不属于用户显式覆盖；保存模型时仍只写入对应的 `Option` 覆盖字段。
#[derive(Debug, Clone, Default)]
pub(crate) struct ModelCapabilityDefaults {
    pub(crate) reasoning: bool,
    pub(crate) tool_call: bool,
    pub(crate) temperature: bool,
    pub(crate) attachment: bool,
    pub(crate) interleaved: bool,
    pub(crate) input_modalities: Vec<String>,
    pub(crate) output_modalities: Vec<String>,
}

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
pub(crate) struct CustomProviderModelDraft {
    /// 模型标识符
    pub(crate) model_id: String,
    /// 模型显示名称
    pub(crate) display_name: String,
    /// 模型家族
    pub(crate) family: String,
    /// 上下文限制输入文本；空值表示沿用基线或安全默认值。
    pub(crate) context_limit: String,
    /// 最大输入限制输入文本；空值表示沿用基线或安全默认值。
    pub(crate) input_limit: String,
    /// 最大输出限制输入文本；空值表示沿用基线或安全默认值。
    pub(crate) output_limit: String,
    /// 显式推理能力覆盖；None 表示沿用基线或安全默认值。
    pub(crate) reasoning: Option<bool>,
    /// 显式工具调用能力覆盖；None 表示沿用基线或安全默认值。
    pub(crate) tool_call: Option<bool>,
    /// 显式温度能力覆盖；None 表示沿用基线或安全默认值。
    pub(crate) temperature: Option<bool>,
    /// 显式附件能力覆盖；None 表示沿用基线或安全默认值。
    pub(crate) attachment: Option<bool>,
    /// 显式交错输入输出能力覆盖；复杂 field 形式保留在原始配置中。
    pub(crate) interleaved: Option<bool>,
    /// 显式输入模态覆盖；None 表示沿用基线或安全默认值。
    pub(crate) input_modalities: Option<Vec<String>>,
    /// 显式输出模态覆盖；None 表示沿用基线或安全默认值。
    pub(crate) output_modalities: Option<Vec<String>>,
    /// 模型目录提供的能力默认值，不直接写入用户配置。
    pub(crate) inherited_capabilities: Option<ModelCapabilityDefaults>,
    /// 是否启用；对应配置中的 `status: active|disabled`。
    pub(crate) enabled: bool,
    /// 是否来自系统默认模型目录；用于保存时避免把未改动的禁用目录模型写入配置。
    pub(crate) from_catalog: bool,
    /// 编辑时原始模型 ID，用于模型改名时迁移旧配置。
    pub(crate) original_model_id: Option<String>,
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
    pub(crate) provider_id: String,
    /// 提供者显示名称
    pub(crate) display_name: String,
    /// API 基础 URL
    pub(crate) base_url: String,
    /// API 密钥
    pub(crate) api_key: String,
    /// 自定义请求头列表
    pub(crate) headers: Vec<ProviderHeaderDraft>,
    /// 模型列表
    pub(crate) models: Vec<CustomProviderModelDraft>,
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
#[derive(Debug, Clone, Default)]
pub(crate) struct CustomProviderModelModalState {
    /// 当前选中的模型配置 Tab。
    pub(crate) active_tab: CustomProviderModelTab,
    /// 正在编辑的模型索引（None 表示新增）
    pub(crate) edit_index: Option<usize>,
    /// 模型标识符输入
    pub(crate) model_id: String,
    /// 模型显示名称输入
    pub(crate) display_name: String,
    /// 模型家族输入
    pub(crate) family: String,
    /// 上下文限制输入文本
    pub(crate) context_limit: String,
    /// 最大输入限制输入文本
    pub(crate) input_limit: String,
    /// 最大输出限制输入文本
    pub(crate) output_limit: String,
    /// 显式推理能力覆盖
    pub(crate) reasoning: Option<bool>,
    /// 显式工具调用能力覆盖
    pub(crate) tool_call: Option<bool>,
    /// 显式温度能力覆盖
    pub(crate) temperature: Option<bool>,
    /// 显式附件能力覆盖
    pub(crate) attachment: Option<bool>,
    /// 显式交错输入输出能力覆盖
    pub(crate) interleaved: Option<bool>,
    /// 显式输入模态覆盖
    pub(crate) input_modalities: Option<Vec<String>>,
    /// 显式输出模态覆盖
    pub(crate) output_modalities: Option<Vec<String>>,
    /// 模型目录提供的能力默认值，不直接写入用户配置。
    pub(crate) inherited_capabilities: Option<ModelCapabilityDefaults>,
    /// 编辑时原始模型 ID
    pub(crate) original_model_id: Option<String>,
}

/// 模型目录条目
///
/// “更多模型”目录中显示的模型及其所属提供者信息。
#[derive(Debug, Clone)]
pub struct ModelCatalogEntry {
    /// 提供者标识符
    pub(crate) provider_id: String,
    /// 提供者显示名称
    pub(crate) provider_name: String,
    /// 模型标识符
    pub(crate) model_id: String,
    /// 模型显示名称
    pub(crate) model_name: String,
}

/// 提供者设置面板状态
///
/// 管理提供者设置页面的完整状态，包括提供者列表、
/// 目录搜索、连接对话框和自定义提供者配置。
#[derive(Debug, Clone)]
pub(crate) struct ProviderSettingsState {
    /// 是否正在加载提供者列表
    pub(crate) loading: bool,
    /// 是否正在从远端同步模型目录
    pub(crate) models_syncing: bool,
    /// 模型目录同步进度
    pub(crate) models_sync_progress: f32,
    /// 模型目录同步状态文案
    pub(crate) models_sync_label: String,
    /// 提供者摘要列表
    pub(crate) providers: Vec<ProviderSummary>,
    /// 热门提供者模式名称列表
    pub(crate) popular_patterns: Vec<String>,
    /// 目录是否正在加载
    pub(crate) catalog_loading: bool,
    /// 目录对话框是否打开
    pub(crate) catalog_open: bool,
    /// 目录搜索查询
    pub(crate) catalog_query: String,
    /// 目录条目列表
    pub(crate) catalog_items: Vec<ModelCatalogEntry>,
    /// 连接对话框状态
    pub(crate) connect_modal: Option<ProviderConnectState>,
    /// 连接错误信息
    pub(crate) connect_error: Option<String>,
    /// 等待确认断开连接的提供者 ID
    pub(crate) disconnect_confirm_provider_id: Option<String>,
    /// 自定义提供者对话框是否打开
    pub(crate) custom_provider_modal_open: bool,
    /// 自定义提供者对话框当前选中的 Tab
    pub(crate) custom_provider_tab: CustomProviderTab,
    /// 当前是否正在编辑已有提供者
    pub(crate) custom_editing_provider_id: Option<String>,
    /// 是否正在发现当前提供者的远端模型
    pub(crate) custom_models_discovering: bool,
    /// 自定义提供者草稿
    pub(crate) custom: CustomProviderDraft,
    /// 编辑提供商模型列表的启用状态过滤
    pub(crate) custom_models_filter: CustomProviderModelFilter,
    /// 自定义模型对话框状态
    pub(crate) custom_model_modal: Option<CustomProviderModelModalState>,
    /// 保存错误信息
    pub(crate) save_error: Option<String>,
}

impl Default for ProviderSettingsState {
    fn default() -> Self {
        Self {
            loading: false,
            models_syncing: false,
            models_sync_progress: 0.0,
            models_sync_label: String::new(),
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

/// 模型摘要信息
///
/// 模型的基本信息和能力配置，
/// 用于在模型列表中显示。
#[derive(Debug, Clone)]
pub struct ModelSummary {
    /// 模型标识符
    pub(crate) id: String,
    /// 模型显示名称
    pub(crate) name: String,
    /// 是否启用
    pub(crate) enabled: bool,
    /// 是否支持工具调用
    pub(crate) toolcall: bool,
    /// 是否支持附件
    pub(crate) attachment: bool,
    /// 上下文长度限制
    pub(crate) context_limit: u64,
    /// 模型详细信息（JSON 格式）
    pub(crate) detail: Value,
}

/// 提供者模型摘要
///
/// 提供者及其包含的模型列表信息。
#[derive(Debug, Clone)]
pub struct ProviderModelsSummary {
    /// 提供者标识符
    pub(crate) id: String,
    /// 提供者显示名称
    pub(crate) name: String,
    /// 模型列表
    pub(crate) models: Vec<ModelSummary>,
}

/// 模型详情行
///
/// 模型详情对话框中显示的单行信息。
#[derive(Debug, Clone)]
pub(crate) struct ModelDetailRow {
    /// 行标签
    pub(crate) label: String,
    /// 行值
    pub(crate) value: String,
}

/// 模型详情对话框状态
///
/// 管理模型详情对话框的显示状态和内容。
#[derive(Debug, Clone)]
pub(crate) struct ModelDetailModalState {
    /// 提供者标识符
    pub(crate) provider_id: String,
    /// 提供者显示名称
    pub(crate) provider_name: String,
    /// 模型标识符
    pub(crate) model_id: String,
    /// 模型显示名称
    pub(crate) model_name: String,
    /// 详情行列表
    pub(crate) rows: Vec<ModelDetailRow>,
    /// 原始 JSON 数据
    pub(crate) raw_json: String,
    /// 是否显示原始 JSON
    pub(crate) show_raw: bool,
}

/// 模型设置面板状态
///
/// 管理模型设置页面的状态，包括模型列表、
/// 搜索过滤和详情对话框。
#[derive(Debug, Clone)]
#[derive(Default)]
pub(crate) struct ModelSettingsState {
    /// 是否正在加载
    pub(crate) loading: bool,
    /// 是否正在保存模型启用状态
    pub(crate) saving: bool,
    /// 当前展开的提供者模型分组；默认全部折叠
    pub(crate) expanded_provider_ids: std::collections::HashSet<String>,
    /// 搜索查询
    pub(crate) query: String,
    /// 提供者模型列表
    pub(crate) providers: Vec<ProviderModelsSummary>,
    /// 保存错误信息
    pub(crate) save_error: Option<String>,
    /// 模型详情对话框状态
    pub(crate) detail_modal: Option<ModelDetailModalState>,
}

/// 嵌入路由草稿
///
/// 表示系统设置界面中一条可编辑的嵌入路由输入。
#[derive(Debug, Clone, Default)]
pub(crate) struct EmbeddingRouteDraft {
    /// 路由模式（映射到配置中的 hint）
    pub(crate) pattern: String,
    /// 提供商名称
    pub(crate) provider: String,
    /// 模型名称
    pub(crate) model: String,
    /// 维度输入文本
    pub(crate) dimensions: String,
    /// 可选 API Key 覆盖，优先于全局 api_key。
    pub(crate) api_key_input: String,
}

/// 嵌入路由设置面板状态
///
/// 管理嵌入模型路由列表的新增、编辑和删除草稿状态。
#[derive(Debug, Clone, Default)]
pub(crate) struct EmbeddingRoutesSettingsState {
    pub(crate) routes: Vec<EmbeddingRouteDraft>,
    pub(crate) save_error: Option<String>,
    pub(crate) save_success: bool,
}

/// 模型路由单条规则输入状态。
#[derive(Debug, Clone, Default)]
pub(crate) struct ModelRoute {
    /// 路由模式/提示。
    pub(crate) pattern: String,
    /// 提供者名称。
    pub(crate) provider: String,
    /// 模型名称。
    pub(crate) model: String,
    /// 优先级输入。
    pub(crate) priority_input: String,
}

/// 模型路由设置面板状态。
#[derive(Debug, Clone, Default)]
pub(crate) struct ModelRoutesSettingsState {
    /// 路由列表。
    pub(crate) routes: Vec<ModelRoute>,
    /// 保存错误信息。
    pub(crate) save_error: Option<String>,
}

/// 查询分类单条规则输入状态。
#[derive(Debug, Clone)]
pub(crate) struct QueryClassificationRuleInput {
    /// 匹配模式输入。
    pub(crate) pattern: String,
    /// 分类类别输入。
    pub(crate) category: String,
    /// 优先级输入。
    pub(crate) priority_input: String,
}

impl Default for QueryClassificationRuleInput {
    fn default() -> Self {
        Self { pattern: String::new(), category: String::new(), priority_input: "0".to_string() }
    }
}

/// 查询分类设置面板状态。
#[derive(Debug, Clone)]
#[derive(Default)]
pub(crate) struct QueryClassificationSettingsState {
    /// 是否启用查询分类。
    pub(crate) enabled: bool,
    /// 查询分类规则列表。
    pub(crate) rules: Vec<QueryClassificationRuleInput>,
    /// 保存错误信息。
    pub(crate) save_error: Option<String>,
}

/// 心跳设置面板状态
///
/// 管理心跳功能的配置，包括定时消息发送设置。
#[derive(Debug, Clone)]
pub(crate) struct HeartbeatSettingsState {
    /// 是否启用心跳
    pub(crate) enabled: bool,
    /// 心跳间隔（分钟）
    pub(crate) interval_minutes: u32,
    /// 心跳消息输入
    pub(crate) message_input: String,
    /// 目标输入
    pub(crate) target_input: String,
    /// 接收者输入
    pub(crate) to_input: String,
    /// 是否显示帮助对话框
    pub(crate) show_help_modal: bool,
    /// 保存错误信息
    pub(crate) save_error: Option<String>,
}

impl Default for HeartbeatSettingsState {
    fn default() -> Self {
        Self {
            enabled: false,
            interval_minutes: 30,
            message_input: String::new(),
            target_input: String::new(),
            to_input: String::new(),
            show_help_modal: false,
            save_error: None,
        }
    }
}

/// 目标循环设置面板状态
///
/// 管理 autonomous goal loop 的执行开关、周期和事件投递目标配置。
#[derive(Debug, Clone)]
pub(crate) struct GoalLoopSettingsState {
    /// 是否启用目标循环
    pub(crate) enabled: bool,
    /// 循环间隔输入（分钟）
    pub(crate) interval_minutes_input: String,
    /// 单步超时输入（秒）
    pub(crate) step_timeout_secs_input: String,
    /// 每轮最大步数输入
    pub(crate) max_steps_per_cycle_input: String,
    /// 事件投递通道输入
    pub(crate) channel_input: String,
    /// 事件投递目标输入
    pub(crate) target_input: String,
    /// 保存错误信息
    pub(crate) save_error: Option<String>,
}

impl Default for GoalLoopSettingsState {
    fn default() -> Self {
        Self {
            enabled: false,
            interval_minutes_input: "10".to_string(),
            step_timeout_secs_input: "120".to_string(),
            max_steps_per_cycle_input: "3".to_string(),
            channel_input: String::new(),
            target_input: String::new(),
            save_error: None,
        }
    }
}

/// 定时任务设置面板状态
///
/// 管理 Cron 定时任务功能的配置。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CronSettingsTab {
    Jobs,
    Config,
    Add,
}

impl Default for CronSettingsTab {
    fn default() -> Self {
        Self::Jobs
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CronAddJobType {
    Shell,
    Agent,
}

impl CronAddJobType {
    pub(crate) fn as_api_value(self) -> &'static str {
        match self {
            Self::Shell => "shell",
            Self::Agent => "agent",
        }
    }
}

impl Default for CronAddJobType {
    fn default() -> Self {
        Self::Shell
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CronAddScheduleKind {
    Cron,
    At,
    Every,
}

impl CronAddScheduleKind {
    pub(crate) fn as_api_value(self) -> &'static str {
        match self {
            Self::Cron => "cron",
            Self::At => "at",
            Self::Every => "every",
        }
    }
}

impl Default for CronAddScheduleKind {
    fn default() -> Self {
        Self::Cron
    }
}

#[derive(Debug, Clone)]
pub(crate) struct CronJobDraft {
    pub(crate) name: String,
    pub(crate) job_type: CronAddJobType,
    pub(crate) schedule_kind: CronAddScheduleKind,
    pub(crate) schedule: String,
    pub(crate) at: String,
    pub(crate) every_ms: String,
    pub(crate) command: String,
    pub(crate) command_editor: text_editor::Content,
    pub(crate) prompt: String,
    pub(crate) prompt_editor: text_editor::Content,
    pub(crate) session_target: String,
    pub(crate) agent: String,
    pub(crate) acp_agent: String,
    pub(crate) project_path: String,
    pub(crate) model_provider: String,
    pub(crate) model: String,
    pub(crate) wake: bool,
    pub(crate) fallbacks: String,
    pub(crate) full_access: bool,
    pub(crate) task_pool: bool,
    pub(crate) delivery_enabled: bool,
    pub(crate) delivery_channel: String,
    pub(crate) delivery_to: String,
    pub(crate) delivery_best_effort: bool,
    pub(crate) delete_after_run: bool,
}

impl Default for CronJobDraft {
    fn default() -> Self {
        Self {
            name: String::new(),
            job_type: CronAddJobType::default(),
            schedule_kind: CronAddScheduleKind::default(),
            schedule: String::new(),
            at: String::new(),
            every_ms: String::new(),
            command: String::new(),
            command_editor: text_editor::Content::new(),
            prompt: String::new(),
            prompt_editor: text_editor::Content::new(),
            session_target: "isolated".to_string(),
            agent: "main".to_string(),
            acp_agent: String::new(),
            project_path: String::new(),
            model_provider: String::new(),
            model: String::new(),
            wake: false,
            fallbacks: String::new(),
            full_access: false,
            task_pool: false,
            delivery_enabled: false,
            delivery_channel: String::new(),
            delivery_to: String::new(),
            delivery_best_effort: true,
            delete_after_run: false,
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct CronSettingsState {
    /// 是否启用定时任务
    pub(crate) enabled: bool,
    /// 最大运行历史记录数
    pub(crate) max_run_history: u32,
    /// 当前 Cron 设置页签。
    pub(crate) active_tab: CronSettingsTab,
    /// 当前任务列表加载状态。
    pub(crate) jobs_loading: bool,
    /// 当前任务列表。
    pub(crate) jobs: Vec<vw_gateway_client::CronJobDto>,
    /// 被批量操作选中的任务 ID。
    pub(crate) selected_job_ids: Vec<String>,
    /// 当前正在编辑的任务 ID。
    pub(crate) editing_job_id: Option<String>,
    /// 编辑表单草稿。
    pub(crate) edit_draft: CronJobDraft,
    /// 新增表单草稿。
    pub(crate) add_draft: CronJobDraft,
    /// 正在查看历史记录的任务 ID。
    pub(crate) runs_modal_job_id: Option<String>,
    /// 历史记录加载状态。
    pub(crate) runs_modal_loading: bool,
    /// 历史记录加载错误。
    pub(crate) runs_modal_error: Option<String>,
    /// 当前弹窗展示的历史记录。
    pub(crate) runs_modal: Vec<vw_gateway_client::CronRunDto>,
    /// 当前弹窗中用于选择复制的完整历史文本。
    pub(crate) runs_modal_editor: text_editor::Content,
    /// 是否显示帮助对话框
    pub(crate) show_help_modal: bool,
    /// 保存错误信息
    pub(crate) save_error: Option<String>,
    /// 最近一次任务操作提示。
    pub(crate) action_status: Option<String>,
}

impl Default for CronSettingsState {
    fn default() -> Self {
        Self {
            enabled: true,
            max_run_history: 50,
            active_tab: CronSettingsTab::default(),
            jobs_loading: false,
            jobs: Vec::new(),
            selected_job_ids: Vec::new(),
            editing_job_id: None,
            edit_draft: CronJobDraft::default(),
            add_draft: CronJobDraft::default(),
            runs_modal_job_id: None,
            runs_modal_loading: false,
            runs_modal_error: None,
            runs_modal: Vec::new(),
            runs_modal_editor: text_editor::Content::new(),
            show_help_modal: false,
            save_error: None,
            action_status: None,
        }
    }
}

/// 调度器设置面板状态
///
/// 管理任务调度器的配置，包括并发限制等。
#[derive(Debug, Clone)]
pub(crate) struct SchedulerSettingsState {
    /// 是否启用调度器
    pub(crate) enabled: bool,
    /// 最大任务数
    pub(crate) max_tasks: u32,
    /// 最大并发数
    pub(crate) max_concurrent: u32,
    /// 是否显示帮助对话框
    pub(crate) show_help_modal: bool,
    /// 保存错误信息
    pub(crate) save_error: Option<String>,
}

impl Default for SchedulerSettingsState {
    fn default() -> Self {
        Self {
            enabled: true,
            max_tasks: 64,
            max_concurrent: 4,
            show_help_modal: false,
            save_error: None,
        }
    }
}

#[cfg(test)]
#[path = "provider_model_tests.rs"]
mod provider_model_tests;
