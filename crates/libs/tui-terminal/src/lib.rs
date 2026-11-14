//! 终端双缓冲绘制后端、底层探测与输入回放。

pub mod custom_terminal;
pub use custom_terminal::{Frame, Terminal};
pub mod terminal_probe;

#[cfg(test)]
mod test_backend;
