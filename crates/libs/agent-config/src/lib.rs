//! 配置迁移与运行时配置路径。

pub mod runtime_home;

#[cfg(all(test, not(target_arch = "wasm32")))]
mod runtime_home_tests;

pub mod schema;
pub mod traits;

pub mod runtime;
pub mod runtime_schema;
pub mod catalog;
#[cfg(not(target_arch = "wasm32"))]
pub mod store;

#[cfg(test)]
mod host_migration_29_tests;

#[cfg(test)]
mod host_migration_58_tests;
