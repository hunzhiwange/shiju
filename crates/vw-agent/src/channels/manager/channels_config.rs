//! 通道运行实现与宿主能力注入。
use super::*;
pub(crate) use channel_runtime::manager::channels_config::*;
pub(crate) fn collect_configured_channels(config: &Config, context: &str) -> Vec<ConfiguredChannel> { channel_runtime::manager::channels_config::collect_configured_channels(config, crate::session::ui_store::current_tenant_scope(), context) }
