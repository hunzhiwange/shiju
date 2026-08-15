//! Telegram 通道模块
//!
//! 本模块实现了 Telegram Bot API 的集成，提供完整的消息收发、媒体处理和交互功能。
//!
//! # 主要功能
//!
//! - **消息收发**：通过长轮询机制从 Telegram Bot API 获取更新，支持文本和媒体消息的发送
//! - **媒体处理**：支持附件、图片、音频、视频等多种媒体类型的处理和转换
//! - **交互功能**：处理回调查询、消息反应、提及(@mention)等用户交互
//! - **配对机制**：实现安全配对流程，限制允许与机器人交互的用户
//! - **流式模式**：支持流式响应，实时更新消息内容
//!
//! # 架构设计
//!
//! 模块采用分层设计，将不同功能拆分到独立的子模块中：
//!
//! - `core`: 核心配置和常量定义
//! - `config`: 配置解析和验证
//! - `channel_impl`: Channel trait 的实现
//! - `incoming`: 接收消息的处理逻辑
//! - `outbound`: 发送消息的处理逻辑
//! - `sending_text`: 文本消息发送
//! - `sending_media`: 媒体消息发送
//! - `attachments`: 附件处理
//! - `attachments_incoming`: 接收的附件处理
//! - `attachments_outbound`: 发送的附件处理
//! - `callbacks`: 回调查询处理
//! - `reactions`: 消息反应处理
//! - `mentions`: 提及(@mention)处理
//! - `pairing`: 配对流程
//! - `polling`: 长轮询机制
//! - `draft`: 草稿消息管理
//! - `file_io`: 文件 I/O 操作
//! - `formatting`: 消息格式化
//! - `message_utils`: 消息工具函数
//! - `tool_tags`: 工具标签处理
//!
//! # 使用示例
//!
//! ```no_run
//! use app::agent::channels::telegram::TelegramChannel;
//! use app::agent::channels::Channel;
//!
//! // 创建 Telegram 通道实例
//! let channel = TelegramChannel::new(/* 配置参数 */);
//!
//! // 发送消息
//! channel.send("chat_id", "Hello, Telegram!").await?;
//!
//! // 启动轮询监听
//! channel.listen().await?;
//! ```

use parking_lot::Mutex;
use std::sync::{Arc, RwLock};

mod attachments;
mod attachments_incoming;
mod attachments_outbound;
mod callbacks;
mod channel_impl;
mod config;
mod core;
mod draft;
mod file_io;
mod formatting;
mod incoming;
mod mentions;
mod message_utils;
mod outbound;
mod pairing;
mod polling;
mod reactions;
mod sending_media;
mod sending_text;
mod tool_tags;

/// Telegram 通道实现
///
/// 通过长轮询 Telegram Bot API 获取更新，实现与 Telegram 用户的实时交互。
/// 该结构体封装了与 Telegram Bot API 通信所需的所有状态和配置。
///
/// # 核心特性
///
/// - 支持文本、媒体、语音等多种消息类型
/// - 提供安全的用户访问控制（通过 `allowed_users` 和 `pairing`）
/// - 支持流式响应模式，实时更新消息内容
/// - 支持群组消息处理，包括提及(@mention)和回复控制
///
/// # 线程安全
///
/// 使用 `Mutex` 和 `RwLock` 保证多线程环境下的安全访问：
/// - `allowed_users`: 使用 `RwLock` 允许多个读取者同时访问
/// - `typing_handle`: 使用 `Mutex` 保护异步任务句柄
/// - `last_draft_edit`: 使用 `Mutex` 保护草稿编辑时间戳
/// - `bot_username`: 使用 `Mutex` 缓存机器人用户名
///
/// # 示例
///
/// ```no_run
/// // 创建 Telegram 通道
/// let channel = TelegramChannel {
///     bot_token: "123456:ABC-DEF".to_string(),
///     allowed_users: Arc::new(RwLock::new(vec!["user1".to_string()])),
///     // ... 其他字段
/// };
/// ```
pub struct TelegramChannel {
    /// Telegram Bot 的访问令牌
    ///
    /// 从 BotFather 获取的唯一标识符，用于认证所有 API 请求。
    /// 格式通常为：`<bot_id>:<token>`
    bot_token: String,

    /// 允许与机器人交互的用户白名单
    ///
    /// 使用 `RwLock` 包装以支持动态更新和并发读取。
    /// 只有在此列表中的用户才能发送消息给机器人。
    /// 空列表表示允许所有用户（不推荐用于生产环境）。
    allowed_users: Arc<RwLock<Vec<String>>>,

