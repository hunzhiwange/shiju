//! 通道子系统 —— 消息平台集成的基础设施。
//!
//! 本模块提供多通道消息基础设施，将 VibeWindow 连接到外部平台。
//! 每个通道都实现了 [`traits`] 中定义的 [`Channel`] trait，该 trait
//! 提供了统一的接口用于：发送消息、监听传入消息、健康检查和打字指示器。
//!
//! 通道实例由 [`start_channels`] 根据运行时配置创建。该子系统管理
//! 每个发送者的会话历史、可配置并行度的并发消息处理，以及指数退避重连机制以保证弹性。
//!
//! # 扩展指南
//!
//! 添加新通道的步骤：
//! 1. 在新的子模块中实现 [`Channel`] trait
//! 2. 在 [`start_channels`] 中注册该通道
//! 3. 完整变更手册参见 `AGENTS.md` §7.2

// ============================================================================
// 子模块声明
// ============================================================================

/// ClawdTalk 通道实现
pub mod clawdtalk;
/// CLI 命令行通道实现
pub mod cli;
/// 钉钉通道实现（非 WASM 平台）
#[cfg(not(target_arch = "wasm32"))]
pub mod dingtalk;
/// Discord 通道实现（非 WASM 平台）
#[cfg(not(target_arch = "wasm32"))]
pub mod discord;
/// 邮件通道实现（非 WASM 平台）
#[cfg(not(target_arch = "wasm32"))]
pub mod email_channel;
/// iMessage 通道实现（非 WASM 平台）
#[cfg(not(target_arch = "wasm32"))]
pub mod imessage;
/// IRC 通道实现（非 WASM 平台）
#[cfg(not(target_arch = "wasm32"))]
pub mod irc;
/// 飞书/Lark 通道实现（需启用 channel-lark 特性）
#[cfg(feature = "channel-lark")]
pub mod lark;
/// Linq 通道实现（非 WASM 平台）
#[cfg(not(target_arch = "wasm32"))]
pub mod linq;
/// Mattermost 通道实现（非 WASM 平台）
#[cfg(not(target_arch = "wasm32"))]
pub mod mattermost;
/// Nextcloud Talk 通道实现（非 WASM 平台）
#[cfg(not(target_arch = "wasm32"))]
pub mod nextcloud_talk;
/// Nostr 通道实现（非 WASM 平台）
#[cfg(not(target_arch = "wasm32"))]
pub mod nostr;
/// QQ 通道实现（非 WASM 平台）
#[cfg(not(target_arch = "wasm32"))]
pub mod qq;
/// Signal 通道实现（非 WASM 平台）
#[cfg(not(target_arch = "wasm32"))]
pub mod signal;
/// Slack 通道实现（非 WASM 平台）
#[cfg(not(target_arch = "wasm32"))]
pub mod slack;
/// Telegram 通道实现（非 WASM 平台）
#[cfg(not(target_arch = "wasm32"))]
pub mod telegram;
/// Channel trait 定义
pub mod traits;
/// WATI 通道实现（非 WASM 平台）
#[cfg(not(target_arch = "wasm32"))]
pub mod wati;
/// 微信 iLink 机器人通道实现（非 WASM 平台）
#[cfg(not(target_arch = "wasm32"))]
pub mod wechat;
/// 企业微信 AI 机器人通道实现（非 WASM 平台）
#[cfg(not(target_arch = "wasm32"))]
pub mod wecom;
/// WhatsApp 通道实现（非 WASM 平台）
#[cfg(not(target_arch = "wasm32"))]
pub mod whatsapp;
/// WhatsApp 存储模块（需启用 whatsapp-web 特性，非 WASM 平台）
#[cfg(all(feature = "whatsapp-web", not(target_arch = "wasm32")))]
pub mod whatsapp_storage;
/// WhatsApp Web 通道实现（需启用 whatsapp-web 特性，非 WASM 平台）
#[cfg(all(feature = "whatsapp-web", not(target_arch = "wasm32")))]
pub mod whatsapp_web;

// ============================================================================
// 公开类型重导出
// ============================================================================

