//! 通道运行时共享句柄契约；不包含会话执行器。
use std::{sync::{Arc, Mutex}, collections::HashMap, path::PathBuf};
use crate::{scope::{ChannelProjectState, ChannelMessageSnapshot}, routing::ChannelRouteSelection};
use channel_types::Channel;
use model_types::{Provider, ChatMessage};
use agent_memory::Memory;
use agent_observability::Observer;
use tool_types::Tool;
use agent_approval::ApprovalManager;
pub type RouteSelectionMap = Arc<Mutex<HashMap<String, ChannelRouteSelection>>>;

pub type ProviderCacheMap = Arc<Mutex<HashMap<String, Arc<dyn Provider>>>>;

pub type ConversationHistoryMap = Arc<Mutex<HashMap<String, Vec<ChatMessage>>>>;

/// 通道运行时上下文。
///
/// 包含处理消息所需的全部共享状态和配置，
/// 在消息处理循环中被克隆和传递。
/// 使用 `Arc` 包装以实现高效的共享所有权。
#[derive(Clone)]
pub struct ChannelRuntimeContext {
    pub project_state: Arc<Mutex<ChannelProjectState>>,
    pub message_snapshot: Option<ChannelMessageSnapshot>,
    /// Webhook 保留网关的受监督工具权限；监听通道沿用既有会话权限。
    pub session_full_access: bool,
    /// 按名称索引的通道实例映射
    pub channels_by_name: Arc<HashMap<String, Arc<dyn Channel>>>,
    /// 当前使用的服务提供者
    pub provider: Arc<dyn Provider>,
    /// 默认提供者名称
    pub default_provider: Arc<String>,
    /// 记忆存储后端
    pub memory: Arc<dyn Memory>,
    /// 已注册工具的注册表
    pub tools_registry: Arc<Vec<Box<dyn Tool>>>,
    /// 可观测性观察者
    pub observer: Arc<dyn Observer>,
    /// 系统提示词
    pub system_prompt: Arc<String>,
    /// 当前使用的模型标识符
    pub model: Arc<String>,
    /// 生成温度参数
    pub temperature: f64,
    /// 是否启用自动记忆保存
    pub auto_save_memory: bool,
    /// 单次请求中工具迭代的最大次数
    pub max_iterations: usize,
    /// 记忆检索的最小相关性分数阈值
    pub min_relevance_score: f64,
    /// 各发送者的会话历史
    pub conversation_histories: ConversationHistoryMap,
    /// Provider 实例缓存
    pub provider_cache: ProviderCacheMap,
    /// 路由覆盖配置
    pub route_overrides: RouteSelectionMap,
    /// 可选的 API 密钥
    pub api_key: Option<String>,
    /// 可选的 API 端点 URL
    pub api_url: Option<String>,
    /// 可靠性配置
    pub reliability: Arc<agent_config::runtime_schema::ReliabilityConfig>,
    /// Provider 运行时选项
    pub provider_runtime_options: agent_providers::ProviderRuntimeOptions,
    /// 工作区目录路径
    pub workspace_dir: Arc<PathBuf>,
    /// 消息处理超时时间（秒）
    pub message_timeout_secs: u64,
    /// 是否在新消息到达时中断当前处理
    pub interrupt_on_new_message: bool,
    /// 多模态配置
    pub multimodal: agent_config::runtime_schema::MultimodalConfig,
    /// 可选的 Hook 运行器
    pub hooks: Option<Arc<agent_hooks::HookRunner>>,
    /// 非 CLI 环境下排除的工具列表（运行时可变）
    pub non_cli_excluded_tools: Arc<Mutex<Vec<String>>>,
    /// 查询分类配置
    pub query_classification: agent_config::runtime_schema::QueryClassificationConfig,
    /// 模型路由配置列表
    pub model_routes: Vec<agent_config::runtime_schema::ModelRouteConfig>,
    /// 审批管理器
    pub approval_manager: Arc<ApprovalManager>,
}
