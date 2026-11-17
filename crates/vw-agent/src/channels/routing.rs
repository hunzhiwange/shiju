//! 通道路由上下文入口归支撑库。
#[cfg(any(test, target_arch = "wasm32"))]
pub(crate) use channel_support::routing_context::*;
