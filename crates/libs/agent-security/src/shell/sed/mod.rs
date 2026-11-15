//! sed 命令解析与安全校验入口。
//!
//! 该模块只暴露受限 sed 能力：解析安全的原地替换编辑，以及校验只读 print/replace
//! 命令。危险的写文件、执行命令和复杂脚本语法由 validation 子模块阻断。

mod parser;
mod validation;

pub use parser::{SedEdit, SedParseError};
pub use validation::{SedCommandKind, SedValidationResult, validate_sed_command};

#[cfg(test)]
#[path = "parser_tests.rs"]
mod parser_tests;

#[cfg(test)]
#[path = "validation_tests.rs"]
mod validation_tests;
