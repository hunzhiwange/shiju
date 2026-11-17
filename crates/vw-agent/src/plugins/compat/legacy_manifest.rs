//! 插件清单兼容解析的唯一实现位于 agent-plugins。
#[cfg(any(test, target_arch = "wasm32"))]
pub use agent_plugins::legacy_manifest::*;
