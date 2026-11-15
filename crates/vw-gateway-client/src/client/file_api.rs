//! 文件与目录 API 的兼容转发。

use vw_api_transport::common::OperationAck;
use vw_api_file::file::{
    CopyFileRequest, DeleteFileRequest, DuplicateFileRequest,
    ListFilesRequest, ListFilesResponse, LocalImportRequest, LocalImportResponse, MoveFileRequest,
    ReadFileRequest, ReadFileResponse, SearchFilesRequest, SearchFilesResponse, StatFileRequest,
    StatFileResponse, WriteFileRequest, WriteFileResponse,
};

use super::GatewayClient;
use gateway_files::{DirectoryFileReadResponse, DirectoryFileWriteResponse};

impl GatewayClient {
    /// 在目录上下文中按文本模式搜索文件内容。
    pub async fn file_find_text_in_directory(
        &self,
        directory: Option<&str>,
        pattern: &str,
    ) -> Result<serde_json::Value, String> {
        gateway_files::GatewayFilesClient::new(self.transport.clone())
            .file_find_text_in_directory(directory, pattern).await
    }

    /// 在目录上下文中按名称搜索文件或目录。
    pub async fn file_find_in_directory(
        &self,
        directory: Option<&str>,
        query_text: &str,
        include_directories: Option<bool>,
        entry_type: Option<&str>,
        limit: Option<usize>,
    ) -> Result<serde_json::Value, String> {
        gateway_files::GatewayFilesClient::new(self.transport.clone())
            .file_find_in_directory(directory, query_text, include_directories, entry_type, limit).await
    }

    /// 列出目录上下文中的单层文件节点。
    pub async fn directory_file_list(
        &self,
        directory: Option<&str>,
        path: &str,
    ) -> Result<serde_json::Value, String> {
        gateway_files::GatewayFilesClient::new(self.transport.clone())
            .directory_file_list(directory, path).await
    }

    /// 读取目录上下文中的完整文件内容。
    pub async fn file_content_in_directory(
        &self,
        directory: Option<&str>,
        path: &str,
    ) -> Result<serde_json::Value, String> {
        gateway_files::GatewayFilesClient::new(self.transport.clone())
            .file_content_in_directory(directory, path).await
    }

    /// 查询目录上下文中的 Git 文件状态。
    /// 查询文件或目录状态。
    pub async fn file_status_in_directory(
        &self,
        directory: Option<&str>,
    ) -> Result<serde_json::Value, String> {
        gateway_files::GatewayFilesClient::new(self.transport.clone())
            .file_status_in_directory(directory).await
    }

    /// 列出项目或 worktree 下的文件树。
    ///
    /// 该方法会先解析出真实目录，再递归拼装文件树。返回结果统一为 `ListFilesResponse`，便于前端直接消费。
    pub async fn file_list(&self, request: &ListFilesRequest) -> Result<ListFilesResponse, String> {
        gateway_files::GatewayFilesClient::new(self.transport.clone())
            .file_list(request).await
    }

    /// 读取文件内容。
    pub async fn file_read(&self, request: &ReadFileRequest) -> Result<ReadFileResponse, String> {
        gateway_files::GatewayFilesClient::new(self.transport.clone())
            .file_read(request).await
    }

    /// 写入文件内容。
    pub async fn file_write(
        &self,
        request: &WriteFileRequest,
    ) -> Result<WriteFileResponse, String> {
        gateway_files::GatewayFilesClient::new(self.transport.clone())
            .file_write(request).await
    }

    /// 移动文件或目录。
    pub async fn file_move(&self, request: &MoveFileRequest) -> Result<OperationAck, String> {
        gateway_files::GatewayFilesClient::new(self.transport.clone())
            .file_move(request).await
    }

    /// 复制文件或目录。
    pub async fn file_copy(&self, request: &CopyFileRequest) -> Result<OperationAck, String> {
        gateway_files::GatewayFilesClient::new(self.transport.clone())
            .file_copy(request).await
    }

    /// 在同一目录创建文件或文件夹的副本。
    pub async fn file_duplicate(
        &self,
        request: &DuplicateFileRequest,
    ) -> Result<OperationAck, String> {
        gateway_files::GatewayFilesClient::new(self.transport.clone())
            .file_duplicate(request).await
    }

    /// 将本机外部路径复制导入项目，仅允许连接本机网关。
    pub async fn file_import_local(
        &self,
        request: &LocalImportRequest,
    ) -> Result<LocalImportResponse, String> {
        gateway_files::GatewayFilesClient::new(self.transport.clone())
            .file_import_local(request).await
    }

    /// 删除文件或目录。
    pub async fn file_delete(&self, request: &DeleteFileRequest) -> Result<OperationAck, String> {
        gateway_files::GatewayFilesClient::new(self.transport.clone())
            .file_delete(request).await
    }

    /// 按条件搜索文件。
    pub async fn file_search(
        &self,
        request: &SearchFilesRequest,
    ) -> Result<SearchFilesResponse, String> {
        gateway_files::GatewayFilesClient::new(self.transport.clone())
            .file_search(request).await
    }

    /// 查询文件或目录状态。
    pub async fn file_stat(&self, request: &StatFileRequest) -> Result<StatFileResponse, String> {
        gateway_files::GatewayFilesClient::new(self.transport.clone())
            .file_stat(request).await
    }

    /// 读取文件内容。
    /// 在指定目录上下文中读取相对路径文件。
    ///
    /// `agent_key` 允许调用方把请求绑定到特定 agent 上下文；不需要时可以传 `None`。
    pub async fn file_read_in_directory(
        &self,
        directory: Option<&str>,
        agent_key: Option<&str>,
        path: &str,
    ) -> Result<DirectoryFileReadResponse, String> {
        gateway_files::GatewayFilesClient::new(self.transport.clone())
            .file_read_in_directory(directory, agent_key, path).await
    }

    /// 写入文件内容。
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
        gateway_files::GatewayFilesClient::new(self.transport.clone())
            .file_write_in_directory(directory, agent_key, path, content, create_if_missing).await
    }
}

#[cfg(test)]
#[path = "file_api_tests.rs"]
mod file_api_tests;
