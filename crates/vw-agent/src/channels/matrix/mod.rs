//! Matrix 频道模块
//!
//! 本模块实现了与 Matrix 协议的集成，通过 [matrix-sdk](https://crates.io/crates/matrix-sdk) 提供
//! 可靠的同步和加密房间解密能力。Matrix 是一个开放的去中心化通信协议，支持端到端加密(E2EE)。
//!
//! # 主要功能
//!
//! - 通过 Matrix Client-Server API 与 Matrix 服务器通信
//! - 支持房间别名和房间 ID 两种房间引用方式
//! - 支持端到端加密(E2EE)房间的消息解密
//! - 支持提及过滤模式（仅响应提及或私信）
//! - 支持用户访问控制（白名单）
//!
//! # 核心类型
//!
//! - [`MatrixChannel`]: 实现 Channel trait 的主要频道类型
//!
//! # 使用示例
//!
//! ```ignore
//! use crate::app::agent::channels::matrix::MatrixChannel;
//!
//! let channel = MatrixChannel::new(
//!     "https://matrix.org".to_string(),
//!     "access_token".to_string(),
//!     "!roomid:matrix.org".to_string(),
//!     vec!["@user:matrix.org".to_string()],
//! );
//! ```

use matrix_sdk::Client as MatrixSdkClient;
use reqwest::Client;
use std::path::PathBuf;
use std::sync::{Arc, atomic::AtomicBool};
use tokio::sync::{OnceCell, RwLock};

mod api;
mod channel_impl;
mod core;
mod listener;
mod outbound;
mod types;

pub(crate) use listener::MatrixAgentSchedulerDispatchTimelineEvent;

/// Matrix 频道实现
///
/// 通过 Matrix Client-Server API 与 Matrix 服务器通信，使用 matrix-sdk
/// 提供可靠的同步和加密房间解密能力。
///
/// # 字段说明
///
/// - `homeserver`: Matrix 主服务器 URL（如 `https://matrix.org`）
/// - `access_token`: 访问令牌，用于认证 API 请求
/// - `room_id`: 目标房间 ID 或别名（以 `!` 开头为 ID，以 `#` 开头为别名）
/// - `allowed_users`: 允许与此代理交互的用户白名单（`*` 表示允许所有用户）
/// - `mention_only`: 是否仅响应提及消息或私信
/// - `session_owner_hint`: 可选的用户 ID 提示，用于 E2EE 会话恢复
/// - `session_device_id_hint`: 可选的设备 ID 提示，用于 E2EE 会话恢复
/// - `vibewindow_dir`: VibeWindow 数据目录，用于存储 Matrix 加密状态
/// - `resolved_room_id_cache`: 已解析的房间 ID 缓存
/// - `sdk_client`: matrix-sdk 客户端实例的延迟初始化容器
/// - `otk_conflict_detected`: 一次性密钥冲突检测标志
/// - `http_client`: HTTP 客户端，用于直接 API 调用
#[derive(Clone)]
pub struct MatrixChannel {
    /// Matrix 主服务器 URL
    homeserver: String,
    /// 访问令牌
    access_token: String,
    /// 目标房间 ID 或别名
    room_id: String,
    /// 允许交互的用户白名单
    allowed_users: Vec<String>,
    /// 是否仅响应提及或私信
    mention_only: bool,
    /// 用户 ID 提示（用于 E2EE 会话恢复）
    session_owner_hint: Option<String>,
    /// 设备 ID 提示（用于 E2EE 会话恢复）
    session_device_id_hint: Option<String>,
    /// 运行时通道实例 ID
    instance_id: Option<String>,
    /// VibeWindow 数据目录
    vibewindow_dir: Option<PathBuf>,
    /// 已解析的房间 ID 缓存
    resolved_room_id_cache: Arc<RwLock<Option<String>>>,
    /// matrix-sdk 客户端实例
    sdk_client: Arc<OnceCell<MatrixSdkClient>>,
    /// 一次性密钥冲突检测标志
    otk_conflict_detected: Arc<AtomicBool>,
    /// HTTP 客户端
    http_client: Client,
}

impl std::fmt::Debug for MatrixChannel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MatrixChannel")
            .field("homeserver", &self.homeserver)
            .field("room_id", &self.room_id)
            .field("allowed_users", &self.allowed_users)
            .finish_non_exhaustive()
    }
}

#[cfg(test)]
#[path = "mod_tests.rs"]
mod mod_tests;
#[cfg(test)]
#[path = "tests.rs"]
mod tests;
