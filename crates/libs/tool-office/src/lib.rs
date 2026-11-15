#![cfg(unix)]
//! Structured Office tools sharing the native control authority.

pub mod excel_control;
pub mod word_control;

pub use excel_control::ExcelControlTool;
pub use word_control::WordControlTool;