#[cfg(not(target_arch = "wasm32"))]
pub use clawdtalk::ClawdTalkChannel;
pub use cli::CliChannel;
#[cfg(not(target_arch = "wasm32"))]
pub use dingtalk::DingTalkChannel;
#[cfg(not(target_arch = "wasm32"))]
pub use discord::DiscordChannel;
#[cfg(not(target_arch = "wasm32"))]
pub use email_channel::EmailChannel;
#[cfg(not(target_arch = "wasm32"))]
pub use imessage::IMessageChannel;
#[cfg(not(target_arch = "wasm32"))]
pub use irc::IrcChannel;
#[cfg(feature = "channel-lark")]
pub use lark::LarkChannel;
#[cfg(not(target_arch = "wasm32"))]
pub use linq::LinqChannel;
#[cfg(not(target_arch = "wasm32"))]
pub use mattermost::MattermostChannel;
#[cfg(not(target_arch = "wasm32"))]
pub use nextcloud_talk::NextcloudTalkChannel;
#[cfg(not(target_arch = "wasm32"))]
pub use nostr::NostrChannel;
#[cfg(not(target_arch = "wasm32"))]
pub use qq::QQChannel;
#[cfg(not(target_arch = "wasm32"))]
pub use signal::SignalChannel;
#[cfg(not(target_arch = "wasm32"))]
pub use slack::SlackChannel;
#[cfg(not(target_arch = "wasm32"))]
pub use telegram::TelegramChannel;
pub use traits::{Channel, SendMessage};
#[cfg(not(target_arch = "wasm32"))]
pub use wati::WatiChannel;
#[cfg(not(target_arch = "wasm32"))]
pub use wechat::WeChatChannel;
#[cfg(not(target_arch = "wasm32"))]
pub use wecom::WeComChannel;
#[cfg(not(target_arch = "wasm32"))]
pub use whatsapp::WhatsAppChannel;
#[cfg(all(feature = "whatsapp-web", not(target_arch = "wasm32")))]
pub use whatsapp_web::WhatsAppWebChannel;

// ============================================================================
// 内部子模块
// ============================================================================

/// 通道配置处理
pub(crate) mod config;
/// 消息格式化工具
pub(crate) mod format;
/// 会话历史管理
pub(crate) mod history;
/// 通道管理器
pub(crate) mod manager;
/// 跨平台出站消息格式规整
pub(crate) mod outbound_format;
/// 提示词构建
pub(crate) mod prompt;
/// 消息路由逻辑
pub(crate) mod routing;
/// 运行时命令处理
pub(crate) mod runtime_command;
pub(crate) mod scope;
/// 会话状态管理
pub(crate) mod session;

// ============================================================================
// 外部依赖导入
// ============================================================================

use crate::app::agent::memory::MemoryCategory;
use crate::app::agent::skills;
pub(crate) use config::*;
pub(crate) use format::*;
pub(crate) use history::*;
pub use manager::*;
pub use prompt::*;
pub(crate) use routing::*;
pub(crate) use runtime_command::*;
pub(crate) use scope::*;
pub(crate) use session::*;

#[cfg(test)]
#[path = "mod_tests.rs"]
mod mod_tests;
#[cfg(test)]
#[path = "outbound_format_tests.rs"]
mod outbound_format_tests;

use crate::app::agent::agent::loop_::{
    build_shell_policy_instructions, build_tool_instructions_from_specs, scrub_credentials,
};
use crate::app::agent::approval::{ApprovalManager, ApprovalResponse, PendingApprovalError};
use crate::app::agent::config::schema::{apply_env_overrides, save_config};
use crate::app::agent::config::{Config, NonCliNaturalLanguageApprovalMode};
use crate::app::agent::memory::{self, Memory};
use crate::app::agent::observability::{self, Observer, runtime_trace};
use crate::app::agent::provider::provider;
use crate::app::agent::providers::{ChatMessage, Provider};
use crate::app::agent::runtime;
use crate::app::agent::security::SecurityPolicy;
use crate::app::agent::tools::Tool;
use crate::app::agent::util::truncate_with_ellipsis;
use anyhow::{Context, Result};
use clap::Subcommand;
use serde::Deserialize;
use std::collections::{HashMap, HashSet};
use std::fmt::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant, SystemTime};
use tokio_util::sync::CancellationToken;

