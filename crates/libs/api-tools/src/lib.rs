//! 工具规格、调用、动态注册与结果契约。

pub mod tools;
pub use tools::*;

#[cfg(test)]
mod tools_tests;
#[cfg(test)]
mod dynamic_tools_tests;
