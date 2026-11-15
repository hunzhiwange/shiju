//! 通道运行实现与宿主能力注入。
use super::*;
pub(crate) use channel_runtime::manager::health::*;
pub async fn doctor_channels(config: Config) -> Result<()> { channel_runtime::manager::health::doctor_channels(config, crate::session::ui_store::current_tenant_scope()).await }
#[cfg(test)]
#[path = "../../../../libs/channel-runtime/src/manager/health_tests.rs"]
mod health_tests;
