//! 聚合调度执行引擎：正常派发由 `dispatch` 处理，异常收敛由 `recovery` 处理。

pub mod dispatch;
pub mod recovery;

#[cfg(test)]
mod dispatch_tests;

#[cfg(test)]
mod recovery_tests;
