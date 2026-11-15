//! 代码格式化模块。
//!
//! 保持对外接口不变，并将实现按职责拆分到同名子目录中，
//! 分别承载状态管理、启用检测、执行流程和内置格式化器清单。

mod builtins;
mod detect;
mod runtime;
mod state;

#[cfg(test)]
mod builtins_tests;
#[cfg(test)]
mod detect_tests;
#[cfg(test)]
mod runtime_tests;
#[cfg(test)]
mod state_tests;

pub use runtime::{init, status};
pub use state::{FormatterStatus, State};
