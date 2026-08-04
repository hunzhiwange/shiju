//! 文件系统工具请求的处理与回调桥接。
//!
//! 本模块负责把工具层发出的文件系统请求转换为受控的本地文件操作，
//! 并在必要时接入确认回调或权限检查逻辑。
//!
//! 这里重点处理路径边界、写入前确认、返回值转换等胶水工作，
//! 以确保上层运行时不需要直接依赖具体的文件系统细节。

use std::future::Future;
use std::path::{Component, Path, PathBuf};
use std::pin::Pin;
use std::sync::Arc;

use agent_client_protocol::{
    ReadTextFileRequest, ReadTextFileResponse, WriteTextFileRequest, WriteTextFileResponse,
};

use crate::errors::{AcpxErrorOptions, ErrorSource, PermissionDeniedError};
use crate::types::{
    ClientOperation, ClientOperationMethod, ClientOperationStatus, NonInteractivePermissionPolicy,
    OutputErrorCode, OutputErrorOrigin, PermissionMode,
};

const WRITE_PREVIEW_MAX_LINES: usize = 16;
const WRITE_PREVIEW_MAX_CHARS: usize = 1_200;

pub type FileSystemConfirmWriteFuture =
    Pin<Box<dyn Future<Output = Result<bool, ErrorSource>> + Send + 'static>>;
pub type FileSystemConfirmWriteFn =
    Arc<dyn Fn(PathBuf, String) -> FileSystemConfirmWriteFuture + Send + Sync>;
pub type FileSystemOperationCallback = Arc<dyn Fn(ClientOperation) + Send + Sync>;

#[derive(Clone, Default)]
pub struct FileSystemHandlersOptions {
    pub cwd: PathBuf,
    pub permission_mode: PermissionMode,
    pub full_access: bool,
    pub non_interactive_permissions: Option<NonInteractivePermissionPolicy>,
    pub on_operation: Option<FileSystemOperationCallback>,
    pub confirm_write: Option<FileSystemConfirmWriteFn>,
}

#[derive(Clone)]
pub struct FileSystemHandlers {
    root_dir: PathBuf,
    permission_mode: PermissionMode,
    full_access: bool,
    on_operation: Option<FileSystemOperationCallback>,
    confirm_write: Option<FileSystemConfirmWriteFn>,
}

impl FileSystemHandlers {
    pub fn new(options: FileSystemHandlersOptions) -> Self {
        let root_dir = normalize_absolute_path(&options.cwd);
        Self {
            root_dir,
            permission_mode: options.permission_mode,
            full_access: options.full_access,
            on_operation: options.on_operation,
            confirm_write: options.confirm_write,
        }
    }

    pub fn update_permission_policy(
        &mut self,
        permission_mode: PermissionMode,
        _non_interactive_permissions: Option<NonInteractivePermissionPolicy>,
    ) {
        self.permission_mode = permission_mode;
    }

    pub async fn read_text_file(
        &self,
        params: &ReadTextFileRequest,
    ) -> Result<ReadTextFileResponse, ErrorSource> {
        let file_path = self.resolve_path_within_root(&params.path)?;
        let summary = format!("read_text_file: {}", file_path.display());
        self.emit_operation(ClientOperation {
            method: ClientOperationMethod::FsReadTextFile,
            status: ClientOperationStatus::Running,
            summary: summary.clone(),
            details: read_window_details(params.line, params.limit),
            timestamp: now_iso(),
        });

        let result = async {
            if self.permission_mode == PermissionMode::DenyAll {
                return Err(Box::new(permission_denied_error(
                    "Permission denied for fs/read_text_file (--deny-all)",
                )) as ErrorSource);
            }

            let content = tokio::fs::read_to_string(&file_path).await?;
            let sliced = slice_content(&content, params.line, params.limit);
            Ok(read_text_file_response(sliced))
        }
        .await;

        self.emit_completion(
            ClientOperationMethod::FsReadTextFile,
            summary,
            read_window_details(params.line, params.limit),
            result.as_ref().err().map(ToString::to_string),
        );
        result
    }

    pub async fn write_text_file(
        &self,
        params: &WriteTextFileRequest,
    ) -> Result<WriteTextFileResponse, ErrorSource> {
        let file_path = self.resolve_path_within_root(&params.path)?;
        let preview = to_write_preview(&params.content);
        let summary = format!("write_text_file: {}", file_path.display());
        self.emit_operation(ClientOperation {
            method: ClientOperationMethod::FsWriteTextFile,
            status: ClientOperationStatus::Running,
            summary: summary.clone(),
            details: Some(preview.clone()),
            timestamp: now_iso(),
        });

        let result = async {
            if !self.is_write_approved(&file_path, &preview).await? {
                return Err(Box::new(permission_denied_error(
                    "Permission denied for fs/write_text_file",
                )) as ErrorSource);
            }

            if let Some(parent) = file_path.parent() {
                tokio::fs::create_dir_all(parent).await?;
            }
            tokio::fs::write(&file_path, params.content.as_bytes()).await?;
            Ok(write_text_file_response())
        }
        .await;

        self.emit_completion(
            ClientOperationMethod::FsWriteTextFile,
            summary,
            Some(preview),
            result.as_ref().err().map(ToString::to_string),
        );
        result
    }

