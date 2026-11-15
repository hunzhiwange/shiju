//! 配置类型与持久化领域的兼容导出。
pub use agent_config::runtime_schema::*;
#[cfg(not(target_arch = "wasm32"))]
mod channel_reload;
#[cfg(not(target_arch = "wasm32"))]
pub(crate) use channel_reload::ChannelConfigWatcher;
#[cfg(test)]
mod config_env_tests;
