//! 任务面板模型及配置契约。

mod executor;
mod subtask;
mod task;
mod board;

pub use executor::*;
pub use subtask::*;
pub use task::*;
pub use board::*;

fn now_ms() -> u64 {
    runtime_time::now_ms()
}

#[cfg(test)]
#[path = "models_tests.rs"]
mod models_tests;
