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
pub(crate) use channel_support::config;
/// 消息格式化工具
pub(crate) use channel_runtime::format;
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
    build_shell_policy_instructions, scrub_credentials,
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

pub use channel_runtime::ChannelCommands;

pub(crate) use channel_support::routing::ChannelRouteSelection;

pub(crate) use channel_support::config::{
    ChannelRuntimeDefaults, ConfigFileStamp, RuntimeAutonomyPolicy, RuntimeConfigState,
};

pub use channel_support::runtime_state::ChannelRuntimeContext;

pub(crate) use channel_support::scope::{InFlightSenderTaskState, InFlightTaskCompletion};

pub(crate) use channel_runtime::ChannelHealthState;

pub(crate) use channel_runtime::ConfiguredChannel;

// ============================================================================
// 辅助函数
// ============================================================================

#[cfg(not(target_arch = "wasm32"))]
use channel_support::blocking::block_on_future;





#[cfg(test)]
include!("scope_delivery_tests.rs");

#[cfg(test)]
#[path = "../../../libs/channel-runtime/src/runtime_command/project_commands_tests.rs"]
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
#[path = "dispatch_tests.rs"]
mod dispatch_tests;

#[cfg(test)]
#[path = "dispatch_cancel_tests.rs"]
mod channel_command_cancel_dispatch_tests;

#[cfg(test)]
include!("tests/message_dispatch.rs");

mod session_turn;
