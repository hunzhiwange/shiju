//! Email 通道配置适配。
//!
//! 本模块重新导出共享配置 crate 中的 Email schema，并在代理侧实现
//! `ChannelConfig`，让 Email 通道能被统一的配置展示和启用状态扫描逻辑识别。

pub use config_channel_services::email::*;

use crate::app::agent::config::traits::ChannelConfig;

impl ChannelConfig for EmailConfig {
    fn name() -> &'static str {
        "Email"
    }

    fn desc() -> &'static str {
        "Email over IMAP/SMTP"
    }
}
