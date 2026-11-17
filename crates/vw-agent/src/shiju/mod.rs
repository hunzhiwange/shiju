//! SHIJU 上游客户端的兼容导出。

#[cfg(any(test, target_arch = "wasm32"))]
pub(crate) use shiju_client::{account, apps};
