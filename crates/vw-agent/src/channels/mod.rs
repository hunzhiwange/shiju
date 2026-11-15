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
pub(crate) mod markdown_chunk;
/// Mattermost 通道实现（非 WASM 平台）
#[cfg(not(target_arch = "wasm32"))]
pub mod mattermost;
/// Nextcloud Talk 通道实现（非 WASM 平台）
#[cfg(not(target_arch = "wasm32"))]
pub mod nextcloud_talk;
/// Nostr 通道实现（非 WASM 平台）
#[cfg(not(target_arch = "wasm32"))]
pub mod nostr;
pub(crate) mod outbound_delivery;
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
pub use channel_nostr::NostrChannel;
#[cfg(not(target_arch = "wasm32"))]
pub use qq::QQChannel;
#[cfg(not(target_arch = "wasm32"))]
pub use signal::SignalChannel;
#[cfg(not(target_arch = "wasm32"))]
pub use channel_slack::SlackChannel;
#[cfg(not(target_arch = "wasm32"))]
pub use telegram::TelegramChannel;
pub use channel_types::{Channel, SendMessage};
#[cfg(not(target_arch = "wasm32"))]
pub use channel_wati::WatiChannel;
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
mod http;
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

use crate::app::agent::agent::loop_::{
    build_shell_policy_instructions, build_tool_instructions_from_specs, scrub_credentials,
};
use crate::app::agent::approval::{ApprovalManager, ApprovalResponse, PendingApprovalError};
use crate::app::agent::config::schema::{apply_env_overrides, save_config};
use crate::app::agent::config::{Config, NonCliNaturalLanguageApprovalMode};
use crate::app::agent::memory::{self, Memory};
use crate::app::agent::observability::{self, Observer, runtime_trace};
use crate::app::agent::provider::provider;
use model_types::{ChatMessage, Provider};
use crate::app::agent::runtime;
use crate::app::agent::security::SecurityPolicy;
use crate::app::agent::tools::Tool;
use crate::app::agent::util::truncate_with_ellipsis;
use anyhow::{Context, Result};
use clap::Subcommand;
use std::collections::{HashMap, HashSet};
use std::fmt::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};
use tokio_util::sync::CancellationToken;

// ============================================================================
// 类型别名定义
// ============================================================================

// ============================================================================
// 常量定义
// ============================================================================

/// 自动保存到记忆的最小用户消息长度（字符数）。
///
/// 短于此长度的消息（如 "好的"、"谢谢"）不会被存储，
/// 以减少记忆检索时的噪音。此设计避免将无意义的简短回复
/// 污染长期记忆存储。
const AUTOSAVE_MIN_MESSAGE_CHARS: usize = 20;

/// 通道初始重连退避时间（秒）。
/// 连接失败后首次重试前的等待时间。
const DEFAULT_CHANNEL_INITIAL_BACKOFF_SECS: u64 = 2;

/// 通道最大重连退避时间（秒）。
/// 指数退避算法的上限，避免无限增长导致过长的等待。
const DEFAULT_CHANNEL_MAX_BACKOFF_SECS: u64 = 60;

/// 处理单个通道消息的默认超时时间（秒）。
///
/// 包含 LLM 调用和工具执行的完整周期。
/// 当 `channels_config.message_timeout_secs` 未配置时作为回退值。
const CHANNEL_MESSAGE_TIMEOUT_SECS: u64 = 300;

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

pub(crate) use channel_support::routing::ChannelRouteSelection;

pub(crate) use channel_support::config::{
    ChannelRuntimeDefaults, ConfigFileStamp, RuntimeAutonomyPolicy, RuntimeConfigState,
};

pub use channel_support::runtime_state::ChannelRuntimeContext;

pub(crate) use channel_support::scope::{InFlightSenderTaskState, InFlightTaskCompletion};

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

#[cfg(test)]
#[path = "runtime_command/command_tests.rs"]
mod channel_command_catalog_tests;

#[cfg(test)]
include!("channel_session_lifecycle_tests.rs");

#[cfg(test)]
#[path = "runtime_command/history_commands_tests.rs"]
mod channel_session_history_tests;

#[cfg(test)]
#[path = "manager/dispatch_tests.rs"]
mod dispatch_tests;

#[cfg(test)]
#[path = "manager/dispatch_cancel_tests.rs"]
mod channel_command_cancel_dispatch_tests;

#[cfg(test)]
include!("tests/message_dispatch.rs");

mod session_turn;

#[cfg(test)]
use channel_support::routing::{MODEL_CACHE_FILE, MODEL_CACHE_PREVIEW_LIMIT};

#[cfg(test)]
use channel_support::prompt::{MEMORY_CONTEXT_MAX_CHARS, MEMORY_CONTEXT_ENTRY_MAX_CHARS};
