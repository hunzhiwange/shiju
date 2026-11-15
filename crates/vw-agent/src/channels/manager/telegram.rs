//! Telegram 身份规范化、白名单绑定及托管服务重载提示。
//!
//! 绑定只更新 Telegram 准入列表；服务重启能力尚未实现，因此调用方会收到明确提示。

use super::*;
pub(crate) use channel_runtime::manager::telegram::*;
pub(crate) async fn bind_telegram_identity(config: &Config, identity: &str) -> Result<()> { channel_runtime::manager::telegram::bind_telegram_identity(&super::runtime_host::HOST, config, identity).await }
#[cfg(test)]
#[path = "../../../../libs/channel-runtime/src/manager/telegram_tests.rs"]
mod telegram_tests;
