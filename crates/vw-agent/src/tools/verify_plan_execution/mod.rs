//! VerifyPlanExecution 工具实现。
//!
//! 该工具检查当前计划模式是否满足进入执行阶段的最小条件：计划模式已开启、存在 todo、
//! 且最多只有一个 in_progress。它只读取会话状态，不修改计划或任务。

use super::context::current_tool_use_context;
use super::traits::{
    Tool, ToolCallResult, ToolCallTelemetry, ToolRenderHint, ToolResult, ToolSpec,
};
use async_trait::async_trait;
use serde_json::{Value, json};
use vw_api_types::tools::ToolResultContentDto;

#[derive(Clone, Default)]
pub struct VerifyPlanExecutionTool;

impl VerifyPlanExecutionTool {
    /// 创建 VerifyPlanExecution 工具实例。
    ///
    /// 返回值：无状态工具实例。
    /// 错误处理：该函数不返回错误。
    pub fn new() -> Self {
        Self
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl Tool for VerifyPlanExecutionTool {
    fn name(&self) -> &str {
        crate::app::agent::tools::VERIFY_PLAN_EXECUTION_TOOL_ID
    }

    fn description(&self) -> &str {
        "校验当前计划模式是否已经满足进入执行阶段的最小条件。"
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "additionalProperties": false,
            "properties": {}
        })
    }

    fn spec(&self) -> ToolSpec {
        ToolSpec::new(self.name(), self.description(), self.parameters_schema())
            .with_display_name("VerifyPlanExecution")
            .with_aliases(
                crate::app::agent::tools::VERIFY_PLAN_EXECUTION_TOOL_ALIASES
                    .iter()
                    .map(|alias| alias.to_string())
                    .collect::<Vec<_>>(),
            )
            .with_read_only(true)
            .with_destructive(false)
            .with_concurrency_safe(true)
            .with_requires_user_interaction(false)
            .with_strict(true)
    }

    async fn call(&self, _input: Value) -> anyhow::Result<ToolCallResult> {
        let context = current_tool_use_context()
            .ok_or_else(|| anyhow::anyhow!("missing active tool context"))?;
        let state = context.plan_mode_state();
        let todos = crate::app::agent::tools::todo::read_session_todos(context.session());
        let pending_count = todos
            .iter()
            .filter(|todo| matches!(todo.status.as_str(), "pending" | "in_progress"))
            .count();
        let in_progress_count = todos.iter().filter(|todo| todo.status == "in_progress").count();

        let mut blockers = Vec::new();
        if !state.active {
            blockers.push("plan mode is not active".to_string());
        }
        if todos.is_empty() {
            // 没有 todo 时直接进入执行阶段会让计划不可验证，因此要求至少一个明确任务。
            blockers.push("at least one todo is required before execution".to_string());
        }
        if in_progress_count > 1 {
            // 多个 in_progress 会让执行焦点不明确，和计划模式“一次推进一项”的约束冲突。
            blockers.push("only one todo may be in_progress".to_string());
        }

        let ready = blockers.is_empty();
        let data = json!({
            "ready": ready,
            "blockers": blockers,
            "todo_count": todos.len(),
            "pending_count": pending_count,
            "in_progress_count": in_progress_count,
            "goal": state.goal,
            "note": state.note
        });

        Ok(ToolCallResult {
            data: data.clone(),
            model_result: Value::String(if ready {
                "Plan execution verified".to_string()
            } else {
                "Plan execution is blocked".to_string()
            }),
            content_blocks: vec![ToolResultContentDto::Json { value: data.clone() }],
            render_hint: Some(ToolRenderHint {
                title: Some("VerifyPlanExecution".to_string()),
                kind: Some("plan_mode".to_string()),
                summary: Some(if ready {
                    format!("Ready to execute {} todo(s)", pending_count)
                } else {
                    format!(
                        "Blocked by {} issue(s)",
                        data["blockers"].as_array().map(Vec::len).unwrap_or(0)
                    )
                }),
                metadata: json!({ "ready": ready }),
            }),
            telemetry: Some(ToolCallTelemetry { success: ready, ..ToolCallTelemetry::default() }),
            ..ToolCallResult::default()
        })
    }

    async fn execute(&self, _args: Value) -> anyhow::Result<ToolResult> {
        Ok(ToolResult { success: true, output: "Verified plan execution".to_string(), error: None })
    }
}

#[cfg(test)]
#[path = "mod_tests.rs"]
mod mod_tests;
#[cfg(test)]
#[path = "tests.rs"]
mod tests;
