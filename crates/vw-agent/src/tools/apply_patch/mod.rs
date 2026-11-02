//! 补丁应用工具。
//!
//! `apply_patch` 保留现有的补丁解析、统一收集、路径安全与动作预算能力，
//! 但对 Claude Tools V2 运行时返回结构化结果：
//! - 结构化变更列表
//! - 结构化 patch hunks
//! - render hint / telemetry
//! - 精确的 read_state 更新与失效

use super::context::{
    current_read_state_for_path, current_tool_use_context, invalidate_current_read_state,
    note_current_read_state,
};
use super::external_directory;
use super::file_edit::types::{FileDescriptor, StructuredPatch};
use super::file_edit::utils::{
    build_file_descriptor, build_patch_summary, display_path, snapshot_from_text,
};
use super::traits::{Tool, ToolCallResult, ToolCallTelemetry, ToolRenderHint, ToolResult};
use crate::app::agent::file;
use crate::app::agent::file::watcher;
use crate::app::agent::patch;
use crate::app::agent::security::SecurityPolicy;
use anyhow::anyhow;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use vw_api_types::tools::{StructuredPatchHunkDto, ToolResultContentDto};

/// `apply_patch` 工具。
pub struct ApplyPatchTool {
    security: Arc<SecurityPolicy>,
}

#[derive(Debug, Clone, Deserialize)]
struct Args {
    #[serde(rename = "patchText", alias = "patch")]
    patch_text: String,
}

#[derive(Debug, Clone, Copy)]
enum ChangeKind {
    Add,
    Update,
    Delete,
    Move,
}

impl ChangeKind {
    fn operation(self) -> &'static str {
        match self {
            Self::Add => "add",
            Self::Update => "update",
            Self::Delete => "delete",
            Self::Move => "move",
        }
    }

    fn summary_code(self) -> &'static str {
        match self {
            Self::Add => "A",
            Self::Update | Self::Move => "M",
            Self::Delete => "D",
        }
    }
}

#[derive(Debug, Clone)]
struct FileChange {
    file_path: PathBuf,
    move_path: Option<PathBuf>,
    display_path: String,
    kind: ChangeKind,
    old_content: String,
    new_content: String,
    additions: usize,
    deletions: usize,
    patch_hunks: Vec<StructuredPatchHunkDto>,
    read_state_before: Option<Value>,
}

impl FileChange {
    fn target_path(&self) -> &Path {
        self.move_path.as_deref().unwrap_or(&self.file_path)
    }

    fn target_size_bytes(&self) -> u64 {
        match self.kind {
            ChangeKind::Delete => self.old_content.len() as u64,
            _ => self.new_content.len() as u64,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
struct ApplyPatchFileSummary {
    operation: String,
    file: FileDescriptor,
    additions: usize,
    deletions: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    from_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    read_state: Option<Value>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum ApplyPatchPayload {
    ApplyPatch { files: Vec<ApplyPatchFileSummary>, structured_patch: StructuredPatch },
}

#[derive(Debug, Clone)]
struct ApplyPatchResponse {
    model_text: String,
    payload: ApplyPatchPayload,
    render_hint: ToolRenderHint,
    patch_hunks: Vec<StructuredPatchHunkDto>,
    warnings: Vec<String>,
}

impl ApplyPatchResponse {
    fn into_tool_call_result(self) -> ToolCallResult {
        let content_blocks = if self.patch_hunks.is_empty() {
            Vec::new()
        } else {
            vec![ToolResultContentDto::StructuredPatch { hunks: self.patch_hunks }]
        };

        ToolCallResult {
            data: serde_json::to_value(&self.payload).unwrap_or(Value::Null),
            model_result: Value::String(self.model_text),
            content_blocks,
            render_hint: Some(self.render_hint),
            telemetry: Some(ToolCallTelemetry {
                success: true,
                warnings: self.warnings,
                ..ToolCallTelemetry::default()
            }),
            ..ToolCallResult::default()
        }
    }
}

impl ApplyPatchTool {
    pub fn new(security: Arc<SecurityPolicy>) -> Self {
        Self { security }
    }

    fn schema() -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "patchText": {
                    "type": "string",
                    "description": "Patch text in apply_patch envelope format."
                }
            },
            "required": ["patchText"]
        })
    }

    fn failure(msg: impl Into<String>) -> ToolResult {
        let msg = msg.into();
        ToolResult { success: false, output: msg.clone(), error: Some(msg) }
    }

    fn resolve_path(&self, path: &str) -> PathBuf {
        let candidate = PathBuf::from(path);
        if candidate.is_absolute() {
            candidate
        } else {
            self.security.workspace_dir.join(candidate)
        }
    }

    fn nearest_existing_ancestor(path: &Path) -> Option<PathBuf> {
        let mut current = Some(path);
        while let Some(candidate) = current {
            if candidate.exists() {
                return Some(candidate.to_path_buf());
            }
            current = candidate.parent();
        }
        None
    }

    async fn ensure_requested_path_allowed(&self, path: &str) -> anyhow::Result<()> {
        if !self.security.is_path_allowed(path) {
            anyhow::bail!("Path not allowed by security policy: {path}")
        }

        external_directory::assert_external_directory(
            &self.security,
            Some(path),
            Some(external_directory::Options {
                bypass: false,
                kind: external_directory::Kind::File,
            }),
        )
        .await
        .map_err(anyhow::Error::msg)
    }

    async fn ensure_target_parent_allowed(&self, path: &Path) -> anyhow::Result<()> {
        let parent = path.parent().ok_or_else(|| anyhow!("Invalid path: missing parent"))?;
        let anchor = Self::nearest_existing_ancestor(parent)
            .ok_or_else(|| anyhow!("Invalid path: cannot resolve parent"))?;
        let resolved = tokio::fs::canonicalize(&anchor)
            .await
            .map_err(|error| anyhow!("Failed to resolve path {}: {error}", anchor.display()))?;
        if !self.security.is_resolved_path_allowed(&resolved) {
            anyhow::bail!(self.security.resolved_path_violation_message(&resolved));
        }
        Ok(())
    }

    async fn ensure_existing_file_allowed(&self, path: &Path) -> anyhow::Result<PathBuf> {
        let meta = tokio::fs::symlink_metadata(path)
            .await
            .map_err(|error| anyhow!("Failed to read file metadata {}: {error}", path.display()))?;
        if meta.file_type().is_symlink() {
            anyhow::bail!("Refusing to operate through symlink: {}", path.display());
        }
        if !meta.is_file() {
            anyhow::bail!("Target is not a regular file: {}", path.display());
        }
        let resolved = tokio::fs::canonicalize(path)
            .await
            .map_err(|error| anyhow!("Failed to resolve file path {}: {error}", path.display()))?;
        if !self.security.is_resolved_path_allowed(&resolved) {
            anyhow::bail!(self.security.resolved_path_violation_message(&resolved));
        }
        Ok(resolved)
    }

