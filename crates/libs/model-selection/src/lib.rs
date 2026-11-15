//! 不访问网络和供应商目录的确定性模型选择。
pub mod classifier;
mod selector;
pub use selector::*;

#[cfg(test)]
mod selector_tests;
