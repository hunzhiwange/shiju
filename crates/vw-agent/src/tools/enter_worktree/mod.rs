//! 进入隔离 Git worktree 的工具实现。
//!
//! 该工具创建真实 worktree，并把当前工具上下文绑定到新目录。后续文件、命令等
//! 工具可以通过上下文切换到隔离工作区，避免直接污染主工作树。

use super::context::current_tool_use_context;
use super::traits::{
    Tool, ToolCallResult, ToolCallTelemetry, ToolRenderHint, ToolResult, ToolSpec,
};
use async_trait::async_trait;
use serde::Deserialize;
use serde_json::{Value, json};
use vw_api_types::tools::ToolResultContentDto;

#[derive(Debug, Clone, Default, Deserialize)]
struct Args {
    #[serde(default)]
    name: Option<String>,
    #[serde(rename = "startCommand", default)]
    start_command: Option<String>,
}

/// 创建并绑定会话 worktree 的工具。
///
/// 工具会调用 `crate::worktree` 子系统完成实际创建，并把目录、分支和名称记录到
/// `ToolUseContext`。创建失败或当前会话已有绑定时会返回错误。
#[derive(Clone, Default)]
pub struct EnterWorktreeTool;

impl EnterWorktreeTool {
    /// 创建进入 worktree 工具。
    ///
    /// 返回无状态工具实例；具体的 Git 和上下文错误在 `Tool::call` 中处理。
    pub fn new() -> Self {
        Self
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl Tool for EnterWorktreeTool {
    fn name(&self) -> &str {
        crate::app::agent::tools::ENTER_WORKTREE_TOOL_ID
    }

    fn description(&self) -> &str {
        "Create and bind a real Git worktree. Subsequent tool calls switch to that isolated workspace."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "additionalProperties": false,
            "properties": {
                "name": {
                    "type": "string",
                    "description": "Optional worktree name."
                },
                "startCommand": {
                    "type": "string",
                    "description": "Startup command to run in the worktree after creation."
                }
            }
        })
    }

    fn spec(&self) -> ToolSpec {
        ToolSpec::new(self.name(), self.description(), self.parameters_schema())
            .with_display_name("EnterWorktree")
            .with_aliases(
                crate::app::agent::tools::ENTER_WORKTREE_TOOL_ALIASES
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
            .map_err(|error| anyhow::anyhow!("invalid enter worktree arguments: {error}"))?;
        let context = current_tool_use_context()
            .ok_or_else(|| anyhow::anyhow!("missing active tool context"))?;
        if context.worktree_binding_state().directory.is_some() {
            return Err(anyhow::anyhow!("a worktree is already bound to this session"));
        }

        // 创建成功后再写入上下文绑定，避免失败路径留下半绑定状态。
        let info = crate::worktree::create(Some(crate::worktree::CreateInput {
            name: args.name.clone(),
            start_command: args.start_command.clone(),
        }))
        .await
        .map_err(|error| anyhow::anyhow!(error.to_string()))?;
        let binding_name = args.name.clone().unwrap_or_else(|| info.name.clone());
        let binding =
            context.bind_worktree(info.directory.clone(), binding_name, info.branch.clone());
        let data = json!({
            "name": info.name,
            "branch": info.branch,
            "directory": info.directory,
            "primary_root": context.root(),
            "binding": binding
        });

        Ok(ToolCallResult {
            data: data.clone(),
            model_result: Value::String("Entered worktree".to_string()),
            content_blocks: vec![ToolResultContentDto::Json { value: data.clone() }],
            render_hint: Some(ToolRenderHint {
                title: Some("EnterWorktree".to_string()),
                kind: Some("worktree".to_string()),
                summary: Some(format!(
                    "Bound worktree {}",
                    data["directory"].as_str().unwrap_or_default()
                )),
                metadata: json!({ "directory": data["directory"] }),
            }),
            telemetry: Some(ToolCallTelemetry { success: true, ..ToolCallTelemetry::default() }),
            ..ToolCallResult::default()
        })
    }

    async fn execute(&self, _args: Value) -> anyhow::Result<ToolResult> {
        Ok(ToolResult { success: true, output: "Entered worktree".to_string(), error: None })
    }
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
