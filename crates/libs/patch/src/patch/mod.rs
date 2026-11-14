//! 补丁解析与应用模块。
//!
//! 本模块提供类 Aider 风格补丁格式的解析、内容推导与文件系统应用能力。
//! 外部接口保持集中导出，内部实现按职责拆分到独立文件。

mod filesystem;
mod parse;
mod replace;
mod types;

pub use filesystem::{apply_hunks_to_files, apply_patch, preview_changes};
pub use parse::parse_patch;
pub use replace::derive_new_contents_from_chunks;
pub use types::{AffectedPaths, ApplyPatchFileUpdate, Error, Hunk, ParseResult, UpdateFileChunk};
