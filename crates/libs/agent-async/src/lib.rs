//! 异步取消、同步、队列与任务协调工具。

pub mod abort;
pub mod eventloop;
pub mod lock;
pub mod queue;
pub mod signal;
pub mod task_set;
pub mod timeout;

#[cfg(test)]
mod abort_tests;
#[cfg(test)]
mod eventloop_tests;
#[cfg(test)]
mod lock_tests;
#[cfg(test)]
mod queue_tests;
#[cfg(test)]
mod task_set_tests;
#[cfg(test)]
mod timeout_tests;

#[cfg(test)]
mod signal_tests;
