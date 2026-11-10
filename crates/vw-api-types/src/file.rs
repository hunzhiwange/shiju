//! 文件树、读写与搜索相关 API 类型。
//!
//! 本模块描述项目工作区中的文件系统操作协议，主要面向：
//! - 文件树面板
//! - 编辑器读取与保存
//! - 工作区全文搜索
//! - 文件元信息查询
//!
//! 这些类型默认以项目和可选工作树为作用域，避免直接暴露宿主机全局文件系统。

use crate::id::{ProjectId, WorktreeId};
use serde::{Deserialize, Serialize};

/// 文件树节点类型。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FileNodeKind {
    /// 普通文件。
    File,
    /// 目录。
    Directory,
}

/// 文件树节点。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FileNodeDto {
    /// 节点相对路径或逻辑路径。
    pub path: String,
    /// 节点名称。
    pub name: String,
    /// 节点类型。
    pub kind: FileNodeKind,
    /// 文件大小；目录通常为空。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub size_bytes: Option<u64>,
    /// 子节点；仅目录可能存在。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub children: Option<Vec<FileNodeDto>>,
}

/// 列出文件树请求。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ListFilesRequest {
    /// 目标项目 ID。
    pub project_id: ProjectId,
    /// 可选工作树 ID。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub worktree_id: Option<WorktreeId>,
    /// 起始路径。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    /// 遍历深度上限。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub depth: Option<u32>,
}

/// 列出文件树响应。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ListFilesResponse {
    /// 根节点。
    pub root: FileNodeDto,
}

/// 读取文件请求。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReadFileRequest {
    /// 目标项目 ID。
    pub project_id: ProjectId,
    /// 可选工作树 ID。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub worktree_id: Option<WorktreeId>,
    /// 文件路径。
    pub path: String,
    /// 起始行号偏移。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub offset_line: Option<u32>,
    /// 读取行数上限。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit_lines: Option<u32>,
}

/// 读取文件响应。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReadFileResponse {
    /// 文件路径。
    pub path: String,
    /// 文件内容片段。
    pub content: String,
    /// 内容编码。
    pub encoding: String,
    /// 起始行号偏移。
    pub offset_line: u32,
    /// 本次返回的行数。
    pub line_count: u32,
    /// 是否因限制而截断。
    pub truncated: bool,
}

/// 写入文件请求。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WriteFileRequest {
    /// 目标项目 ID。
    pub project_id: ProjectId,
    /// 可选工作树 ID。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub worktree_id: Option<WorktreeId>,
    /// 目标路径。
    pub path: String,
    /// 写入内容。
    pub content: String,
    /// 文件不存在时是否自动创建。
    #[serde(default)]
    pub create_if_missing: bool,
}

/// 写入文件响应。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WriteFileResponse {
    /// 是否成功。
    pub ok: bool,
    /// 实际写入路径。
    pub path: String,
    /// 写入字节数。
    pub bytes_written: u64,
}

/// 移动文件请求。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MoveFileRequest {
    /// 目标项目 ID。
    pub project_id: ProjectId,
    /// 可选工作树 ID。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub worktree_id: Option<WorktreeId>,
    /// 源路径。
    pub from_path: String,
    /// 包含文件或目录名称的完整项目内目标路径；缺少的父目录会自动创建。
    pub to_path: String,
    /// 是否覆盖已存在的普通文件；目录不支持覆盖。树内拖放必须为 false。
    #[serde(default)]
    pub overwrite: bool,
}

/// 复制文件请求。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CopyFileRequest {
    /// 目标项目 ID。
    pub project_id: ProjectId,
    /// 可选工作树 ID。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub worktree_id: Option<WorktreeId>,
    /// 源路径。
    pub from_path: String,
    /// 目标路径。
    pub to_path: String,
    /// 是否覆盖已存在目标。
    #[serde(default)]
    pub overwrite: bool,
}

/// 在项目或工作树中的源路径旁创建副本。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DuplicateFileRequest {
    pub project_id: ProjectId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub worktree_id: Option<WorktreeId>,
    pub path: String,
}

/// 从网关所在本机复制外部路径到项目；仅支持真实回环连接。
///
/// 网关先验证项目、工作树与连接授权，再读取源文件；授权失败整体拒绝。
/// 空列表或超过 128 项返回 HTTP 400，合法请求返回按输入顺序排列的逐项结果。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LocalImportRequest {
    /// 目标项目 ID。
    pub project_id: ProjectId,
    /// 可选工作树 ID。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub worktree_id: Option<WorktreeId>,
    /// 项目内相对目标目录；空字符串表示根目录。
    pub target_directory: String,
    /// 1–128 个本机绝对源路径；复制保留源，同名拒绝覆盖。
    pub source_paths: Vec<String>,
}

/// 单个本机路径的导入结果。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LocalImportItemResult {
    /// 对应输入的本机源路径。
    pub source_path: String,
    /// 成功导入后的项目内相对目标路径。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub destination_path: Option<String>,
    /// 该项是否导入成功。
    pub ok: bool,
    /// 该项失败原因。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// 本机导入响应；逐项结果与输入顺序一致，允许部分失败。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LocalImportResponse {
    /// 每个源路径的处理结果。
    pub items: Vec<LocalImportItemResult>,
}

/// 删除文件或目录请求。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DeleteFileRequest {
    /// 目标项目 ID。
    pub project_id: ProjectId,
    /// 可选工作树 ID。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub worktree_id: Option<WorktreeId>,
    /// 待删除路径。
    pub path: String,
    /// 删除目录时是否递归。
    #[serde(default)]
    pub recursive: bool,
}

/// 搜索文件内容请求。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SearchFilesRequest {
    /// 目标项目 ID。
    pub project_id: ProjectId,
    /// 可选工作树 ID。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub worktree_id: Option<WorktreeId>,
    /// 搜索模式。
    pub pattern: String,
    /// 可选包含路径过滤规则。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub include: Option<String>,
    /// 是否按正则解释 pattern。
    #[serde(default)]
    pub regex: bool,
    /// 是否区分大小写。
    #[serde(default)]
    pub case_sensitive: bool,
}

/// 单个搜索命中项。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SearchMatchDto {
    /// 命中文件路径。
    pub path: String,
    /// 命中行号。
    pub line: u32,
    /// 命中列号。
    pub column: u32,
    /// 命中行文本。
    pub text: String,
}

/// 搜索文件响应。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SearchFilesResponse {
    /// 所有命中项。
    pub matches: Vec<SearchMatchDto>,
}

/// 查询文件元信息请求。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StatFileRequest {
    /// 目标项目 ID。
    pub project_id: ProjectId,
    /// 可选工作树 ID。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub worktree_id: Option<WorktreeId>,
    /// 目标路径。
    pub path: String,
}

/// 单个文件系统条目元信息。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FileEntryDto {
    /// 路径。
    pub path: String,
    /// 名称。
    pub name: String,
    /// 类型。
    pub kind: FileNodeKind,
    /// 文件大小；目录通常为空。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub size_bytes: Option<u64>,
    /// 最后修改时间。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub modified_at_ms: Option<i64>,
}

/// 查询文件元信息响应。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StatFileResponse {
    /// 文件或目录条目。
    pub entry: FileEntryDto,
}
