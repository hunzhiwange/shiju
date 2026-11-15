//! 通道运行实现与宿主能力注入。
use super::*;
pub(crate) use channel_runtime::manager::telegram::*;
pub(crate) async fn bind_telegram_identity(config: &Config, identity: &str) -> Result<()> { channel_runtime::manager::telegram::bind_telegram_identity(&super::runtime_host::HOST, config, identity).await }
#[cfg(test)]
#[path = "../../../../libs/channel-runtime/src/manager/telegram_tests.rs"]
mod telegram_tests;
