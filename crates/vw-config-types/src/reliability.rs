use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// 可靠性配置：重试与退避策略（`[reliability]`）。
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ReliabilityConfig {
    /// provider 调用失败后的最大重试次数。默认值为 `3`。
    #[serde(default = "default_provider_retries")]
    pub provider_retries: u32,

    /// provider 调用的初始退避时间，单位为毫秒。默认值为 `500`。
    #[serde(default = "default_provider_backoff_ms")]
    pub provider_backoff_ms: u64,

    /// channel 操作的初始退避时间，单位为秒。
    #[serde(default = "default_channel_initial_backoff_secs")]
    pub channel_initial_backoff_secs: u64,

    /// channel 操作允许的最大退避时间，单位为秒。
    #[serde(default = "default_channel_max_backoff_secs")]
    pub channel_max_backoff_secs: u64,

    /// 调度器轮询间隔，单位为秒。
    #[serde(default = "default_scheduler_poll_secs")]
    pub scheduler_poll_secs: u64,

    /// 调度器重试次数。
    #[serde(default = "default_scheduler_retries")]
    pub scheduler_retries: u32,
}

fn default_provider_retries() -> u32 {
    3
}
fn default_provider_backoff_ms() -> u64 {
    500
}
fn default_channel_initial_backoff_secs() -> u64 {
    1
}
fn default_channel_max_backoff_secs() -> u64 {
    60
}
fn default_scheduler_poll_secs() -> u64 {
    60
}
fn default_scheduler_retries() -> u32 {
    3
}

impl Default for ReliabilityConfig {
    fn default() -> Self {
        Self {
            provider_retries: default_provider_retries(),
            provider_backoff_ms: default_provider_backoff_ms(),
            channel_initial_backoff_secs: default_channel_initial_backoff_secs(),
            channel_max_backoff_secs: default_channel_max_backoff_secs(),
            scheduler_poll_secs: default_scheduler_poll_secs(),
            scheduler_retries: default_scheduler_retries(),
        }
    }
}
#[cfg(test)]
#[path = "reliability_tests.rs"]
mod reliability_tests;