    async fn is_write_approved(
        &self,
        file_path: &Path,
        preview: &str,
    ) -> Result<bool, ErrorSource> {
        match self.permission_mode {
            PermissionMode::ApproveAll => Ok(true),
            PermissionMode::DenyAll => Ok(false),
            PermissionMode::ApproveReads => {
                if let Some(confirm_write) = &self.confirm_write {
                    return confirm_write(file_path.to_path_buf(), preview.to_string()).await;
                }
                Ok(true)
            }
        }
    }

    fn resolve_path_within_root(&self, raw_path: &Path) -> Result<PathBuf, ErrorSource> {
        if !raw_path.is_absolute() {
            return Err(format!("Path must be absolute: {}", raw_path.display()).into());
        }

        let resolved = normalize_absolute_path(raw_path);
        if !self.full_access && !is_within_root(&self.root_dir, &resolved) {
            return Err(
                format!("Path is outside allowed cwd subtree: {}", resolved.display()).into()
            );
        }
        Ok(resolved)
    }

    fn emit_operation(&self, operation: ClientOperation) {
        if let Some(on_operation) = &self.on_operation {
            on_operation(operation);
        }
    }

    fn emit_completion(
        &self,
        method: ClientOperationMethod,
        summary: String,
        success_details: Option<String>,
        error: Option<String>,
    ) {
        self.emit_operation(ClientOperation {
            method,
            status: if error.is_some() {
                ClientOperationStatus::Failed
            } else {
                ClientOperationStatus::Completed
            },
            summary,
            details: error.or(success_details),
            timestamp: now_iso(),
        });
    }
}

fn permission_denied_error(message: impl Into<String>) -> PermissionDeniedError {
    PermissionDeniedError::new(
        message,
        AcpxErrorOptions::default().with_defaults(
            OutputErrorCode::PermissionDenied,
            "PERMISSION_DENIED",
            OutputErrorOrigin::Runtime,
        ),
    )
}

fn now_iso() -> String {
    time::OffsetDateTime::now_utc()
        .format(&time::format_description::well_known::Rfc3339)
        .expect("RFC3339 formatting should succeed for UTC timestamps")
}

fn normalize_absolute_path(path: &Path) -> PathBuf {
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::Prefix(_) | Component::RootDir | Component::Normal(_) => {
                normalized.push(component.as_os_str());
            }
            Component::CurDir => {}
            Component::ParentDir => {
                normalized.pop();
            }
        }
    }
    normalized
}

fn is_within_root(root_dir: &Path, target_path: &Path) -> bool {
    target_path == root_dir || target_path.starts_with(root_dir)
}

fn to_write_preview(content: &str) -> String {
    let normalized = content.replace("\r\n", "\n");
    let lines: Vec<&str> = normalized.split('\n').collect();
    let visible_lines = &lines[..lines.len().min(WRITE_PREVIEW_MAX_LINES)];
    let mut preview = visible_lines.join("\n");

    if lines.len() > visible_lines.len() {
        preview.push_str(&format!("\n... ({} more lines)", lines.len() - visible_lines.len()));
    }

    if preview.chars().count() > WRITE_PREVIEW_MAX_CHARS {
        preview = preview.chars().take(WRITE_PREVIEW_MAX_CHARS - 3).collect::<String>() + "...";
    }

    preview
}

fn slice_content(content: &str, line: Option<u32>, limit: Option<u32>) -> String {
    if line.is_none() && limit.is_none() {
        return content.to_string();
    }

    let lines: Vec<&str> = content.split('\n').collect();
    let start_line = line.unwrap_or(1).max(1) as usize;
    let start_index = start_line.saturating_sub(1);
    let max_lines = limit.map(|value| value as usize);

    if max_lines == Some(0) {
        return String::new();
    }

    let end_index = max_lines
        .map(|value| lines.len().min(start_index.saturating_add(value)))
        .unwrap_or(lines.len());

    lines.get(start_index..end_index).unwrap_or(&[]).join("\n")
}

fn read_window_details(line: Option<u32>, limit: Option<u32>) -> Option<String> {
    if line.is_none() && limit.is_none() {
        return None;
    }
    Some(format!(
        "line={}, limit={}",
        line.unwrap_or(1).max(1),
        limit.map_or_else(|| "all".to_string(), |value| value.to_string())
    ))
}

fn read_text_file_response(content: String) -> ReadTextFileResponse {
    serde_json::from_value(serde_json::json!({ "content": content }))
        .expect("valid ACP read_text_file response")
}

fn write_text_file_response() -> WriteTextFileResponse {
    serde_json::from_value(serde_json::json!({})).expect("valid ACP write_text_file response")
}