    async fn ensure_target_not_symlink(&self, path: &Path) -> anyhow::Result<()> {
        if let Ok(meta) = tokio::fs::symlink_metadata(path).await
            && meta.file_type().is_symlink()
        {
            anyhow::bail!("Refusing to write through symlink: {}", path.display());
        }
        Ok(())
    }

    fn read_state_metadata(&self, source_path: &Path, display_path: &str) -> Option<Value> {
        if current_tool_use_context().is_none() {
            return None;
        }

        Some(match current_read_state_for_path(source_path) {
            Some(entry) => {
                let status = if entry.partial_view { "partial" } else { "full" };
                let message = if entry.partial_view {
                    "File was last read via a partial view in the current tool context before apply_patch."
                } else {
                    "File was read in the current tool context before apply_patch."
                };
                json!({
                    "status": status,
                    "path": display_path,
                    "message": message,
                    "bytesRead": entry.bytes_read,
                    "partialView": entry.partial_view,
                    "offset": entry.offset,
                    "limit": entry.limit,
                    "snapshot": entry.snapshot.as_ref().map(|snapshot| json!({
                        "sizeBytes": snapshot.size_bytes,
                        "contentDigest": snapshot.content_digest,
                    })),
                })
            }
            None => json!({
                "status": "unread",
                "path": display_path,
                "message": "No file_read state was recorded for this path in the current tool context before apply_patch."
            }),
        })
    }

    fn update_current_read_state(&self, change: &FileChange) {
        match change.kind {
            ChangeKind::Add | ChangeKind::Update => {
                let _ = note_current_read_state(
                    change.target_path(),
                    change.new_content.len(),
                    false,
                    None,
                    None,
                    Some(snapshot_from_text(&change.new_content)),
                );
            }
            ChangeKind::Move => {
                let _ = invalidate_current_read_state(&change.file_path);
                let _ = note_current_read_state(
                    change.target_path(),
                    change.new_content.len(),
                    false,
                    None,
                    None,
                    Some(snapshot_from_text(&change.new_content)),
                );
            }
            ChangeKind::Delete => {
                let _ = invalidate_current_read_state(&change.file_path);
            }
        }
    }

    fn publish_change(&self, change: &FileChange) {
        match change.kind {
            ChangeKind::Add => {
                file::publish_edited(change.display_path.clone());
                watcher::publish_updated(change.display_path.clone(), "add");
            }
            ChangeKind::Update => {
                file::publish_edited(change.display_path.clone());
                watcher::publish_updated(change.display_path.clone(), "change");
            }
            ChangeKind::Move => {
                let source_display = display_path(&self.security.workspace_dir, &change.file_path);
                file::publish_edited(change.display_path.clone());
                watcher::publish_updated(source_display, "remove");
                watcher::publish_updated(change.display_path.clone(), "add");
            }
            ChangeKind::Delete => {
                watcher::publish_updated(change.display_path.clone(), "remove");
            }
        }
    }

    async fn collect_changes(&self, patch_text: &str) -> anyhow::Result<Vec<FileChange>> {
        let parsed = patch::parse_patch(patch_text)
            .map_err(|error| anyhow!("apply_patch validation failed: {error}"))?;

        if parsed.hunks.is_empty() {
            let normalized = patch_text.replace("\r\n", "\n").replace('\r', "\n");
            let trimmed = normalized.trim();
            let empty_patch = trimmed == "*** Begin Patch\n*** End Patch"
                || trimmed == "*** 开始补丁\n*** 结束补丁";
            if empty_patch {
                anyhow::bail!("Patch rejected: empty patch");
            }
            anyhow::bail!("apply_patch validation failed: no patch hunks found");
        }

        let mut changes = Vec::new();

        for hunk in &parsed.hunks {
            match hunk {
                patch::Hunk::Add { path, contents } => {
                    self.ensure_requested_path_allowed(path).await?;
                    let file_path = self.resolve_path(path);
                    self.ensure_target_parent_allowed(&file_path).await?;

                    let mut new_content = contents.to_string();
                    if !new_content.is_empty() && !new_content.ends_with('\n') {
                        new_content.push('\n');
                    }

                    let display = display_path(&self.security.workspace_dir, &file_path);
                    let patch_summary = build_patch_summary(&display, "", &new_content);
                    changes.push(FileChange {
                        file_path,
                        move_path: None,
                        display_path: display,
                        kind: ChangeKind::Add,
                        old_content: String::new(),
                        new_content,
                        additions: patch_summary.additions,
                        deletions: patch_summary.deletions,
                        patch_hunks: patch_summary.hunks,
                        read_state_before: None,
                    });
                }
                patch::Hunk::Delete { path } => {
                    self.ensure_requested_path_allowed(path).await?;
                    let file_path = self.resolve_path(path);
                    let resolved = self.ensure_existing_file_allowed(&file_path).await?;
                    let old_content =
                        tokio::fs::read_to_string(&resolved).await.map_err(|error| {
                            anyhow!("Failed to read file {}: {error}", resolved.display())
                        })?;

                    let display = display_path(&self.security.workspace_dir, &resolved);
                    let patch_summary = build_patch_summary(&display, &old_content, "");
                    let read_state_before = self.read_state_metadata(&resolved, &display);
                    changes.push(FileChange {
                        file_path: resolved,
                        move_path: None,
                        display_path: display,
                        kind: ChangeKind::Delete,
                        old_content,
                        new_content: String::new(),
                        additions: patch_summary.additions,
                        deletions: patch_summary.deletions,
                        patch_hunks: patch_summary.hunks,
                        read_state_before,
                    });
                }
                patch::Hunk::Update { path, move_path, chunks } => {
                    self.ensure_requested_path_allowed(path).await?;
                    let source_path = self.resolve_path(path);
                    let resolved_source = self.ensure_existing_file_allowed(&source_path).await?;
                    let old_content =
                        tokio::fs::read_to_string(&resolved_source).await.map_err(|error| {
                            anyhow!("Failed to read file {}: {error}", resolved_source.display())
                        })?;
                    let update =
                        patch::derive_new_contents_from_chunks(&resolved_source, chunks.as_slice())
                            .map_err(|error| anyhow!("apply_patch validation failed: {error}"))?;

                    let mut resolved_move_path = None;
                    if let Some(destination) =
                        move_path.as_deref().filter(|value| !value.trim().is_empty())
                    {
                        self.ensure_requested_path_allowed(destination).await?;
                        let target = self.resolve_path(destination);
                        self.ensure_target_parent_allowed(&target).await?;
                        resolved_move_path = Some(target);
                    }

                    let change_kind = if resolved_move_path.is_some() {
                        ChangeKind::Move
                    } else {
                        ChangeKind::Update
                    };
                    let target_for_diff = resolved_move_path.as_ref().unwrap_or(&resolved_source);
                    let display = display_path(&self.security.workspace_dir, target_for_diff);
                    let patch_summary =
                        build_patch_summary(&display, &old_content, &update.content);
                    let read_state_before = self.read_state_metadata(&resolved_source, &display);
                    changes.push(FileChange {
                        file_path: resolved_source,
                        move_path: resolved_move_path,
                        display_path: display,
                        kind: change_kind,
                        old_content,
                        new_content: update.content,
                        additions: patch_summary.additions,
                        deletions: patch_summary.deletions,
                        patch_hunks: patch_summary.hunks,
                        read_state_before,
                    });
                }
            }
        }

        Ok(changes)
    }

