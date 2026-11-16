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
