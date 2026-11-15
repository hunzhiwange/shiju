//! # 通道配置收集模块
//!
//! 本模块负责从主配置对象中收集并实例化所有已配置的通信通道。
//!
//! ## 主要功能
//!
//! - **通道收集**：遍历配置中所有启用的通道类型，创建对应的通道实例
//! - **特性门控**：根据编译时特性标志有条件地启用或禁用特定通道
//! - **平台适配**：针对不同目标平台（如 wasm32）调整可用通道集合
//! - **异步初始化**：支持需要异步初始化的通道（如 Nostr）
//!
//! ## 支持的通道类型
//!
//! 本模块支持以下通信通道（具体可用性取决于编译特性）：
//!
//! - **即时通讯**：Telegram、Discord、Slack、Mattermost、iMessage、Signal、WhatsApp
//! - **企业协作**：Lark/Feishu、DingTalk、QQ、Nextcloud Talk
//! - **其他协议**：IRC、Email、Nostr、Linq、WATI、ClawdTalk
//!
//! ## 架构说明
//!
//! 模块采用工厂模式，根据配置动态创建通道实例。每个通道都实现了 `Channel` trait，
//! 并被包装在 `ConfiguredChannel` 结构中以提供显示名称和统一的接口。

use super::*;
pub(crate) use channel_runtime::manager::channels_config::*;
pub(crate) fn collect_configured_channels(config: &Config, context: &str) -> Vec<ConfiguredChannel> { channel_runtime::manager::channels_config::collect_configured_channels(config, crate::session::ui_store::current_tenant_scope(), context) }
