//! 文件整写工具。
//!
//! `file_write` 负责两类操作：
//! - 创建新文件
//! - 整体覆盖已有文件
//!
//! 对于已有文件，调用前必须先在当前工具上下文中执行过 `file_read`；如果文件在最
//! 近一次读取之后发生变化，则当前写入会被拒绝。局部替换应优先使用 `edit`。

use super::context::note_current_read_state;
use super::external_directory;
use super::file_edit::types::{FileDescriptor, StructuredPatch};
use super::file_edit::utils::{
    build_file_descriptor, build_patch_summary, display_path, ensure_read_state_is_fresh,
    normalize_slashes, read_state_metadata_for_path, require_read_state_for_existing_file,
    snapshot_from_text,
};
use super::traits::{Tool, ToolCallResult, ToolCallTelemetry, ToolRenderHint, ToolResult};
use crate::app::agent::file;
use crate::app::agent::file::watcher;
use crate::app::agent::security::SecurityPolicy;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use vw_api_types::tools::{StructuredPatchHunkDto, ToolResultContentDto};

pub struct FileWriteTool {
    security: Arc<SecurityPolicy>,
}

#[derive(Debug, Clone, Deserialize)]
struct Args {
    #[serde(alias = "filePath", alias = "file_path")]
    path: String,
    content: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum WritePayload {
    Create {
        file: FileDescriptor,
        structured_patch: StructuredPatch,
        #[serde(skip_serializing_if = "Option::is_none")]
        read_state: Option<Value>,
    },
    Update {
        file: FileDescriptor,
        structured_patch: StructuredPatch,
        #[serde(skip_serializing_if = "Option::is_none")]
        read_state: Option<Value>,
    },
}

#[derive(Debug, Clone)]
struct WriteResponse {
    model_text: String,
    payload: WritePayload,
    render_hint: ToolRenderHint,
    patch_hunks: Vec<StructuredPatchHunkDto>,
}

impl WriteResponse {
    fn into_tool_call_result(self) -> ToolCallResult {
        ToolCallResult {
            data: serde_json::to_value(&self.payload).unwrap_or(Value::Null),
            model_result: Value::String(self.model_text),
            content_blocks: vec![ToolResultContentDto::StructuredPatch { hunks: self.patch_hunks }],
            render_hint: Some(self.render_hint),
            telemetry: Some(ToolCallTelemetry { success: true, ..ToolCallTelemetry::default() }),
            ..ToolCallResult::default()
        }
    }
}

impl FileWriteTool {
    fn failure(msg: impl Into<String>) -> ToolResult {
        let msg = msg.into();
        ToolResult { success: false, output: msg.clone(), error: Some(msg) }
    }

    pub fn new(security: Arc<SecurityPolicy>) -> Self {
        Self { security }
    }

    fn resolve_path(&self, path: &str) -> PathBuf {
        let candidate = PathBuf::from(path);
        if candidate.is_absolute() { candidate } else { self.security.workspace_dir.join(path) }
    }

    async fn ensure_existing_file_allowed(&self, path: &Path) -> anyhow::Result<PathBuf> {
        let meta = tokio::fs::symlink_metadata(path).await.map_err(|error| {
            anyhow::anyhow!("Failed to read file metadata {}: {error}", path.display())
        })?;
        if meta.file_type().is_symlink() {
            anyhow::bail!("Refusing to write through symlink: {}", path.display());
        }
        if !meta.is_file() {
            anyhow::bail!("Path is a directory, not a file: {}", path.display());
        }
        let resolved = tokio::fs::canonicalize(path).await.map_err(|error| {
            anyhow::anyhow!("Failed to resolve file path {}: {error}", path.display())
        })?;
        if !self.security.is_resolved_path_allowed(&resolved) {
            anyhow::bail!(self.security.resolved_path_violation_message(&resolved));
        }
        Ok(resolved)
    }

    async fn ensure_target_parent_allowed(&self, path: &Path) -> anyhow::Result<PathBuf> {
        let parent =
            path.parent().ok_or_else(|| anyhow::anyhow!("Invalid path: missing parent"))?;
        tokio::fs::create_dir_all(parent).await.map_err(|error| {
            anyhow::anyhow!("Failed to create parent directory {}: {error}", parent.display())
        })?;
        let resolved_parent = tokio::fs::canonicalize(parent)
            .await
            .map_err(|error| anyhow::anyhow!("Failed to resolve file path: {error}"))?;
        if !self.security.is_resolved_path_allowed(&resolved_parent) {
            anyhow::bail!(self.security.resolved_path_violation_message(&resolved_parent));
        }
        Ok(resolved_parent)
    }

    async fn ensure_target_not_symlink(&self, path: &Path) -> anyhow::Result<()> {
        if let Ok(meta) = tokio::fs::symlink_metadata(path).await
            && meta.file_type().is_symlink()
        {
            anyhow::bail!("Refusing to write through symlink: {}", path.display());
        }
        Ok(())
    }