    async fn apply_changes(&self, changes: &[FileChange]) -> anyhow::Result<()> {
        for change in changes {
            match change.kind {
                ChangeKind::Add => {
                    if let Some(parent) = change.file_path.parent() {
                        tokio::fs::create_dir_all(parent).await?;
                    }
                    self.ensure_target_not_symlink(&change.file_path).await?;
                    tokio::fs::write(&change.file_path, &change.new_content).await?;
                }
                ChangeKind::Update => {
                    self.ensure_target_not_symlink(&change.file_path).await?;
                    tokio::fs::write(&change.file_path, &change.new_content).await?;
                }
                ChangeKind::Move => {
                    let Some(target) = &change.move_path else {
                        anyhow::bail!("apply_patch validation failed: missing move path");
                    };
                    if let Some(parent) = target.parent() {
                        tokio::fs::create_dir_all(parent).await?;
                    }
                    self.ensure_target_not_symlink(target).await?;
                    tokio::fs::write(target, &change.new_content).await?;
                    tokio::fs::remove_file(&change.file_path).await?;
                }
                ChangeKind::Delete => {
                    tokio::fs::remove_file(&change.file_path).await?;
                }
            }

            self.publish_change(change);
            self.update_current_read_state(change);
        }

        Ok(())
    }

    fn build_response(&self, changes: Vec<FileChange>) -> ApplyPatchResponse {
        let mut patch_hunks = Vec::new();
        let mut warnings = Vec::new();
        let mut file_summaries = Vec::new();
        let mut summary_lines = Vec::new();
        let mut added = 0usize;
        let mut updated = 0usize;
        let mut deleted = 0usize;
        let mut moved = 0usize;

        for change in &changes {
            match change.kind {
                ChangeKind::Add => added += 1,
                ChangeKind::Update => updated += 1,
                ChangeKind::Delete => deleted += 1,
                ChangeKind::Move => moved += 1,
            }

            patch_hunks.extend(change.patch_hunks.clone());

            let from_path = change
                .move_path
                .as_ref()
                .map(|_| display_path(&self.security.workspace_dir, &change.file_path));
            if let Some(from_path) = from_path.as_ref() {
                summary_lines.push(format!("R {} -> {}", from_path, change.display_path));
            } else {
                summary_lines.push(format!(
                    "{} {}",
                    change.kind.summary_code(),
                    change.display_path
                ));
            }

            if let Some(status) = change
                .read_state_before
                .as_ref()
                .and_then(|value| value.get("status"))
                .and_then(Value::as_str)
                .filter(|status| *status != "full")
            {
                warnings.push(format!(
                    "{} was patched with {} file_read state in the current tool context",
                    change.display_path, status
                ));
            }

            let file = build_file_descriptor(
                &self.security.workspace_dir,
                change.target_path(),
                change.target_size_bytes(),
            );
            file_summaries.push(ApplyPatchFileSummary {
                operation: change.kind.operation().to_string(),
                file,
                additions: change.additions,
                deletions: change.deletions,
                from_path,
                read_state: change.read_state_before.clone(),
            });
        }

        let files_len = file_summaries.len();
        let model_text = if summary_lines.is_empty() {
            "Patch applied with no file changes.".to_string()
        } else {
            format!("Applied patch to {} file(s): {}", files_len, summary_lines.join(", "))
        };
        let render_summary = if files_len == 0 {
            "No file changes".to_string()
        } else {
            format!("Updated {} file(s)", files_len)
        };

        ApplyPatchResponse {
            model_text,
            payload: ApplyPatchPayload::ApplyPatch {
                files: file_summaries.clone(),
                structured_patch: StructuredPatch { hunks: patch_hunks.clone() },
            },
            render_hint: ToolRenderHint {
                title: Some("Applied patch".to_string()),
                kind: Some("apply_patch".to_string()),
                summary: Some(render_summary),
                metadata: json!({
                    "changedFiles": files_len,
                    "added": added,
                    "updated": updated,
                    "deleted": deleted,
                    "moved": moved,
                    "paths": file_summaries
                        .iter()
                        .map(|file| file.file.path.clone())
                        .collect::<Vec<_>>(),
                }),
            },
            patch_hunks,
            warnings,
        }
    }

