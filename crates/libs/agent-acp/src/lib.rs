//! 原生运行时的 ACP 协议适配。

pub mod approval;
pub mod backend;
pub mod models;
pub mod mount_store;
pub mod protocol;
pub mod prompt;

#[cfg(test)]
mod approval_tests;