// ============================================================================
// 类型别名定义
// ============================================================================

/// 每个发送者的会话历史映射表。
///
/// 外层 `Arc` 允许多个任务共享同一份历史数据，
/// 内层 `Mutex` 保证并发安全访问，
/// `HashMap` 的键为发送者标识符，值为该发送者的消息历史列表。
type ConversationHistoryMap = Arc<Mutex<HashMap<String, Vec<ChatMessage>>>>;

/// Provider 缓存映射表。
/// 键为 provider 名称，值为对应的 Provider 实例。
type ProviderCacheMap = Arc<Mutex<HashMap<String, Arc<dyn Provider>>>>;

/// 路由选择映射表。
/// 键为路由标识，值为对应的路由选择配置。
type RouteSelectionMap = Arc<Mutex<HashMap<String, ChannelRouteSelection>>>;

// ============================================================================
// 常量定义
// ============================================================================

/// 每个发送者保留的最大历史消息数量。
/// 超过此数量的旧消息将被压缩或丢弃，以控制内存使用。
const MAX_CHANNEL_HISTORY: usize = 50;

/// 自动保存到记忆的最小用户消息长度（字符数）。
///
/// 短于此长度的消息（如 "好的"、"谢谢"）不会被存储，
/// 以减少记忆检索时的噪音。此设计避免将无意义的简短回复
/// 污染长期记忆存储。
const AUTOSAVE_MIN_MESSAGE_CHARS: usize = 20;

/// 每个注入的工作区文件的最大字符数。
/// 与 `OpenClaw` 默认值保持一致，防止单个文件占用过多上下文空间。
const BOOTSTRAP_MAX_CHARS: usize = 20_000;

/// 通道初始重连退避时间（秒）。
/// 连接失败后首次重试前的等待时间。
const DEFAULT_CHANNEL_INITIAL_BACKOFF_SECS: u64 = 2;

/// 通道最大重连退避时间（秒）。
/// 指数退避算法的上限，避免无限增长导致过长的等待。
const DEFAULT_CHANNEL_MAX_BACKOFF_SECS: u64 = 60;

/// 通道消息处理的最小超时时间（秒）。
/// 即使配置了更小的值，也不会低于此下限。
const MIN_CHANNEL_MESSAGE_TIMEOUT_SECS: u64 = 30;

/// 处理单个通道消息的默认超时时间（秒）。
///
/// 包含 LLM 调用和工具执行的完整周期。
/// 当 `channels_config.message_timeout_secs` 未配置时作为回退值。
const CHANNEL_MESSAGE_TIMEOUT_SECS: u64 = 300;

/// 超时缩放上限系数。
/// 防止过大的 `max_iterations` 配置导致无界等待。
/// 实际超时 = 基础超时 × min(缩放系数, 此上限)
const CHANNEL_MESSAGE_TIMEOUT_SCALE_CAP: u64 = 4;

/// 每个通道的并行处理消息数。
/// 单个通道实例可同时处理的消息上限。
const CHANNEL_PARALLELISM_PER_CHANNEL: usize = 4;

/// 最小同时处理消息数。
/// 系统保证至少能同时处理此数量的消息。
const CHANNEL_MIN_IN_FLIGHT_MESSAGES: usize = 8;

/// 最大同时处理消息数。
/// 系统同时处理消息的上限，防止资源耗尽。
const CHANNEL_MAX_IN_FLIGHT_MESSAGES: usize = 64;

/// 打字指示器刷新间隔（秒）。
/// 在长时间处理过程中，定期发送打字状态以保持用户连接。
const CHANNEL_TYPING_REFRESH_INTERVAL_SECS: u64 = 4;

/// 健康检查心跳间隔（秒）。
/// 定期检查通道连接状态的频率。
const CHANNEL_HEALTH_HEARTBEAT_SECS: u64 = 30;