    async fn execute_internal(&self, args: Args) -> anyhow::Result<ApplyPatchResponse> {
        let patch_text = args.patch_text.trim();
        if patch_text.is_empty() {
            anyhow::bail!("Missing patchText");
        }
        if !self.security.can_act() {
            anyhow::bail!("Action blocked: autonomy is read-only");
        }
        if self.security.is_rate_limited() {
            anyhow::bail!("Rate limit exceeded: too many actions in the last hour");
        }

        let changes = self.collect_changes(patch_text).await?;

        if !self.security.record_action() {
            anyhow::bail!("Rate limit exceeded: action budget exhausted");
        }

        self.apply_changes(&changes).await?;
        Ok(self.build_response(changes))
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl Tool for ApplyPatchTool {
    fn name(&self) -> &str {
        "apply_patch"
    }

    fn description(&self) -> &str {
        include_str!("apply_patch.txt")
    }

    fn parameters_schema(&self) -> serde_json::Value {
        Self::schema()
    }

    async fn execute(&self, args: serde_json::Value) -> anyhow::Result<ToolResult> {
        let args: Args = serde_json::from_value(crate::plugins::patch_input(args))
            .map_err(|error| anyhow!("Missing or invalid 'patchText' parameter: {error}"))?;

        match self.execute_internal(args).await {
            Ok(response) => {
                Ok(ToolResult { success: true, output: response.model_text, error: None })
            }
            Err(error) => Ok(Self::failure(error.to_string())),
        }
    }

    async fn call(&self, input: Value) -> anyhow::Result<ToolCallResult> {
        let args: Args = serde_json::from_value(crate::plugins::patch_input(input))
            .map_err(|error| anyhow!("Missing or invalid 'patchText' parameter: {error}"))?;

        match self.execute_internal(args).await {
            Ok(response) => Ok(response.into_tool_call_result()),
            Err(error) => Ok(ToolCallResult::from_legacy_result(Self::failure(error.to_string()))),
        }
    }
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;

#[cfg(any())]
mod obsolete_apply_patch_impl {
    //! 补丁应用工具
    //!
    //! 本模块实现了统一格式（unified diff）补丁的应用功能，用于将补丁应用到工作区文件。
    //!
    //! # 主要功能
    //!
    //! - **添加文件**：创建新文件并写入内容
    //! - **修改文件**：对现有文件进行增量更新
    //! - **删除文件**：从工作区移除文件
    //! - **移动文件**：将文件移动到新位置并可选地修改内容
    //!
    //! # 安全特性
    //!
    //! 所有操作都受安全策略约束：
    //! - 路径访问控制：仅允许操作安全策略许可的路径
    //! - 符号链接保护：拒绝通过符号链接进行操作以防止逃逸
    //! - 操作频率限制：防止过多的自动化操作
    //! - 只读模式支持：可在只读模式下预览变更而不实际执行
    //!
    //! # 补丁格式
    //!
    //! 支持的补丁格式由 `patch` 模块定义，包括：
    //! - `*** Add` 块用于添加新文件
    //! - `*** Delete` 块用于删除文件
    //! - `*** Update` 块用于修改现有文件（支持移动）
    //!
    //! # 示例
    //!
    //! ```ignore
    //! use std::sync::Arc;
    //! use crate::app::agent::tools::apply_patch::ApplyPatchTool;
    //! use crate::app::agent::security::SecurityPolicy;
    //!
    //! let security = Arc::new(SecurityPolicy::default());
    //! let tool = ApplyPatchTool::new(security);
    //! // 调用 execute 方法执行补丁
    //! ```

    use crate::app::agent::patch;
    use crate::app::agent::security::SecurityPolicy;
    use crate::app::agent::tools::context::{
        current_read_state_for_path, current_tool_use_context,
    };
    use crate::app::agent::tools::traits::{Tool, ToolResult};
    use anyhow::anyhow;
    use async_trait::async_trait;
    use serde::Deserialize;
    use serde_json::{Value, json};
    use std::path::{Path, PathBuf};
    use std::sync::Arc;

    /// 补丁应用工具
    ///
    /// 实现了 [`Tool`] trait，用于解析和应用统一格式的补丁到工作区文件。
    /// 该工具在执行任何文件操作前会进行严格的安全检查，确保所有操作
    /// 都在安全策略允许的范围内进行。
    ///
    /// # 线程安全
    ///
    /// 该结构体内部使用 `Arc<SecurityPolicy>` 共享安全策略，
    /// 可以安全地在多个线程间共享。
    ///
    /// # 示例
    ///
    /// ```ignore
    /// let security = Arc::new(SecurityPolicy::default());
    /// let tool = ApplyPatchTool::new(security);
    /// let result = tool.execute(args).await?;
    /// ```
    pub struct ApplyPatchTool {
        /// 安全策略引用，用于路径验证和操作权限控制
        security: Arc<SecurityPolicy>,
    }

    /// 补丁应用工具的参数结构
    ///
    /// 用于反序列化 `execute` 方法接收的 JSON 参数。
    /// 支持两种参数名称：`patchText`（首选）和 `patch`（别名）。
    ///
    /// # 示例 JSON
    ///
    /// ```json
    /// {
    ///     "patchText": "*** Begin Patch\n*** Add path: new.txt\n+new content\n*** End Patch"
    /// }
    /// ```
    #[derive(Debug, Deserialize)]
    struct Args {
        /// 补丁文本内容
        ///
        /// 支持统一格式的补丁文本，包含添加、修改、删除等操作指令。
        /// 该字段可通过 `patchText` 或 `patch`（别名）指定。
        #[serde(rename = "patchText", alias = "patch")]
        patch_text: String,
    }

    /// 文件变更类型枚举
    ///
    /// 定义了补丁操作可能产生的四种文件变更类型。
    /// 每种类型对应不同的文件系统操作和输出格式。
    #[derive(Debug, Clone)]
    enum ChangeKind {
        /// 添加新文件
        ///
        /// 创建一个新文件。如果文件已存在，将被覆盖。
        /// 在输出摘要中显示为 `A <path>`。
        Add,

        /// 更新现有文件
        ///
        /// 修改现有文件的内容。文件必须已存在。
        /// 在输出摘要中显示为 `M <path>`。
        Update,

        /// 删除文件
        ///
        /// 从文件系统中移除指定文件。文件必须已存在。
        /// 在输出摘要中显示为 `D <path>`。
        Delete,

        /// 移动文件
        ///
        /// 将文件移动到新位置，同时可能修改其内容。
        /// 在输出摘要中显示为 `M <new_path>`。
        Move,
    }

    /// 文件变更记录
    ///
    /// 记录单个文件变更的所有相关信息，包括变更前后的内容、
    /// 变更类型以及统计信息（添加/删除行数）。
    ///
    /// 该结构体用于在验证阶段收集所有变更，然后统一执行，
    /// 确保要么全部成功，要么全部失败（事务性语义）。
    #[derive(Debug, Clone)]
    struct FileChange {
        /// 文件的完整路径
        ///
        /// 对于移动操作，这是源文件路径。
        file_path: PathBuf,

        /// 移动操作的目标路径
        ///
        /// 仅当 [`ChangeKind::Move`] 时有效，其他情况下为 `None`。
        move_path: Option<PathBuf>,

        /// 变更类型
        kind: ChangeKind,

        /// 变更前的文件内容
        ///
        /// 对于添加操作，内容为空字符串。
        old_content: String,

        /// 变更后的文件内容
        ///
        /// 对于删除操作，内容为空字符串。
        new_content: String,

        /// 新增行数
        ///
        /// 统计补丁中新增的行数，用于变更摘要。
        additions: usize,

        /// 删除行数
        ///
        /// 统计补丁中删除的行数，用于变更摘要。
        deletions: usize,
    }

    impl ApplyPatchTool {
        /// 创建新的补丁应用工具实例
        ///
        /// # 参数
        ///
        /// - `security`: 安全策略的共享引用，用于控制文件访问权限和操作限制
        ///
        /// # 返回值
        ///
        /// 返回配置好的 `ApplyPatchTool` 实例
        ///
        /// # 示例
        ///
        /// ```ignore
        /// let security = Arc::new(SecurityPolicy::default());
        /// let tool = ApplyPatchTool::new(security);
        /// ```
        pub fn new(security: Arc<SecurityPolicy>) -> Self {
            Self { security }
        }

        /// 返回工具参数的 JSON Schema
        ///
        /// 定义了 `execute` 方法所期望的参数格式，用于工具调用时的参数验证。
        ///
        /// # 返回值
        ///
        /// 返回一个 JSON Schema 对象，描述 `patchText` 参数：
        /// - 类型：字符串
        /// - 必填：是
        fn schema() -> serde_json::Value {
            json!({
                "type": "object",
                "properties": {
                    "patchText": {
                        "type": "string",
                        "description": "Patch text in apply_patch envelope format."
                    }
                },
                "required": ["patchText"]
            })
        }

        /// 将路径解析为绝对路径
        ///
        /// 如果输入路径是相对路径，则将其与工作区目录拼接；
        /// 如果是绝对路径，则直接返回。
        ///
        /// # 参数
        ///
        /// - `path`: 待解析的路径字符串
        ///
        /// # 返回值
        ///
        /// 返回解析后的完整路径
        ///
        /// # 注意
        ///
        /// 此方法不验证路径是否存在或是否被允许访问，
        /// 仅进行路径拼接。安全检查在后续步骤进行。
        fn resolve_path(&self, path: &str) -> PathBuf {
            let p = PathBuf::from(path);
            if p.is_absolute() {
                return p;
            }
            self.security.workspace_dir.join(p)
        }

        /// 规范化路径中的斜杠
        ///
        /// 将路径中的反斜杠（Windows 风格）转换为正斜杠（Unix 风格），
        /// 确保输出路径在不同操作系统间保持一致。
        ///
        /// # 参数
        ///
        /// - `path`: 待规范化的路径
        ///
        /// # 返回值
        ///
        /// 返回使用正斜杠的路径字符串
        fn normalize_slashes(path: &Path) -> String {
            path.to_string_lossy().replace('\\', "/")
        }

        /// 获取相对于工作区目录的路径表示
        ///
        /// 如果路径位于工作区目录内，返回相对路径；否则返回原始路径。
        /// 用于在输出中显示更简洁、用户友好的路径。
        ///
        /// # 参数
        ///
        /// - `full`: 文件的完整绝对路径
        ///
        /// # 返回值
        ///
        /// 返回适合显示的路径字符串：
        /// - 工作区内的文件：相对路径
        /// - 工作区外的文件：完整路径（经过斜杠规范化）
        fn output_path(&self, full: &Path) -> String {
            if let Ok(rel) = full.strip_prefix(&self.security.workspace_dir) {
                return Self::normalize_slashes(rel);
            }
            Self::normalize_slashes(full)
        }

        /// 查找路径的最近已存在祖先目录
        ///
        /// 从给定路径开始向上遍历，找到第一个实际存在的路径。
        /// 用于在创建新文件前验证父目录的访问权限。
        ///
        /// # 参数
        ///
        /// - `path`: 起始路径
        ///
        /// # 返回值
        ///
        /// - `Some(PathBuf)`: 找到的最近已存在祖先路径
        /// - `None`: 没有找到任何存在的祖先（已到达文件系统根目录）
        fn nearest_existing_ancestor(path: &Path) -> Option<PathBuf> {
            let mut current = Some(path);
            while let Some(p) = current {
                if p.exists() {
                    return Some(p.to_path_buf());
                }
                current = p.parent();
            }
            None
        }

        /// 确保目标文件的父目录被安全策略允许
        ///
        /// 对于新创建的文件，无法直接使用 `canonicalize`（因为文件尚不存在），
        /// 因此通过查找最近的已存在祖先目录并进行验证。
        ///
        /// # 参数
        ///
        /// - `path`: 目标文件的完整路径（文件可能尚不存在）
        ///
        /// # 返回值
        ///
        /// - `Ok(())`: 父目录路径被允许访问
        /// - `Err`: 父目录路径不被允许或无法解析
        ///
        /// # 错误
        ///
        /// - 路径缺少父目录
        /// - 无法找到任何已存在的祖先目录
        /// - 解析后的路径违反安全策略
        async fn ensure_target_parent_allowed(&self, path: &Path) -> anyhow::Result<()> {
            // 获取父目录，如果没有父目录则报错
            let parent = path.parent().ok_or_else(|| anyhow!("Invalid path: missing parent"))?;
            // 查找最近的已存在祖先目录
            let anchor = Self::nearest_existing_ancestor(parent)
                .ok_or_else(|| anyhow!("Invalid path: cannot resolve parent"))?;
            // 解析祖先目录的真实路径（跟随符号链接）
            let resolved = tokio::fs::canonicalize(&anchor)
                .await
                .map_err(|e| anyhow!("Failed to resolve path {}: {e}", anchor.display()))?;
            // 验证解析后的路径是否在安全策略允许范围内
            if !self.security.is_resolved_path_allowed(&resolved) {
                return Err(anyhow!(self.security.resolved_path_violation_message(&resolved)));
            }
            Ok(())
        }

        /// 确保现有文件被安全策略允许
        ///
        /// 验证现有文件：1) 确实是常规文件而非目录或符号链接；2) 其真实路径被安全策略允许。
        ///
        /// # 参数
        ///
        /// - `path`: 待验证的文件路径
        ///
        /// # 返回值
        ///
        /// - `Ok(PathBuf)`: 文件验证通过，返回解析后的真实路径
        /// - `Err`: 验证失败
        ///
        /// # 错误
        ///
        /// - 无法读取文件元数据
        /// - 文件是符号链接（拒绝操作以防止符号链接攻击）
        /// - 目标不是常规文件
        /// - 解析后的路径违反安全策略
        async fn ensure_existing_file_allowed(&self, path: &Path) -> anyhow::Result<PathBuf> {
            // 获取文件元数据（不跟随符号链接）
            let meta = tokio::fs::symlink_metadata(path)
                .await
                .map_err(|e| anyhow!("Failed to read file metadata {}: {e}", path.display()))?;
            // 拒绝通过符号链接操作，防止符号链接逃逸攻击
            if meta.file_type().is_symlink() {
                return Err(anyhow!("Refusing to operate through symlink: {}", path.display()));
            }
            // 确保目标是常规文件，而非目录或其他特殊文件
            if !meta.is_file() {
                return Err(anyhow!("Target is not a regular file: {}", path.display()));
            }
            // 解析文件的真实路径（跟随符号链接以获取最终目标）
            let resolved = tokio::fs::canonicalize(path)
                .await
                .map_err(|e| anyhow!("Failed to resolve file path {}: {e}", path.display()))?;
            // 验证解析后的路径是否在安全策略允许范围内
            if !self.security.is_resolved_path_allowed(&resolved) {
                return Err(anyhow!(self.security.resolved_path_violation_message(&resolved)));
            }
            Ok(resolved)
        }

        /// 确保目标路径不是符号链接
        ///
        /// 在写入文件前检查，防止通过预先放置的符号链接进行写入攻击。
        /// 这是一个额外的安全检查，确保我们不会无意中通过符号链接写入到意外位置。
        ///
        /// # 参数
        ///
        /// - `path`: 待检查的路径
        ///
        /// # 返回值
        ///
        /// - `Ok(())`: 路径不是符号链接或不存在
        /// - `Err`: 路径是符号链接
        async fn ensure_target_not_symlink(&self, path: &Path) -> anyhow::Result<()> {
            // 尝试获取元数据，如果文件不存在则通过检查
            if let Ok(meta) = tokio::fs::symlink_metadata(path).await {
                // 如果文件存在且是符号链接，拒绝操作
                if meta.file_type().is_symlink() {
                    return Err(anyhow!("Refusing to write through symlink: {}", path.display()));
                }
            }
            Ok(())
        }

        /// 生成统一格式的差异输出
        ///
        /// 比较新旧内容并生成类似 `diff -u` 格式的输出，同时统计变更行数。
        ///
        /// # 参数
        ///
        /// - `filepath`: 文件路径（用于差异头部）
        /// - `old`: 变更前的内容
        /// - `new`: 变更后的内容
        ///
        /// # 返回值
        ///
        /// 返回元组 `(diff_string, additions, deletions)`：
        /// - `diff_string`: 统一格式的差异文本
        /// - `additions`: 新增行数
        /// - `deletions`: 删除行数
        fn unified_diff(filepath: &Path, old: &str, new: &str) -> (String, usize, usize) {
            // 规范化文件路径用于差异头部
            let name = Self::normalize_slashes(filepath);
            // 使用 similar 库生成统一格式差异，包含标准的 ---+++ 头部
            let diff = similar::TextDiff::from_lines(old, new)
                .unified_diff()
                .header(&name, &name)
                .to_string();
            // 统计新增和删除的行数
            let mut additions = 0usize;
            let mut deletions = 0usize;
            for change in similar::TextDiff::from_lines(old, new).iter_all_changes() {
                match change.tag() {
                    similar::ChangeTag::Insert => additions += 1,
                    similar::ChangeTag::Delete => deletions += 1,
                    similar::ChangeTag::Equal => {}
                }
            }
            (diff.trim().to_string(), additions, deletions)
        }

        fn read_state_metadata(&self, change: &FileChange, target: &Path) -> Option<Value> {
            if current_tool_use_context().is_none() || matches!(change.kind, ChangeKind::Add) {
                return None;
            }

            let path = self.output_path(target);
            Some(match current_read_state_for_path(&change.file_path) {
                Some(entry) => {
                    let status = if entry.partial_view { "partial" } else { "full" };
                    let message = if entry.partial_view {
                        "File was last read via a partial view in the current tool context before apply_patch."
                    } else {
                        "File was read in the current tool context before apply_patch."
                    };
                    json!({
                        "status": status,
                        "path": path,
                        "message": message,
                        "bytesRead": entry.bytes_read,
                        "partialView": entry.partial_view,
                        "offset": entry.offset,
                        "limit": entry.limit,
                    })
                }
                None => json!({
                    "status": "unread",
                    "path": path,
                    "message": "No file_read state was recorded for this path in the current tool context before apply_patch."
                }),
            })
        }

        fn append_read_state_block(output: &mut String, warnings: &[Value]) {
            if warnings.is_empty() {
                return;
            }
            output.push_str("\n\n<read_state>\n");
            output.push_str(&json!({ "files": warnings }).to_string());
            output.push_str("\n</read_state>");
        }
    }

    #[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
    #[cfg_attr(not(target_arch = "wasm32"), async_trait)]
    impl Tool for ApplyPatchTool {
        /// 返回工具名称
        ///
        /// 工具名称为 `"apply_patch"`，用于在工具注册表中标识此工具。
        fn name(&self) -> &str {
            "apply_patch"
        }

        /// 返回工具描述
        ///
        /// 从 `apply_patch.txt` 文件加载详细的工具描述，
        /// 包含使用说明、格式规范和示例。
        fn description(&self) -> &str {
            include_str!("apply_patch.txt")
        }

        /// 返回工具参数的 JSON Schema
        ///
        /// 委托给 [`Self::schema()`] 方法，返回参数验证模式。
        fn parameters_schema(&self) -> serde_json::Value {
            Self::schema()
        }

        /// 执行补丁应用操作
        ///
        /// 解析补丁文本，验证所有操作的安全性和有效性，
        /// 然后执行文件变更。
        ///
        /// # 参数
        ///
        /// - `args`: JSON 格式的参数，必须包含 `patchText` 字段
        ///
        /// # 返回值
        ///
        /// 返回 `ToolResult`，包含：
        /// - `success`: 操作是否成功
        /// - `output`: 成功时包含变更摘要、差异输出和 JSON 格式的详细变更信息
        /// - `error`: 失败时的错误信息
        ///
        /// # 执行流程
        ///
        /// 1. 解析并验证输入参数
        /// 2. 解析补丁文本，提取所有操作块
        /// 3. 对每个操作块进行安全验证并收集变更
        /// 4. 检查操作权限（只读模式、频率限制）
        /// 5. 原子性地执行所有文件变更
        /// 6. 生成并返回结果摘要
        ///
        /// # 错误处理
        ///
        /// 所有错误都通过 `ToolResult::error` 返回，不会 panic。
        /// 错误情况包括：
        /// - 参数缺失或格式错误
        /// - 补丁文本格式错误
        /// - 路径不在安全策略允许范围内
        /// - 文件系统操作失败
        /// - 频率限制
        async fn execute(&self, args: serde_json::Value) -> anyhow::Result<ToolResult> {
            // 解析输入参数
            let parsed_args: Args = serde_json::from_value(crate::plugins::patch_input(args))
                .map_err(|e| anyhow!("Missing or invalid 'patchText' parameter: {e}"))?;
            let patch_text = parsed_args.patch_text.trim();

            // 拒绝空补丁
            if patch_text.is_empty() {
                return Ok(ToolResult {
                    success: false,
                    output: String::new(),
                    error: Some("Missing patchText".to_string()),
                });
            }

            // 解析补丁文本
            let parsed = match patch::parse_patch(patch_text) {
                Ok(v) => v,
                Err(e) => {
                    return Ok(ToolResult {
                        success: false,
                        output: String::new(),
                        error: Some(format!("apply_patch validation failed: {e}")),
                    });
                }
            };

            // 验证补丁是否包含有效的操作块
            if parsed.hunks.is_empty() {
                // 规范化换行符以便检测空补丁模式
                let normalized =
                    patch_text.replace("\r\n", "\n").replace('\r', "\n").trim().to_string();
                // 检测常见的空补丁模式（中英文）
                let empty_patch = normalized == "*** Begin Patch\n*** End Patch"
                    || normalized == "*** 开始补丁\n*** 结束补丁";
                let msg = if empty_patch {
                    "Patch rejected: empty patch"
                } else {
                    "apply_patch validation failed: no patch hunks found"
                };
                return Ok(ToolResult {
                    success: false,
                    output: String::new(),
                    error: Some(msg.to_string()),
                });
            }

            // 收集所有文件变更，用于后续统一执行
            let mut changes = Vec::<FileChange>::new();
            // 累积所有差异输出，用于最终摘要
            let mut total_diff = String::new();

            // 处理每个补丁块
            for hunk in &parsed.hunks {
                match hunk {
                    // 处理添加文件操作
                    patch::Hunk::Add { path, contents } => {
                        // 安全策略检查：路径是否被允许
                        if !self.security.is_path_allowed(path) {
                            return Ok(ToolResult {
                                success: false,
                                output: String::new(),
                                error: Some(format!("Path not allowed by security policy: {path}")),
                            });
                        }

                        let file_path = self.resolve_path(path);
                        // 验证父目录的访问权限
                        if let Err(e) = self.ensure_target_parent_allowed(&file_path).await {
                            return Ok(ToolResult {
                                success: false,
                                output: String::new(),
                                error: Some(e.to_string()),
                            });
                        }

                        // 准备文件内容：确保以换行符结尾
                        let old_content = String::new();
                        let mut new_content = contents.to_string();
                        if !new_content.is_empty() && !new_content.ends_with('\n') {
                            new_content.push('\n');
                        }
                        // 生成差异输出和统计信息
                        let (diff, additions, deletions) =
                            Self::unified_diff(&file_path, &old_content, &new_content);
                        total_diff.push_str(&diff);
                        total_diff.push('\n');

                        changes.push(FileChange {
                            file_path,
                            move_path: None,
                            kind: ChangeKind::Add,
                            old_content,
                            new_content,
                            additions,
                            deletions,
                        });
                    }
                    // 处理删除文件操作
                    patch::Hunk::Delete { path } => {
                        // 安全策略检查：路径是否被允许
                        if !self.security.is_path_allowed(path) {
                            return Ok(ToolResult {
                                success: false,
                                output: String::new(),
                                error: Some(format!("Path not allowed by security policy: {path}")),
                            });
                        }

                        let file_path = self.resolve_path(path);
                        // 验证现有文件的访问权限并获取真实路径
                        let resolved = match self.ensure_existing_file_allowed(&file_path).await {
                            Ok(p) => p,
                            Err(e) => {
                                return Ok(ToolResult {
                                    success: false,
                                    output: String::new(),
                                    error: Some(e.to_string()),
                                });
                            }
                        };

                        // 读取现有内容用于差异生成
                        let old_content = match tokio::fs::read_to_string(&resolved).await {
                            Ok(c) => c,
                            Err(e) => {
                                return Ok(ToolResult {
                                    success: false,
                                    output: String::new(),
                                    error: Some(format!(
                                        "Failed to read file {}: {e}",
                                        resolved.display()
                                    )),
                                });
                            }
                        };
                        let new_content = String::new();
                        // 生成差异输出（显示所有内容被删除）
                        let (diff, additions, deletions) =
                            Self::unified_diff(&resolved, &old_content, &new_content);
                        total_diff.push_str(&diff);
                        total_diff.push('\n');

                        changes.push(FileChange {
                            file_path: resolved,
                            move_path: None,
                            kind: ChangeKind::Delete,
                            old_content,
                            new_content,
                            additions,
                            deletions,
                        });
                    }
                    // 处理更新文件操作（包括可选的移动）
                    patch::Hunk::Update { path, move_path, chunks } => {
                        // 安全策略检查：源路径是否被允许
                        if !self.security.is_path_allowed(path) {
                            return Ok(ToolResult {
                                success: false,
                                output: String::new(),
                                error: Some(format!("Path not allowed by security policy: {path}")),
                            });
                        }

                        let source_path = self.resolve_path(path);
                        // 验证现有文件的访问权限并获取真实路径
                        let resolved_source =
                            match self.ensure_existing_file_allowed(&source_path).await {
                                Ok(p) => p,
                                Err(e) => {
                                    return Ok(ToolResult {
                                        success: false,
                                        output: String::new(),
                                        error: Some(e.to_string()),
                                    });
                                }
                            };

                        // 读取现有内容
                        let old_content = match tokio::fs::read_to_string(&resolved_source).await {
                            Ok(c) => c,
                            Err(e) => {
                                return Ok(ToolResult {
                                    success: false,
                                    output: String::new(),
                                    error: Some(format!(
                                        "Failed to read file {}: {e}",
                                        resolved_source.display()
                                    )),
                                });
                            }
                        };

                        // 根据补丁块计算新内容
                        let update = match patch::derive_new_contents_from_chunks(
                            &resolved_source,
                            chunks,
                        ) {
                            Ok(v) => v,
                            Err(e) => {
                                return Ok(ToolResult {
                                    success: false,
                                    output: String::new(),
                                    error: Some(format!("apply_patch validation failed: {e}")),
                                });
                            }
                        };

                        // 处理可选的移动操作
                        let mut resolved_move_path = None;
                        if let Some(mp) = move_path.as_deref().filter(|s| !s.trim().is_empty()) {
                            // 安全策略检查：目标路径是否被允许
                            if !self.security.is_path_allowed(mp) {
                                return Ok(ToolResult {
                                    success: false,
                                    output: String::new(),
                                    error: Some(format!(
                                        "Path not allowed by security policy: {mp}"
                                    )),
                                });
                            }

                            let target = self.resolve_path(mp);
                            // 验证目标父目录的访问权限
                            if let Err(e) = self.ensure_target_parent_allowed(&target).await {
                                return Ok(ToolResult {
                                    success: false,
                                    output: String::new(),
                                    error: Some(e.to_string()),
                                });
                            }
                            resolved_move_path = Some(target);
                        }

                        // 确定用于差异显示的目标路径（移动时使用新路径）
                        let target_for_diff =
                            resolved_move_path.as_ref().unwrap_or(&resolved_source);
                        // 生成差异输出
                        let (diff, additions, deletions) =
                            Self::unified_diff(target_for_diff, &old_content, &update.content);
                        total_diff.push_str(&diff);
                        total_diff.push('\n');

                        // 根据是否有移动目标确定变更类型
                        let change_kind = if resolved_move_path.is_some() {
                            ChangeKind::Move
                        } else {
                            ChangeKind::Update
                        };

                        changes.push(FileChange {
                            file_path: resolved_source,
                            move_path: resolved_move_path,
                            kind: change_kind,
                            old_content,
                            new_content: update.content,
                            additions,
                            deletions,
                        });
                    }
                }
            }

            // === 执行前权限检查 ===

            // 检查是否处于只读模式
            if !self.security.can_act() {
                return Ok(ToolResult {
                    success: false,
                    output: String::new(),
                    error: Some("Action blocked: autonomy is read-only".into()),
                });
            }

            // 检查小时级频率限制
            if self.security.is_rate_limited() {
                return Ok(ToolResult {
                    success: false,
                    output: String::new(),
                    error: Some("Rate limit exceeded: too many actions in the last hour".into()),
                });
            }

            // 记录本次操作并检查操作预算
            if !self.security.record_action() {
                return Ok(ToolResult {
                    success: false,
                    output: String::new(),
                    error: Some("Rate limit exceeded: action budget exhausted".into()),
                });
            }

            // === 执行文件变更 ===

            for change in &changes {
                match change.kind {
                    // 执行添加文件操作
                    ChangeKind::Add => {
                        // 确保父目录存在
                        if let Some(parent) = change.file_path.parent() {
                            tokio::fs::create_dir_all(parent).await?;
                        }
                        // 检查目标不是符号链接
                        if let Err(e) = self.ensure_target_not_symlink(&change.file_path).await {
                            return Ok(ToolResult {
                                success: false,
                                output: String::new(),
                                error: Some(e.to_string()),
                            });
                        }
                        // 写入文件内容
                        tokio::fs::write(&change.file_path, &change.new_content).await?;
                    }
                    // 执行更新文件操作
                    ChangeKind::Update => {
                        // 检查目标不是符号链接
                        if let Err(e) = self.ensure_target_not_symlink(&change.file_path).await {
                            return Ok(ToolResult {
                                success: false,
                                output: String::new(),
                                error: Some(e.to_string()),
                            });
                        }
                        // 写入新内容
                        tokio::fs::write(&change.file_path, &change.new_content).await?;
                    }
                    // 执行移动文件操作
                    ChangeKind::Move => {
                        let Some(target) = &change.move_path else {
                            return Ok(ToolResult {
                                success: false,
                                output: String::new(),
                                error: Some(
                                    "apply_patch validation failed: missing move path".into(),
                                ),
                            });
                        };
                        // 确保目标父目录存在
                        if let Some(parent) = target.parent() {
                            tokio::fs::create_dir_all(parent).await?;
                        }
                        // 检查目标不是符号链接
                        if let Err(e) = self.ensure_target_not_symlink(target).await {
                            return Ok(ToolResult {
                                success: false,
                                output: String::new(),
                                error: Some(e.to_string()),
                            });
                        }
                        // 在新位置写入内容
                        tokio::fs::write(target, &change.new_content).await?;
                        // 删除原文件
                        tokio::fs::remove_file(&change.file_path).await?;
                    }
                    // 执行删除文件操作
                    ChangeKind::Delete => {
                        tokio::fs::remove_file(&change.file_path).await?;
                    }
                }
            }

            // === 生成输出摘要 ===

            // 生成简短的变更列表（类似 git status 格式）
            let summary_lines = changes
                .iter()
                .map(|change| match change.kind {
                    ChangeKind::Add => format!("A {}", self.output_path(&change.file_path)),
                    ChangeKind::Delete => format!("D {}", self.output_path(&change.file_path)),
                    ChangeKind::Update | ChangeKind::Move => {
                        let target = change.move_path.as_ref().unwrap_or(&change.file_path);
                        format!("M {}", self.output_path(target))
                    }
                })
                .collect::<Vec<_>>();

            // 构建主输出内容
            let mut output =
                format!("Success. Updated the following files:\n{}", summary_lines.join("\n"));
            // 附加差异输出
            let diff = total_diff.trim();
            if !diff.is_empty() {
                output.push_str("\n\n<diff>\n");
                output.push_str(diff);
                output.push_str("\n</diff>");
            }

            // 生成 JSON 格式的详细变更信息，便于程序解析
            let mut read_state_warnings = Vec::new();
            let files = changes
                .iter()
                .map(|change| {
                    // 确定变更类型代码和目标路径
                    let (kind, target) = match change.kind {
                        ChangeKind::Add => ("A", &change.file_path),
                        ChangeKind::Delete => ("D", &change.file_path),
                        ChangeKind::Update | ChangeKind::Move => {
                            ("M", change.move_path.as_ref().unwrap_or(&change.file_path))
                        }
                    };
                    let read_state = self.read_state_metadata(change, target);
                    if let Some(read_state) = read_state.as_ref()
                        && read_state
                            .get("status")
                            .and_then(Value::as_str)
                            .is_some_and(|status| status != "full")
                    {
                        read_state_warnings.push(read_state.clone());
                    }
                    let mut file = json!({
                        "kind": kind,
                        "path": self.output_path(target),
                        "absPath": Self::normalize_slashes(target),
                        "additions": change.additions,
                        "deletions": change.deletions,
                        "before": change.old_content,
                        "after": change.new_content,
                    });
                    if let Some(read_state) = read_state
                        && let Some(file_object) = file.as_object_mut()
                    {
                        file_object.insert("readState".to_string(), read_state);
                    }
                    file
                })
                .collect::<Vec<_>>();
            let changes_json = json!({ "files": files });
            Self::append_read_state_block(&mut output, &read_state_warnings);
            // 附加 JSON 变更详情
            output.push_str("\n\n<changes>\n");
            output.push_str(&changes_json.to_string());
            output.push_str("\n</changes>");

            Ok(ToolResult { success: true, output, error: None })
        }
    }
}
