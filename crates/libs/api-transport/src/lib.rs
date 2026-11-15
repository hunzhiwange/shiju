//! 共享传输协议：基础响应、错误、标识符与作用域。

pub mod common;
pub mod error;
pub mod id;

pub use common::*;
pub use error::*;
pub use id::*;

#[cfg(test)]
mod common_tests;
#[cfg(test)]
mod error_tests;
#[cfg(test)]
mod id_tests;