/// 模型缓存文件名。
/// 存储已发现的模型列表，避免每次启动都重新查询。
const MODEL_CACHE_FILE: &str = "models_cache.json";

/// 模型缓存预览显示数量上限。
/// 在列出缓存模型时，最多显示的条目数。
const MODEL_CACHE_PREVIEW_LIMIT: usize = 10;

/// 记忆上下文最大条目数。
/// 注入到提示词中的相关记忆片段数量上限。
const MEMORY_CONTEXT_MAX_ENTRIES: usize = 4;

/// 单条记忆上下文条目的最大字符数。
/// 防止单条记忆过长影响上下文窗口。
const MEMORY_CONTEXT_ENTRY_MAX_CHARS: usize = 800;

/// 记忆上下文总字符数上限。
/// 所有注入记忆片段的总长度限制。
const MEMORY_CONTEXT_MAX_CHARS: usize = 4_000;

/// 历史压缩时保留的消息数量。
/// 压缩会话历史时，保留最近 N 条完整消息。
const CHANNEL_HISTORY_COMPACT_KEEP_MESSAGES: usize = 12;

/// 历史压缩时内容的字符数限制。
/// 被压缩的消息内容截断到此长度。
const CHANNEL_HISTORY_COMPACT_CONTENT_CHARS: usize = 600;

/// Hook 修改后的出站消息内容最大字符数。
/// 防止 Hook 产生过长的输出内容。
const CHANNEL_HOOK_MAX_OUTBOUND_CHARS: usize = 20_000;

/// 一次性批准所有工具的特殊令牌。
/// 用于审批流程中标记"本次会话批准所有工具"的状态。
const APPROVAL_ALL_TOOLS_ONCE_TOKEN: &str = "__all_tools_once__";

/// Systemd 服务状态检查参数。
/// 用于检查 vibewindow 服务的运行状态。
const SYSTEMD_STATUS_ARGS: [&str; 3] = ["--user", "is-active", "vibewindow.service"];

/// Systemd 服务重启参数。
/// 用于重启 vibewindow 服务。
const SYSTEMD_RESTART_ARGS: [&str; 3] = ["--user", "restart", "vibewindow.service"];

/// OpenRC 服务状态检查参数。
/// 用于在 OpenRC init 系统上检查服务状态。
const OPENRC_STATUS_ARGS: [&str; 2] = ["vibewindow", "status"];

/// OpenRC 服务重启参数。
/// 用于在 OpenRC init 系统上重启服务。
const OPENRC_RESTART_ARGS: [&str; 2] = ["vibewindow", "restart"];

// ============================================================================
// 结构体定义
// ============================================================================

/// 通道管理子命令枚举。
///
/// 与 CLI 枚举镜像，但避免 clap 依赖。
/// 用于运行时动态处理通道相关的管理命令。
#[derive(Debug, Clone, Subcommand)]
pub enum ChannelCommands {
    /// 列出所有已配置的通道
    List,
    /// 启动通道服务
    Start,
    /// 诊断通道配置问题
    Doctor,
    /// 添加新通道
    ///
    /// # 参数
    /// - `channel_type`: 通道类型标识符（如 "telegram"、"discord"）
    /// - `config`: 通道配置（JSON 格式字符串）
    Add { channel_type: String, config: String },
    /// 移除指定通道
    ///
    /// # 参数
    /// - `name`: 要移除的通道名称
    Remove { name: String },
    /// 绑定 Telegram 身份
    ///
    /// # 参数
    /// - `identity`: 身份标识符
    BindTelegram { identity: String },
}

/// 通道路由选择配置。
///
/// 记录为特定会话选择的服务提供者和模型信息，
/// 以及是否启用了任务模式。用于消息路由决策。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ChannelRouteSelection {
    /// 选中的服务提供者名称
    pub(crate) provider: String,
    /// 选中的模型标识符
    pub(crate) model: String,
    /// 是否启用了任务模式
    pub(crate) task_mode_enabled: bool,
}

