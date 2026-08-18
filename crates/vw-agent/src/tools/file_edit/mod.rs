//! 文件局部编辑工具。
//!
//! `file_edit` 只负责对已存在文本文件执行基于字符串的精确替换：
//! - 调用前必须先在当前工具上下文中执行过 `file_read`
//! - 如果最近一次 `file_read` 之后文件内容发生变化，则拒绝继续编辑
//! - 默认要求 `old_string` 唯一；仅当 `replace_all=true` 时才允许多处替换
//! - 返回结构化 patch，并在 quote 风格不一致时尽量保留现有文件的写法

pub(crate) mod types;
pub(crate) mod utils;

use self::types::{Args, EditPayload, StructuredPatch};
use self::utils::{
    build_file_descriptor, build_patch_summary, build_replacement_plan, display_path,
    ensure_read_state_is_fresh, normalize_slashes, read_state_metadata_for_path,
    require_read_state_for_existing_file, snapshot_from_text,
};
use super::context::note_current_read_state;
use super::external_directory;
use super::traits::{
    Tool, ToolCallResult, ToolCallTelemetry, ToolRenderHint, ToolResult, ToolSpec,
};
use crate::app::agent::file;
use crate::app::agent::file::watcher;
use crate::app::agent::security::SecurityPolicy;
use async_trait::async_trait;
use serde_json::{Value, json};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use vw_api_types::tools::ToolResultContentDto;

/// `file_edit` 工具。
pub struct FileEditTool {
    security: Arc<SecurityPolicy>,
}

#[derive(Debug, Clone)]
struct EditResponse {
    model_text: String,
    payload: EditPayload,
    render_hint: ToolRenderHint,
    patch_hunks: Vec<vw_api_types::tools::StructuredPatchHunkDto>,
}

impl EditResponse {
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

impl FileEditTool {
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
            anyhow::bail!("Refusing to edit through symlink: {}", path.display());
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

    async fn execute_internal(&self, args: Args) -> anyhow::Result<EditResponse> {
        if args.file_path.trim().is_empty() {
            anyhow::bail!("Missing file_path")
        }
        if !self.security.can_act() {
            anyhow::bail!("Action blocked: autonomy is read-only")
        }
        if self.security.is_rate_limited() {
            anyhow::bail!("Rate limit exceeded: too many actions in the last hour")
        }
        if !self.security.is_path_allowed(&args.file_path) {
            anyhow::bail!("Path not allowed by security policy: {}", args.file_path)
        }

        let requested_path = self.resolve_path(&args.file_path);
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

        let resolved = self.ensure_existing_file_allowed(&requested_path).await?;
        let current_text = tokio::fs::read_to_string(&resolved)
            .await
            .map_err(|error| anyhow::anyhow!("Failed to read file: {error}"))?;
        let display = display_path(&self.security.workspace_dir, &resolved);
        let read_state = require_read_state_for_existing_file(&resolved, &display, "file_edit")?;
        ensure_read_state_is_fresh(&read_state, &current_text, &display, "file_edit")?;

        let replacement = build_replacement_plan(
            &current_text,
            &args.old_string,
            &args.new_string,
            args.replace_all,
        )?;
        let patch_summary =
            build_patch_summary(&display, &current_text, &replacement.updated_content);
        let read_state_metadata =
            read_state_metadata_for_path(&self.security.workspace_dir, &resolved, "file_edit");

        let changed = replacement.updated_content != current_text;
        if changed {
            if !self.security.record_action() {
                anyhow::bail!("Rate limit exceeded: action budget exhausted")
            }
            tokio::fs::write(&resolved, &replacement.updated_content)
                .await
                .map_err(|error| anyhow::anyhow!("Failed to write file: {error}"))?;

            file::publish_edited(display.clone());
            watcher::publish_updated(display.clone(), "change");
        }

        let _ = note_current_read_state(
            &resolved,
            replacement.updated_content.len(),
            false,
            None,
            None,
            Some(snapshot_from_text(&replacement.updated_content)),
        );

        let file = build_file_descriptor(
            &self.security.workspace_dir,
            &resolved,
            replacement.updated_content.len() as u64,
        );
        let structured_patch = StructuredPatch { hunks: patch_summary.hunks.clone() };
        let payload = EditPayload::Update {
            file: file.clone(),
            replacements: replacement.replacements,
            replace_all: args.replace_all,
            quote_normalized_match: replacement.quote_normalized_match,
            structured_patch,
            read_state: read_state_metadata.clone(),
        };

        let summary = if changed {
            format!("Updated {}", file.path)
        } else {
            format!("No-op edit for {}", file.path)
        };
        let model_text = if replacement.quote_normalized_match {
            format!(
                "Updated {} by replacing {} occurrence(s). Quote normalization was used to match the existing text.",
                file.path, replacement.replacements
            )
        } else if changed {
            format!(
                "Updated {} by replacing {} occurrence(s).",
                file.path, replacement.replacements
            )
        } else {
            format!(
                "Matched {} occurrence(s) in {}, but the replacement did not change the file contents.",
                replacement.replacements, file.path
            )
        };

        Ok(EditResponse {
            model_text,
            payload,
            render_hint: ToolRenderHint {
                title: Some(format!("Edited {}", file.path)),
                kind: Some("file_edit".to_string()),
                summary: Some(summary),
                metadata: json!({
                    "path": file.path,
                    "operation": "update",
                    "replacements": replacement.replacements,
                    "replaceAll": args.replace_all,
                    "quoteNormalizedMatch": replacement.quote_normalized_match,
                    "additions": patch_summary.additions,
                    "deletions": patch_summary.deletions,
                }),
            },
            patch_hunks: patch_summary.hunks,
        })
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl Tool for FileEditTool {
    fn name(&self) -> &str {
        "file_edit"
    }

    fn description(&self) -> &str {
        "Edit existing files by replacing old_string with new_string. Existing files must be read first with file_read in the current tool context. By default old_string must be unique; use replace_all=true to replace multiple matches."
    }

    fn parameters_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "file_path": {
                    "type": "string",
                    "description": "Target file path. Relative paths are resolved from the workspace."
                },
                "filePath": {
                    "type": "string",
                    "description": "Compatibility alias for file_path."
                },
                "path": {
                    "type": "string",
                    "description": "Compatibility alias for file_path."
                },
                "old_string": {
                    "type": "string",
                    "description": "Original text to match. It must be unique by default."
                },
                "oldString": {
                    "type": "string",
                    "description": "Compatibility alias for old_string."
                },
                "new_string": {
                    "type": "string",
                    "description": "Replacement text."
                },
                "newString": {
                    "type": "string",
                    "description": "Compatibility alias for new_string."
                },
                "replace_all": {
                    "type": "boolean",
                    "description": "Whether to replace all matches. Defaults to false."
                },
                "replaceAll": {
                    "type": "boolean",
                    "description": "Compatibility alias for replace_all."
                }
            },
            "required": ["file_path", "old_string", "new_string"]
        })
    }

    fn spec(&self) -> ToolSpec {
        ToolSpec::new("file_edit", self.description(), self.parameters_schema())
            .with_display_name("file_edit")
            .with_read_only(false)
            .with_destructive(false)
            .with_concurrency_safe(false)
            .with_requires_user_interaction(false)
            .with_strict(true)
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