    /// 配对保护机制
    ///
    /// 实现安全配对流程，要求用户先完成配对才能与机器人交互。
    /// `None` 表示禁用配对机制（不推荐）。
    pairing: Option<crate::app::agent::security::pairing::PairingGuard>,

    /// HTTP 客户端
    ///
    /// 用于向 Telegram Bot API 发送 HTTP 请求。
    /// 复用连接池以提高性能。
    client: reqwest::Client,

    /// "正在输入"状态的任务句柄
    ///
    /// 用于管理发送"typing"聊天动作的异步任务。
    /// 使用 `Mutex` 保证线程安全的访问和更新。
    typing_handle: Mutex<Option<tokio::task::JoinHandle<()>>>,

    /// 流式响应模式配置
    ///
    /// 控制是否启用流式响应，以及流式响应的行为。
    /// 启用后，消息将以增量方式实时更新，而非一次性发送完整内容。
    stream_mode: crate::app::agent::config::StreamMode,

    /// 草稿消息的更新间隔（毫秒）
    ///
    /// 在流式模式下，控制消息编辑的频率，避免过于频繁地调用 API。
    /// 默认值通常为 500-1000 毫秒。
    draft_update_interval_ms: u64,

    /// 草稿消息的最后编辑时间
    ///
    /// 记录每个聊天的最后编辑时间，用于实现节流控制。
    /// Key: 聊天 ID，Value: 最后编辑时间戳
    last_draft_edit: Mutex<std::collections::HashMap<String, std::time::Instant>>,

    /// 是否仅在提及时响应
    ///
    /// 在群组中，如果为 `true`，则只在机器人被 @mention 时才响应消息。
    /// 适用于避免在活跃群组中产生过多噪音。
    mention_only: bool,

    /// 群组中允许回复的消息发送者 ID 列表
    ///
    /// 在群组环境中，机器人只会回复这些发送者的消息。
    /// 用于限制群组中的交互范围。
    group_reply_allowed_sender_ids: Vec<String>,

    /// 缓存的机器人用户名
    ///
    /// 通过 Bot API 获取并缓存，用于提及(@mention)检测。
    /// 使用 `Mutex` 实现懒加载和线程安全。
    bot_username: Mutex<Option<String>>,

    /// Telegram Bot API 的基础 URL
    ///
    /// 默认为 `https://api.telegram.org`。
    /// 可覆盖此值以使用本地 Bot API 服务器或测试环境。
    api_base: String,

    /// 工作空间目录路径
    ///
    /// 用于存储下载的文件、临时数据等。
    /// `None` 表示使用系统默认临时目录。
    workspace_dir: Option<std::path::PathBuf>,
}

#[cfg(test)]
#[path = "attachments_incoming_tests.rs"]
mod attachments_incoming_tests;
#[cfg(test)]
#[path = "attachments_outbound_tests.rs"]
mod attachments_outbound_tests;
#[cfg(test)]
#[path = "attachments_tests.rs"]
mod attachments_tests;
#[cfg(test)]
#[path = "callbacks_tests.rs"]
mod callbacks_tests;
#[cfg(test)]
#[path = "channel_impl_tests.rs"]
mod channel_impl_tests;
#[cfg(test)]
#[path = "config_tests.rs"]
mod config_tests;
#[cfg(test)]
#[path = "core_tests.rs"]
mod core_tests;
#[cfg(test)]
#[path = "draft_tests.rs"]
mod draft_tests;
#[cfg(test)]
#[path = "file_io_tests.rs"]
mod file_io_tests;
#[cfg(test)]
#[path = "formatting_tests.rs"]
mod formatting_tests;
#[cfg(test)]
#[path = "incoming_tests.rs"]
mod incoming_tests;
#[cfg(test)]
#[path = "mentions_tests.rs"]
mod mentions_tests;
#[cfg(test)]
#[path = "message_utils_tests.rs"]
mod message_utils_tests;
#[cfg(test)]
#[path = "outbound_tests.rs"]
mod outbound_tests;
#[cfg(test)]
#[path = "pairing_tests.rs"]
mod pairing_tests;
#[cfg(test)]
#[path = "polling_tests.rs"]
mod polling_tests;
#[cfg(test)]
#[path = "reactions_tests.rs"]
mod reactions_tests;
#[cfg(test)]
#[path = "sending_media_tests.rs"]
mod sending_media_tests;
#[cfg(test)]
#[path = "sending_text_tests.rs"]
mod sending_text_tests;
#[cfg(test)]
mod tests;
#[cfg(test)]
#[path = "tool_tags_tests.rs"]
mod tool_tags_tests;
