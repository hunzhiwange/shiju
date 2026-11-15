use serde::{Deserialize, Serialize};
use serde_json::Value;
use vw_api_tools::StructuredPatchHunkDto;

/// `edit` 工具输入参数。
#[derive(Debug, Clone, Deserialize)]
pub(crate) struct Args {
    /// 目标文件路径。
    #[serde(alias = "filePath", alias = "path")]
    pub file_path: String,
    /// 需要匹配的旧字符串。
    #[serde(alias = "oldString")]
    pub old_string: String,
    /// 替换后的新字符串。
    #[serde(alias = "newString")]
    pub new_string: String,
    /// 是否替换所有匹配项。
    #[serde(default, alias = "replaceAll")]
    pub replace_all: bool,
}

/// 文件描述信息。
#[derive(Debug, Clone, Serialize)]
pub struct FileDescriptor {
    pub path: String,
    pub absolute_path: String,
    pub open: String,
    pub size_bytes: u64,
}

/// 结构化 patch 载荷。
#[derive(Debug, Clone, Serialize)]
pub struct StructuredPatch {
    pub hunks: Vec<StructuredPatchHunkDto>,
}

/// `edit` 工具成功结果。
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub(crate) enum EditPayload {
    Update {
        file: FileDescriptor,
        replacements: usize,
        replace_all: bool,
        quote_normalized_match: bool,
        structured_patch: StructuredPatch,
        #[serde(skip_serializing_if = "Option::is_none")]
        read_state: Option<Value>,
    },
}
#[cfg(test)]
#[path = "types_tests.rs"]
mod types_tests;