/// 模型缓存状态。
///
/// 持久化存储已知模型的列表，避免每次启动时重新发现。
#[derive(Debug, Clone, Default, Deserialize)]
struct ModelCacheState {
    /// 缓存条目列表，按提供者分组
    pub(crate) entries: Vec<ModelCacheEntry>,
}

/// 模型缓存条目。
///
/// 记录单个提供者下的可用模型列表。
#[derive(Debug, Clone, Default, Deserialize)]
struct ModelCacheEntry {
    /// 提供者名称
    pub(crate) provider: String,
    /// 该提供者支持的模型列表
    pub(crate) models: Vec<String>,
}

/// 通道运行时默认配置。
///
/// 包含通道启动时所需的默认值，如默认提供者、模型、
/// 温度参数以及可靠性配置。
#[derive(Debug, Clone)]
pub(crate) struct ChannelRuntimeDefaults {
    /// 默认的服务提供者名称
    pub(crate) default_provider: String,
    /// 默认使用的模型标识符
    pub(crate) model: String,
    /// 生成温度参数（0.0-2.0）
    pub(crate) temperature: f64,
    /// 可选的 API 密钥
    pub(crate) api_key: Option<String>,
    /// 可选的 API 端点 URL
    pub(crate) api_url: Option<String>,
    /// 可靠性配置（重试、超时等）
    pub(crate) reliability: crate::app::agent::config::ReliabilityConfig,
}

/// 配置文件时间戳。
///
/// 用于检测配置文件是否已更改，
/// 通过修改时间和文件长度共同判断。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ConfigFileStamp {
    /// 文件最后修改时间
    pub(crate) modified: SystemTime,
    /// 文件字节长度
    pub(crate) len: u64,
}

/// 运行时配置状态。
///
/// 维护当前生效的配置及上次应用配置的时间戳，
/// 用于热重载时比较配置是否需要更新。
#[derive(Debug, Clone)]
pub(crate) struct RuntimeConfigState {
    /// 当前生效的运行时默认配置
    pub(crate) defaults: ChannelRuntimeDefaults,
    /// 上次应用配置时的文件时间戳
    pub(crate) last_applied_stamp: Option<ConfigFileStamp>,
}

/// 运行时自治策略。
///
/// 定义非 CLI 通道（如 Telegram、Discord）的工具审批行为，
/// 包括自动批准的工具列表、始终询问的工具列表，
/// 以及自然语言审批模式配置。
#[derive(Debug, Clone)]
pub(crate) struct RuntimeAutonomyPolicy {
    /// 自动批准的工具名称列表
    pub(crate) auto_approve: Vec<String>,
    /// 始终需要用户确认的工具名称列表
    pub(crate) always_ask: Vec<String>,
    /// 非 CLI 环境下排除的工具列表
    pub(crate) non_cli_excluded_tools: Vec<String>,
    /// 非 CLI 环境下有权审批的用户列表
    pub(crate) non_cli_approval_approvers: Vec<String>,
    /// 全局自然语言审批模式
    pub(crate) non_cli_natural_language_approval_mode: NonCliNaturalLanguageApprovalMode,
    /// 按通道细分的自然语言审批模式
    pub(crate) non_cli_natural_language_approval_mode_by_channel:
        HashMap<String, NonCliNaturalLanguageApprovalMode>,
}