    async fn execute_internal(&self, args: Args) -> anyhow::Result<WriteResponse> {
        if args.path.trim().is_empty() {
            anyhow::bail!("Missing filePath")
        }
        if !self.security.can_act() {
            anyhow::bail!("Action blocked: autonomy is read-only")
        }
        if self.security.is_rate_limited() {
            anyhow::bail!("Rate limit exceeded: too many actions in the last hour")
        }
        if !self.security.is_path_allowed(&args.path) {
            anyhow::bail!("Path not allowed by security policy: {}", args.path)
        }

        let requested_path = self.resolve_path(&args.path);
        let requested_path_string = normalize_slashes(&requested_path);
        external_directory::assert_external_directory(
            &self.security,
            Some(&requested_path_string),
            Some(external_directory::Options {
                bypass: false,
                kind: external_directory::Kind::File,
            }),
        )
        .await
        .map_err(anyhow::Error::msg)?;

        if requested_path.exists() && requested_path.is_dir() {
            anyhow::bail!("Path is a directory, not a file: {}", requested_path.display())
        }

        let existed = requested_path.exists();
        let (old_content, resolved_existing, read_state_metadata) = if existed {
            let resolved = self.ensure_existing_file_allowed(&requested_path).await?;
            let old_content = tokio::fs::read_to_string(&resolved)
                .await
                .map_err(|error| anyhow::anyhow!("Failed to read file: {error}"))?;
            let display = display_path(&self.security.workspace_dir, &resolved);
            let read_state = require_read_state_for_existing_file(&resolved, &display, "write")?;
            ensure_read_state_is_fresh(&read_state, &old_content, &display, "write")?;
            let metadata =
                read_state_metadata_for_path(&self.security.workspace_dir, &resolved, "write");
            (old_content, Some(resolved), metadata)
        } else {
            (
                String::new(),
                None,
                read_state_metadata_for_path(
                    &self.security.workspace_dir,
                    &requested_path,
                    "write",
                ),
            )
        };

        let resolved_parent = self.ensure_target_parent_allowed(&requested_path).await?;
        let file_name = requested_path
            .file_name()
            .ok_or_else(|| anyhow::anyhow!("Invalid path: missing file name"))?;
        let resolved_target = resolved_existing.unwrap_or_else(|| resolved_parent.join(file_name));
        if !existed {
            self.ensure_target_not_symlink(&resolved_target).await?;
        }

        let changed = !existed || old_content != args.content;
        if changed {
            if !self.security.record_action() {
                anyhow::bail!("Rate limit exceeded: action budget exhausted")
            }
            tokio::fs::write(&resolved_target, &args.content)
                .await
                .map_err(|error| anyhow::anyhow!("Failed to write file: {error}"))?;

            let rel = display_path(&self.security.workspace_dir, &resolved_target);
            file::publish_edited(rel.clone());
            watcher::publish_updated(rel, if existed { "change" } else { "add" });
        }

        let _ = note_current_read_state(
            &resolved_target,
            args.content.len(),
            false,
            None,
            None,
            Some(snapshot_from_text(&args.content)),
        );

        let file = build_file_descriptor(
            &self.security.workspace_dir,
            &resolved_target,
            args.content.len() as u64,
        );
        let patch_summary = build_patch_summary(&file.path, &old_content, &args.content);
        let structured_patch = StructuredPatch { hunks: patch_summary.hunks.clone() };
        let payload = if existed {
            WritePayload::Update {
                file: file.clone(),
                structured_patch,
                read_state: read_state_metadata.clone(),
            }
        } else {
            WritePayload::Create {
                file: file.clone(),
                structured_patch,
                read_state: read_state_metadata.clone(),
            }
        };

        let operation = if existed { "update" } else { "create" };
        let summary = if existed {
            format!("Updated {}", file.path)
        } else {
            format!("Created {}", file.path)
        };
        let model_text = if existed {
            format!("Overwrote {} after verifying the latest file_read snapshot.", file.path)
        } else {
            format!("Created {}.", file.path)
        };

        Ok(WriteResponse {
            model_text,
            payload,
            render_hint: ToolRenderHint {
                title: Some(summary.clone()),
                kind: Some("file_write".to_string()),
                summary: Some(summary),
                metadata: json!({
                    "path": file.path,
                    "operation": operation,
                    "additions": patch_summary.additions,
                    "deletions": patch_summary.deletions,
                    "changed": changed,
                }),
            },
            patch_hunks: patch_summary.hunks,
        })
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl Tool for FileWriteTool {
    fn name(&self) -> &str {
        "file_write"
    }

    fn description(&self) -> &str {
        include_str!("write.txt")
    }

    fn parameters_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "filePath": {
                    "type": "string",
                    "description": "Compatibility alias for path."
                },
                "file_path": {
                    "type": "string",
                    "description": "Compatibility alias for path."
                },
                "path": {
                    "type": "string",
                    "description": "File path. Relative paths are resolved from the workspace; external paths require a policy allowlist entry."
                },
                "content": {
                    "type": "string",
                    "description": "Complete content to write. Existing files are overwritten in full."
                }
            },
            "required": ["path", "content"]
        })
    }

    async fn execute(&self, args: serde_json::Value) -> anyhow::Result<ToolResult> {
        let args: Args = serde_json::from_value(args)
            .map_err(|error| anyhow::anyhow!("Missing or invalid parameters: {error}"))?;

        match self.execute_internal(args).await {
            Ok(response) => {
                Ok(ToolResult { success: true, output: response.model_text, error: None })
            }
            Err(error) => Ok(Self::failure(error.to_string())),
        }
    }

    async fn call(&self, input: Value) -> anyhow::Result<ToolCallResult> {
        let args: Args = serde_json::from_value(input)
            .map_err(|error| anyhow::anyhow!("Missing or invalid parameters: {error}"))?;

        match self.execute_internal(args).await {
            Ok(response) => Ok(response.into_tool_call_result()),
            Err(error) => Ok(ToolCallResult::from_legacy_result(Self::failure(error.to_string()))),
        }
    }
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
