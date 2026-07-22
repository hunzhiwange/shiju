//! 文件与目录 API。
//!
//! 该模块同时兼容两类网关文件接口：
//! - 新版 `/v1/files/*` 结构化文件操作接口
//! - 旧版 `/v1/file/*` 目录上下文接口
//!
//! 其中 `file_list` 还会把旧版目录列表结果转换为新版 `FileNodeDto` 树结构，供上层统一消费。

use std::future::Future;
use std::path::Path;
use std::pin::Pin;

use serde::{Deserialize, Serialize};
use vw_api_types::common::OperationAck;
use vw_api_types::file::{
    CopyFileRequest, DeleteFileRequest, FileNodeDto, FileNodeKind, ListFilesRequest,
    ListFilesResponse, MoveFileRequest, ReadFileRequest, ReadFileResponse, SearchFilesRequest,
    SearchFilesResponse, StatFileRequest, StatFileResponse, WriteFileRequest, WriteFileResponse,
};

use super::GatewayClient;

#[cfg(target_arch = "wasm32")]
/// 文件树递归构建过程使用的异步返回类型（WASM 版本）。
type FileNodeFuture<'a> = Pin<Box<dyn Future<Output = Result<FileNodeDto, String>> + 'a>>;

#[cfg(not(target_arch = "wasm32"))]
/// 文件树递归构建过程使用的异步返回类型（原生版本，要求 `Send`）。
type FileNodeFuture<'a> = Pin<Box<dyn Future<Output = Result<FileNodeDto, String>> + Send + 'a>>;

#[derive(Debug, Clone, Serialize)]
struct DirectoryFileReadRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    directory: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    agent_key: Option<String>,
    path: String,
}

/// 目录内文件读取接口的响应体。
///
/// 对应旧版 `/v1/file/read` 接口，适用于需要目录上下文与可选 agent 标识的调用场景。
#[derive(Debug, Clone, Deserialize)]
pub struct DirectoryFileReadResponse {
    /// 实际解析后的根目录。
    pub root_directory: String,
    /// 相对根目录的文件路径。
    pub path: String,
    /// 文件文本内容。
    pub content: String,
}

#[derive(Debug, Clone, Serialize)]
struct DirectoryFileWriteRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    directory: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    agent_key: Option<String>,
    path: String,
    content: String,
    create_if_missing: bool,
}

/// 目录内文件写入接口的响应体。
///
/// 对应旧版 `/v1/file/write` 接口。
#[derive(Debug, Clone, Deserialize)]
pub struct DirectoryFileWriteResponse {
    /// 是否写入成功。
    pub ok: bool,
    /// 实际解析后的根目录。
    pub root_directory: String,
    /// 相对根目录的文件路径。
    pub path: String,
    /// 本次写入的字节数。
    pub bytes_written: u64,
}

impl GatewayClient {
    /// 列出项目或 worktree 下的文件树。
    ///
    /// 该方法会先解析出真实目录，再递归拼装文件树。返回结果统一为 `ListFilesResponse`，便于前端直接消费。
    pub async fn file_list(&self, request: &ListFilesRequest) -> Result<ListFilesResponse, String> {
        let directory =
            self.resolve_file_directory(&request.project_id, request.worktree_id.as_ref()).await?;
        let path = normalize_requested_path(request.path.as_deref());
        let root = self.file_list_directory(&directory, path, request.depth).await?;
        Ok(ListFilesResponse { root })
    }

    /// 读取文件内容。
    pub async fn file_read(&self, request: &ReadFileRequest) -> Result<ReadFileResponse, String> {
        self.post_json("/v1/files/read", &[], request).await
    }

    /// 写入文件内容。
    pub async fn file_write(
        &self,
        request: &WriteFileRequest,
    ) -> Result<WriteFileResponse, String> {
        self.post_json("/v1/files/write", &[], request).await
    }

    /// 移动文件或目录。
    pub async fn file_move(&self, request: &MoveFileRequest) -> Result<OperationAck, String> {
        self.post_json("/v1/files/move", &[], request).await
    }

    /// 复制文件或目录。
    pub async fn file_copy(&self, request: &CopyFileRequest) -> Result<OperationAck, String> {
        self.post_json("/v1/files/copy", &[], request).await
    }

    /// 删除文件或目录。
    pub async fn file_delete(&self, request: &DeleteFileRequest) -> Result<OperationAck, String> {
        self.post_json("/v1/files/delete", &[], request).await
    }

    /// 按条件搜索文件。
    pub async fn file_search(
        &self,
        request: &SearchFilesRequest,
    ) -> Result<SearchFilesResponse, String> {
        self.post_json("/v1/files/search", &[], request).await
    }

    /// 查询文件或目录状态。
    pub async fn file_stat(&self, request: &StatFileRequest) -> Result<StatFileResponse, String> {
        self.post_json("/v1/files/stat", &[], request).await
    }

    /// 在指定目录上下文中读取相对路径文件。
    ///
    /// `agent_key` 允许调用方把请求绑定到特定 agent 上下文；不需要时可以传 `None`。
    pub async fn file_read_in_directory(
        &self,
        directory: Option<&str>,
        agent_key: Option<&str>,
        path: &str,
    ) -> Result<DirectoryFileReadResponse, String> {
        self.post_json(
            "/v1/file/read",
            &[],
            &DirectoryFileReadRequest {
                directory: directory.map(str::to_string),
                agent_key: agent_key.map(str::to_string),
                path: path.to_string(),
            },
        )
        .await
    }

