//! 复用终端底层测试后端，保持上层测试入口。

#[path = "../../libs/tui-terminal/src/test_backend.rs"]
mod implementation;
pub use implementation::*;
