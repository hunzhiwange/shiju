//! 队列所有者子进程的启动参数、载荷与启动逻辑。

pub mod queue_owner_process;

pub use queue_owner_process::*;

#[cfg(test)]
mod queue_owner_process_tests;