    /// 在指定目录上下文中写入相对路径文件。
    ///
    /// `create_if_missing` 为 `true` 时，后端可以在目标不存在时创建新文件。
    pub async fn file_write_in_directory(
        &self,
        directory: Option<&str>,
        agent_key: Option<&str>,
        path: &str,
        content: &str,
        create_if_missing: bool,
    ) -> Result<DirectoryFileWriteResponse, String> {
        self.post_json(
            "/v1/file/write",
            &[],
            &DirectoryFileWriteRequest {
                directory: directory.map(str::to_string),
                agent_key: agent_key.map(str::to_string),
                path: path.to_string(),
                content: content.to_string(),
                create_if_missing,
            },
        )
        .await
    }
}

impl GatewayClient {
    /// 根据项目 ID 或 worktree ID 解析实际文件根目录。
    async fn resolve_file_directory<T: AsRef<str>, U: AsRef<str>>(
        &self,
        project_id: &T,
        worktree_id: Option<&U>,
    ) -> Result<String, String> {
        let scope = vw_api_types::id::ProjectScope::for_default_tenant(
            vw_api_types::id::ProjectId::from(project_id.as_ref()),
        )
        .map_err(|error| error.to_string())?;
        if let Some(worktree_id) = worktree_id {
            return self
                .worktree_get(&scope, worktree_id.as_ref())
                .await
                .map(|response| response.worktree.directory);
        }

        self.project_get(&scope).await.map(|response| response.project.directory)
    }

    /// 递归列出目录，并构造统一的 `FileNodeDto` 树节点。
    fn file_list_directory<'a>(
        &'a self,
        directory: &'a str,
        path: String,
        depth: Option<u32>,
    ) -> FileNodeFuture<'a> {
        Box::pin(async move {
            let entries: Vec<LegacyFileNode> =
                self.get_json("/v1/file", &legacy_file_list_query(directory, &path)).await?;

            let children = match depth {
                Some(0) => None,
                _ => {
                    let child_depth = depth.and_then(|value| value.checked_sub(1));
                    let mut children = Vec::with_capacity(entries.len());
                    for entry in entries {
                        children
                            .push(self.map_legacy_file_node(directory, entry, child_depth).await?);
                    }
                    Some(children)
                }
            };

            Ok(FileNodeDto {
                path: path.clone(),
                name: directory_node_name(directory, &path),
                kind: FileNodeKind::Directory,
                size_bytes: None,
                children,
            })
        })
    }

    /// 将旧版文件节点映射为统一文件树节点；目录会继续递归展开。
    fn map_legacy_file_node<'a>(
        &'a self,
        directory: &'a str,
        entry: LegacyFileNode,
        depth: Option<u32>,
    ) -> FileNodeFuture<'a> {
        Box::pin(async move {
            let kind = entry.kind.into();
            if matches!(kind, FileNodeKind::Directory) {
                return self
                    .file_list_directory(directory, normalize_entry_path(&entry.path), depth)
                    .await;
            }

            Ok(FileNodeDto {
                path: normalize_entry_path(&entry.path),
                name: entry.name,
                kind,
                size_bytes: None,
                children: None,
            })
        })
    }
}

/// 生成旧版文件列表接口所需的查询参数。
fn legacy_file_list_query(directory: &str, path: &str) -> Vec<(String, String)> {
    vec![
        ("directory".to_string(), directory.to_string()),
        ("path".to_string(), if path == "." { String::new() } else { path.to_string() }),
    ]
}

/// 规范化调用方传入的路径，空值统一回退到当前目录 `.`。
fn normalize_requested_path(path: Option<&str>) -> String {
    let value = path.unwrap_or(".").trim();
    if value.is_empty() { ".".to_string() } else { normalize_entry_path(value) }
}

/// 规范化网关返回的条目路径，统一分隔符并去掉外围斜杠。
fn normalize_entry_path(path: &str) -> String {
    let value = path.trim().replace('\\', "/").trim_matches('/').to_string();
    if value.is_empty() { ".".to_string() } else { value }
}

/// 根据目录与相对路径推导目录节点名称。
fn directory_node_name(directory: &str, path: &str) -> String {
    if path == "." {
        return Path::new(directory)
            .file_name()
            .and_then(|value| value.to_str())
            .filter(|value| !value.is_empty())
            .unwrap_or(directory)
            .to_string();
    }

    Path::new(path)
        .file_name()
        .and_then(|value| value.to_str())
        .filter(|value| !value.is_empty())
        .unwrap_or(path)
        .to_string()
}

#[derive(Debug, Deserialize)]
struct LegacyFileNode {
    path: String,
    name: String,
    #[serde(rename = "type")]
    kind: LegacyFileNodeKind,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
enum LegacyFileNodeKind {
    File,
    Directory,
}

impl From<LegacyFileNodeKind> for FileNodeKind {
    fn from(value: LegacyFileNodeKind) -> Self {
        match value {
            LegacyFileNodeKind::File => Self::File,
            LegacyFileNodeKind::Directory => Self::Directory,
        }
    }
}

#[cfg(test)]
#[path = "file_api_tests.rs"]
mod file_api_tests;
