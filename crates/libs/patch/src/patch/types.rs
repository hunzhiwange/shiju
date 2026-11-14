//! 补丁类型模块，定义解析、预览和文件应用阶段共享的数据结构与错误类型。

use serde::{Deserialize, Serialize};

/// Hunk 枚举描述该模块支持的 Hunk 取值集合。
///
/// 每个变体代表一个明确分支，调用方应通过显式匹配处理新增状态。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum Hunk {
    Add { path: String, contents: String },
    Delete { path: String },
    Update { path: String, move_path: Option<String>, chunks: Vec<UpdateFileChunk> },
}

/// UpdateFileChunk 数据结构承载该模块对外传递的 UpdateFileChunk 状态。
///
/// 字段保持可序列化或可渲染形态，便于调用方直接组合 UI 或持久化数据。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateFileChunk {
    pub old_lines: Vec<String>,
    pub new_lines: Vec<String>,
    pub change_context: Option<String>,
    pub is_end_of_file: Option<bool>,
}

/// ParseResult 数据结构承载该模块对外传递的 ParseResult 状态。
///
/// 字段保持可序列化或可渲染形态，便于调用方直接组合 UI 或持久化数据。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParseResult {
    pub hunks: Vec<Hunk>,
}

/// AffectedPaths 数据结构承载该模块对外传递的 AffectedPaths 状态。
///
/// 字段保持可序列化或可渲染形态，便于调用方直接组合 UI 或持久化数据。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AffectedPaths {
    pub added: Vec<String>,
    pub modified: Vec<String>,
    pub deleted: Vec<String>,
}

/// ApplyPatchFileUpdate 数据结构承载该模块对外传递的 ApplyPatchFileUpdate 状态。
///
/// 字段保持可序列化或可渲染形态，便于调用方直接组合 UI 或持久化数据。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApplyPatchFileUpdate {
    pub unified_diff: String,
    pub content: String,
}

/// Error 枚举描述该模块支持的 Error 取值集合。
///
/// 每个变体代表一个明确分支，调用方应通过显式匹配处理新增状态。
#[derive(Debug)]
pub enum Error {
    Parse(String),
    Io(std::io::Error),
    ComputeReplacements(String),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Parse(msg) => write!(f, "{}", msg),
            Error::Io(err) => write!(f, "{}", err),
            Error::ComputeReplacements(msg) => write!(f, "{}", msg),
        }
    }
}

impl std::error::Error for Error {}

impl From<std::io::Error> for Error {
    fn from(value: std::io::Error) -> Self {
        Error::Io(value)
    }
}

#[cfg(test)]
#[path = "types_tests.rs"]
mod types_tests;
