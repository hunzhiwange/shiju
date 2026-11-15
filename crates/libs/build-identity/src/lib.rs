//! 可执行文件 SHA-256 构建身份。

mod build_identity;

pub use build_identity::{current_executable_build_id, executable_build_id};

#[cfg(test)]
mod build_identity_tests;