/// 通道运行时上下文。
///
/// 包含处理消息所需的全部共享状态和配置，
/// 在消息处理循环中被克隆和传递。
/// 使用 `Arc` 包装以实现高效的共享所有权。
#[derive(Clone)]
pub(crate) struct ChannelRuntimeContext {
    pub(crate) project_state: Arc<Mutex<ChannelProjectState>>,
    pub(crate) message_snapshot: Option<ChannelMessageSnapshot>,
    /// 按名称索引的通道实例映射
    pub(crate) channels_by_name: Arc<HashMap<String, Arc<dyn Channel>>>,
    /// 当前使用的服务提供者
    pub(crate) provider: Arc<dyn Provider>,
    /// 默认提供者名称
    pub(crate) default_provider: Arc<String>,
    /// 记忆存储后端
    pub(crate) memory: Arc<dyn Memory>,
    /// 已注册工具的注册表
    pub(crate) tools_registry: Arc<Vec<Box<dyn Tool>>>,
    /// 可观测性观察者
    pub(crate) observer: Arc<dyn Observer>,
    /// 系统提示词
    pub(crate) system_prompt: Arc<String>,
    /// 当前使用的模型标识符
    pub(crate) model: Arc<String>,
    /// 生成温度参数
    pub(crate) temperature: f64,
    /// 是否启用自动记忆保存
    pub(crate) auto_save_memory: bool,
    /// 单次请求中工具迭代的最大次数
    pub(crate) max_iterations: usize,
    /// 记忆检索的最小相关性分数阈值
    pub(crate) min_relevance_score: f64,
    /// 各发送者的会话历史
    pub(crate) conversation_histories: ConversationHistoryMap,
    /// Provider 实例缓存
    pub(crate) provider_cache: ProviderCacheMap,
    /// 路由覆盖配置
    pub(crate) route_overrides: RouteSelectionMap,
    /// 可选的 API 密钥
    pub(crate) api_key: Option<String>,
    /// 可选的 API 端点 URL
    pub(crate) api_url: Option<String>,
    /// 可靠性配置
    pub(crate) reliability: Arc<crate::app::agent::config::ReliabilityConfig>,
    /// Provider 运行时选项
    pub(crate) provider_runtime_options: crate::app::agent::providers::ProviderRuntimeOptions,
    /// 工作区目录路径
    pub(crate) workspace_dir: Arc<PathBuf>,
    /// 消息处理超时时间（秒）
    pub(crate) message_timeout_secs: u64,
    /// 是否在新消息到达时中断当前处理
    pub(crate) interrupt_on_new_message: bool,
    /// 多模态配置
    pub(crate) multimodal: crate::app::agent::config::MultimodalConfig,
    /// 可选的 Hook 运行器
    pub(crate) hooks: Option<Arc<crate::app::agent::hooks::HookRunner>>,
    /// 非 CLI 环境下排除的工具列表（运行时可变）
    pub(crate) non_cli_excluded_tools: Arc<Mutex<Vec<String>>>,
    /// 查询分类配置
    pub(crate) query_classification: crate::app::agent::config::QueryClassificationConfig,
    /// 模型路由配置列表
    pub(crate) model_routes: Vec<crate::app::agent::config::ModelRouteConfig>,
    /// 审批管理器
    pub(crate) approval_manager: Arc<ApprovalManager>,
}

/// 正在处理中的发送者任务状态。
///
/// 跟踪每个消息处理任务的生命周期，
/// 支持取消和完成通知。
#[derive(Clone)]
pub(crate) struct InFlightSenderTaskState {
    /// 任务唯一标识符
    pub(crate) task_id: u64,
    /// 用于取消任务的令牌
    pub(crate) cancellation: CancellationToken,
    /// 任务完成状态通知器
    pub(crate) completion: Arc<InFlightTaskCompletion>,
}

/// 正在处理中的任务完成状态。
///
/// 提供原子完成标志和异步等待机制，
/// 允许其他任务等待当前任务完成。
pub(crate) struct InFlightTaskCompletion {
    /// 完成标志（原子布尔值）
    pub(crate) done: AtomicBool,
    /// 用于通知等待者的异步原语
    pub(crate) notify: tokio::sync::Notify,
}

impl InFlightTaskCompletion {
    /// 创建新的未完成状态实例。
    fn new() -> Self {
        Self { done: AtomicBool::new(false), notify: tokio::sync::Notify::new() }
    }

    /// 标记任务为已完成。
    ///
    /// 设置完成标志并通知所有等待者。
    fn mark_done(&self) {
        self.done.store(true, Ordering::Release);
        self.notify.notify_waiters();
    }

