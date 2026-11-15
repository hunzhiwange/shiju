//! 补丁应用工具。
//!
//! `apply_patch` 保留现有的补丁解析、统一收集、路径安全与动作预算能力，
//! 但对 Claude Tools V2 运行时返回结构化结果：
//! - 结构化变更列表
//! - 结构化 patch hunks
//! - render hint / telemetry
//! - 精确的 read_state 更新与失效

use tool_context::context::{
    current_read_state_for_path, current_tool_use_context, invalidate_current_read_state,
    note_current_read_state,
};
use tool_files::{external_directory, FileToolHost};
use tool_files::file_edit::types::{FileDescriptor, StructuredPatch};
use tool_files::file_edit::utils::{
    build_file_descriptor, build_patch_summary, display_path, snapshot_from_text,
};
use tool_types::{Tool, ToolCallResult, ToolCallTelemetry, ToolRenderHint, ToolResult};
use agent_files as file;
use agent_files::watcher;
use agent_security::SecurityPolicy;
use anyhow::anyhow;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use vw_api_tools::{StructuredPatchHunkDto, ToolResultContentDto};

/// `apply_patch` 工具。
pub struct ApplyPatchTool<H: FileToolHost> {
    host: std::marker::PhantomData<H>,
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

impl<H: FileToolHost> ApplyPatchTool<H> {
    pub fn new(security: Arc<SecurityPolicy>) -> Self {
        Self { security, host: std::marker::PhantomData }
    }

    pub fn schema() -> serde_json::Value {
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

    pub fn failure(msg: impl Into<String>) -> ToolResult {
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

        external_directory::assert_external_directory::<H>(
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
        if current_tool_use_context::<H>().is_none() {
            return None;
        }

        Some(match current_read_state_for_path::<H>(source_path) {
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
                let _ = note_current_read_state::<H>(
                    change.target_path(),
                    change.new_content.len(),
                    false,
                    None,
                    None,
                    Some(snapshot_from_text(&change.new_content)),
                );
            }
            ChangeKind::Move => {
                let _ = invalidate_current_read_state::<H>(&change.file_path);
                let _ = note_current_read_state::<H>(
                    change.target_path(),
                    change.new_content.len(),
                    false,
                    None,
                    None,
                    Some(snapshot_from_text(&change.new_content)),
                );
            }
            ChangeKind::Delete => {
                let _ = invalidate_current_read_state::<H>(&change.file_path);
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
impl<H: FileToolHost> Tool for ApplyPatchTool<H> {
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
        let args: Args = serde_json::from_value(plugin_install::host_files::patch_input(args))
            .map_err(|error| anyhow!("Missing or invalid 'patchText' parameter: {error}"))?;

        match self.execute_internal(args).await {
            Ok(response) => {
                Ok(ToolResult { success: true, output: response.model_text, error: None })
            }
            Err(error) => Ok(Self::failure(error.to_string())),
        }
    }

    async fn call(&self, input: Value) -> anyhow::Result<ToolCallResult> {
        let args: Args = serde_json::from_value(plugin_install::host_files::patch_input(input))
            .map_err(|error| anyhow!("Missing or invalid 'patchText' parameter: {error}"))?;

        match self.execute_internal(args).await {
            Ok(response) => Ok(response.into_tool_call_result()),
            Err(error) => Ok(ToolCallResult::from_legacy_result(Self::failure(error.to_string()))),
        }
    }
}




#[cfg(any())]
include!("obsolete.rs");
