//! ACP 文本、JSON 与静默输出格式化及读类工具输出抑制。

pub mod output;
pub mod output_json_formatter;
pub mod read_output_suppression;

pub use output::{
    AnyOutputFormatter, OutputFormatterOptions, QuietOutputFormatter, TextOutputFormatter,
    create_output_formatter,
};
pub use output_json_formatter::{JsonOutputFormatter, create_json_output_formatter};
pub use read_output_suppression::{
    ReadLikeToolDescriptor, SUPPRESSED_READ_OUTPUT, is_read_like_tool,
};
