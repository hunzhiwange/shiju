//! 代码格式化领域服务。

pub mod host;
pub mod format;
pub use format::*;

#[cfg(test)]
mod test_support;