    /// 异步等待任务完成。
    ///
    /// 如果任务已完成则立即返回，否则阻塞等待通知。
    async fn wait(&self) {
        // 快速路径：已完成后直接返回
        if self.done.load(Ordering::Acquire) {
            return;
        }
        // 慢速路径：等待完成通知
        self.notify.notified().await;
    }
}

/// 通道健康状态枚举。
///
/// 表示通道连接的健康检查结果。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ChannelHealthState {
    /// 健康：通道正常工作
    Healthy,
    /// 不健康：通道存在问题
    Unhealthy,
    /// 超时：健康检查超时未响应
    Timeout,
}

/// 已配置的通道信息。
///
/// 包装通道实例及其人类可读的显示名称。
#[derive(Clone)]
pub(crate) struct ConfiguredChannel {
    /// 稳定实例 ID，独立于显示名称。
    pub(crate) instance_id: String,
    /// 用于日志和 UI 显示的名称
    pub(crate) display_name: String,
    /// 通道实例
    pub(crate) channel: Arc<dyn Channel>,
}

impl ConfiguredChannel {
    pub(crate) fn with_instance_id(mut self, instance_id: &str) -> Self {
        self.instance_id = instance_id.to_string();
        self
    }

    pub(crate) fn stamp_inbound_message(&self, message: &mut traits::ChannelMessage) {
        message.instance_id.clone_from(&self.instance_id);
    }

    /// 保持有界背压；监听退出前排空已接收消息，取消时不遗留转发任务。
    pub(crate) async fn listen(
        &self,
        tx: tokio::sync::mpsc::Sender<traits::ChannelMessage>,
    ) -> Result<()> {
        let (instance_tx, mut instance_rx) = tokio::sync::mpsc::channel(100);
        let listen = self.channel.listen(instance_tx);
        tokio::pin!(listen);
        loop {
            tokio::select! {
                biased;
                result = &mut listen => {
                    while let Ok(mut message) = instance_rx.try_recv() {
                        self.stamp_inbound_message(&mut message);
                        if tx.send(message).await.is_err() { break; }
                    }
                    return result;
                }
                message = instance_rx.recv() => {
                    let Some(mut message) = message else { return listen.await; };
                    self.stamp_inbound_message(&mut message);
                    if tx.send(message).await.is_err() { return Ok(()); }
                }
                _ = tx.closed() => return Ok(()),
            }
        }
    }

    pub(crate) fn new(display_name: impl Into<String>, channel: Arc<dyn Channel>) -> Self {
        Self { display_name: display_name.into(), channel, instance_id: "default".into() }
    }
}

// ============================================================================
// 辅助函数
// ============================================================================

/// 在同步上下文中阻塞执行异步 Future。
///
/// 此函数处理两种情况：
/// 1. 如果已经在 Tokio 运行时中，使用 `block_in_place` 在当前上下文中执行
/// 2. 如果不在运行时中，创建新的单线程运行时来执行
///
/// # 平台支持
/// 仅在非 WASM 目标平台上可用，因为 WASM 不支持阻塞操作。
///
/// # 参数
/// - `fut`: 要执行的异步 Future
///
/// # 返回值
/// Future 的输出结果
///
/// # Panic
/// 如果无法创建新的 Tokio 运行时会 panic（极少发生）
#[cfg(not(target_arch = "wasm32"))]
fn block_on_future<F: std::future::Future>(fut: F) -> F::Output {
    // 尝试获取当前运行时句柄
    if let Ok(handle) = tokio::runtime::Handle::try_current() {
        // 在现有运行时中使用 block_in_place 避免阻塞整个运行时
        return tokio::task::block_in_place(|| handle.block_on(fut));
    }
    // 不在运行时中，创建新的单线程运行时
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("failed to build tokio runtime")
        .block_on(fut)
}

#[cfg(test)]
mod tests;

#[cfg(test)]
#[path = "manager/channels_config_tests.rs"]
mod channel_project_binding_tests;

#[cfg(test)]
include!("channel_project_scope_tests.rs");

#[cfg(test)]
include!("scope_delivery_tests.rs");

#[cfg(test)]
#[path = "runtime_command/project_commands_tests.rs"]
mod channel_project_select_tests;
