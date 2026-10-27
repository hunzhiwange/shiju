//! 通道配置模块
//!
//! 本模块定义了 VibeWindow 代理系统中所有通道类型的配置结构。
//! 通道是代理与外部系统进行通信的接口，支持多种消息平台和协议。
//!
//! # 模块结构
//!
//! - [`chat`] - 聊天平台配置（如通用聊天接口）
//! - [`email`] - 电子邮件通道配置
//! - [`helpers`] - 通道配置的辅助工具函数（仅内部使用）
//! - [`lark`] - 飞书（Lark）平台配置
//! - [`other`] - 其他/自定义通道配置
//! - [`secrets`] - 通道密钥和敏感信息管理
//! - [`telegram_env`] - Telegram 环境配置
//! - [`types`] - 通道类型定义和通用类型结构
//!
//! # 使用示例
//!
//! ```ignore
//! use vibe_window::app::agent::config::schema::channels::{TelegramConfig, EmailConfig};
//!
//! // 加载 Telegram 配置
//! let telegram = TelegramConfig::from_env()?;
//!
//! // 加载邮件配置
//! let email = EmailConfig::from_env()?;
//! ```
//!
//! # 架构说明
//!
//! 通道配置采用分层设计：
//! 1. 每个通道类型有独立的配置结构
//! 2. 所有配置都支持从环境变量加载
//! 3. 敏感信息通过 [`secrets`] 模块统一管理
//! 4. 通用类型和枚举在 [`types`] 模块中定义

pub mod chat;
pub mod email;
pub mod helpers;
pub mod lark;
pub mod other;
pub mod secrets;
mod secrets_isolation;
pub mod telegram_env;
pub mod types;

#[cfg(test)]
#[path = "chat_tests.rs"]
mod chat_tests;
#[cfg(test)]
#[path = "email_tests.rs"]
mod email_tests;
#[cfg(test)]
#[path = "helpers_tests.rs"]
mod helpers_tests;
#[cfg(test)]
#[path = "lark_tests.rs"]
mod lark_tests;
#[cfg(test)]
#[path = "other_tests.rs"]
mod other_tests;
#[cfg(test)]
#[path = "secrets_tests.rs"]
mod secrets_tests;
#[cfg(test)]
#[path = "telegram_env_tests.rs"]
mod telegram_env_tests;
#[cfg(test)]
#[path = "types_tests.rs"]
mod types_tests;

pub use chat::*;
pub use email::*;
pub(crate) use helpers::*;
pub use lark::*;
pub use other::*;
pub use secrets::*;
pub(crate) use secrets_isolation::decrypt_channel_secrets_isolated;
pub use telegram_env::*;
pub use types::*;
