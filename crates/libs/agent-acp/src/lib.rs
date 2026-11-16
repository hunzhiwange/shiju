//! 原生运行时的 ACP 协议适配。

pub mod approval;
pub mod backend;
pub mod models;
pub mod mount_store;
#[cfg(not(target_arch = "wasm32"))]
pub mod catalog_mounts;
pub mod protocol;
pub mod prompt;

#[cfg(test)]
mod approval_tests;

pub mod persistence;

#[cfg(test)]
mod host_migration_53_tests;

pub mod mcp;

pub mod mounts;

#[cfg(feature = "test-support")]
pub mod host_migration_53_mcp_contracts;

#[cfg(all(feature = "test-support", not(target_arch = "wasm32")))]
pub mod host_migration_53_recovery_contracts;

#[cfg(all(feature = "test-support", not(target_arch = "wasm32")))]
pub mod host_migration_53_update_contracts;

#[cfg(all(feature = "test-support", not(target_arch = "wasm32")))]
pub mod host_migration_53_acceptance_contracts;

#[cfg(all(feature = "test-support", target_os = "macos"))]
pub mod host_migration_53_catalog_contracts;
