//! 终端显示宽度、文本截断、换行与格式化。

#[allow(dead_code)]
pub mod width;
pub mod line_truncation;
pub mod wrapping;
pub mod live_wrap;
#[allow(dead_code)]
pub mod text_formatting;

mod line_utils;
pub use line_utils::{line_to_static, push_owned_lines};
