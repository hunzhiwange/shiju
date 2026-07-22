//! 退出当前绑定 worktree 的工具实现。
//!
//! 默认路径会先检查 Git 工作树是否干净，再移除 worktree 并清理会话绑定。
//! `force` 模式用于用户明确接受丢弃隔离工作区未提交改动的场景。

use super::context::current_tool_use_context;
use super::traits::{
    Tool, ToolCallResult, ToolCallTelemetry, ToolRenderHint, ToolResult, ToolSpec,
};
use async_trait::async_trait;
use serde::Deserialize;
use serde_json::{Value, json};
use std::path::Path;
use vw_api_types::tools::ToolResultContentDto;

#[derive(Debug, Clone, Default, Deserialize)]
struct Args {
    #[serde(default)]
    force: bool,
}

/// 移除并解绑当前会话 worktree 的工具。
///
/// 工具会调用 Git 读取 porcelain 状态；非强制模式下发现未提交改动时返回结构化
/// 阻断结果，避免静默删除用户可能需要保留的变更。
#[derive(Clone, Default)]
pub struct ExitWorktreeTool;

impl ExitWorktreeTool {
    /// 创建退出 worktree 工具。
    ///
    /// 返回无状态工具实例；上下文绑定、Git 状态和移除错误在调用阶段处理。
    pub fn new() -> Self {
        Self
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl Tool for ExitWorktreeTool {
    fn name(&self) -> &str {
        crate::app::agent::tools::EXIT_WORKTREE_TOOL_ID
    }

    fn description(&self) -> &str {
        "退出当前绑定的 worktree。默认要求工作树干净，force 模式会强制移除。"
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "additionalProperties": false,
            "properties": {
                "force": {
                    "type": "boolean",
                    "description": "为 true 时强制移除 worktree。"
                }
            }
        })
    }

    fn spec(&self) -> ToolSpec {
        ToolSpec::new(self.name(), self.description(), self.parameters_schema())
            .with_display_name("ExitWorktree")
            .with_aliases(
                crate::app::agent::tools::EXIT_WORKTREE_TOOL_ALIASES
                    .iter()
                    .map(|alias| alias.to_string())
                    .collect::<Vec<_>>(),
            )
            .with_read_only(false)
            .with_destructive(true)
            .with_concurrency_safe(false)
            .with_requires_user_interaction(false)
            .with_strict(true)
    }

    async fn call(&self, input: Value) -> anyhow::Result<ToolCallResult> {
        let args: Args = serde_json::from_value(input)
            .map_err(|error| anyhow::anyhow!("invalid exit worktree arguments: {error}"))?;
        let context = current_tool_use_context()
            .ok_or_else(|| anyhow::anyhow!("missing active tool context"))?;
        let binding = context.worktree_binding_state();
        let Some(directory) = binding.directory.clone() else {
            return Err(anyhow::anyhow!("no active worktree binding"));
        };

        let status = git_status_porcelain(Path::new(&directory))?;
        if !args.force && !status.trim().is_empty() {
            let data = json!({
                "exited": false,
                "directory": directory,
                "dirty": true,
                "status": status
            });
            return Ok(ToolCallResult {
                data: data.clone(),
                model_result: Value::String("Worktree exit blocked".to_string()),
                content_blocks: vec![ToolResultContentDto::Json { value: data }],
                render_hint: Some(ToolRenderHint {
                    title: Some("ExitWorktree".to_string()),
                    kind: Some("worktree".to_string()),
                    summary: Some("Worktree has uncommitted changes".to_string()),
                    metadata: json!({ "exited": false }),
                }),
                telemetry: Some(ToolCallTelemetry {
                    success: false,
                    ..ToolCallTelemetry::default()
                }),
                ..ToolCallResult::default()
            });
        }

        // 只有底层 worktree 移除成功后才清理上下文绑定，避免会话指向丢失但目录仍在。
        crate::worktree::remove(crate::worktree::RemoveInput {
            directory: directory.clone(),
            force: args.force,
        })
        .await
        .map_err(|error| anyhow::anyhow!("Failed to remove git worktree: {error}"))?;
        context.clear_worktree_binding();

        let data = json!({
            "exited": true,
            "directory": directory,
            "force": args.force
        });
        Ok(ToolCallResult {
            data: data.clone(),
            model_result: Value::String("Exited worktree".to_string()),
            content_blocks: vec![ToolResultContentDto::Json { value: data.clone() }],
            render_hint: Some(ToolRenderHint {
                title: Some("ExitWorktree".to_string()),
                kind: Some("worktree".to_string()),
                summary: Some("Worktree removed and binding cleared".to_string()),
                metadata: json!({ "exited": true }),
            }),
            telemetry: Some(ToolCallTelemetry { success: true, ..ToolCallTelemetry::default() }),
            ..ToolCallResult::default()
        })
    }

    async fn execute(&self, _args: Value) -> anyhow::Result<ToolResult> {
        Ok(ToolResult { success: true, output: "Exited worktree".to_string(), error: None })
    }
}

fn git_status_porcelain(directory: &Path) -> anyhow::Result<String> {
    let output = std::process::Command::new("git")
        .args(["status", "--porcelain=v1"])
        .current_dir(directory)
        .output()?;
    if !output.status.success() {
        return Err(anyhow::anyhow!(
            "failed to read worktree status: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
