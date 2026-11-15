//! 配置迁移与运行时配置路径。

#[cfg(not(target_arch = "wasm32"))]
pub mod migration;
pub mod runtime_home;

#[cfg(all(test, not(target_arch = "wasm32")))]
mod runtime_home_tests;

pub mod schema;
pub mod traits;
