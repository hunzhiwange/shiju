//! 终端双缓冲绘制后端、底层探测与输入回放。

pub mod custom_terminal;
pub use custom_terminal::{Frame, Terminal};
pub mod terminal_probe;

#[cfg(test)]
mod test_backend;

pub mod insert_history;
#[allow(dead_code)]
pub mod transcript_reflow;
pub mod resize_reflow_cap;
