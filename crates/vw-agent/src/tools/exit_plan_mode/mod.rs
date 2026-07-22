//! 退出计划模式的工具实现。
//!
//! 该工具在会话上下文中关闭计划模式。默认情况下，它会要求当前会话已有待办项，
//! 以降低“没有明确执行计划就开始写代码”的风险；`force` 用于显式绕过阻断。

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
    force: bool,
}

/// 关闭当前会话计划模式的工具。
///
/// 工具会读取会话待办状态作为最小执行准备度校验。校验失败时返回结构化阻断
/// 结果，而不是抛出错误，便于模型继续补齐计划。
#[derive(Clone, Default)]
pub struct ExitPlanModeTool;

impl ExitPlanModeTool {
    /// 创建退出计划模式工具。
    ///
    /// 返回无状态工具实例；输入解析、上下文缺失和待办校验在调用阶段处理。
    pub fn new() -> Self {
        Self
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl Tool for ExitPlanModeTool {
    fn name(&self) -> &str {
        crate::app::agent::tools::EXIT_PLAN_MODE_TOOL_ID
    }

    fn description(&self) -> &str {
        "退出计划模式。默认会先检查当前计划是否已满足进入执行阶段的最小条件。"
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "additionalProperties": false,
            "properties": {
                "force": {
                    "type": "boolean",
                    "description": "为 true 时忽略校验阻塞并直接退出计划模式。"
                }
            }
        })
    }

    fn spec(&self) -> ToolSpec {
        ToolSpec::new(self.name(), self.description(), self.parameters_schema())
            .with_display_name("ExitPlanMode")
            .with_aliases(
                crate::app::agent::tools::EXIT_PLAN_MODE_TOOL_ALIASES
                    .iter()
                    .map(|alias| alias.to_string())
                    .collect::<Vec<_>>(),
            )
            .with_read_only(true)
            .with_destructive(false)
            .with_concurrency_safe(false)
            .with_requires_user_interaction(false)
            .with_strict(true)
    }

    async fn call(&self, input: Value) -> anyhow::Result<ToolCallResult> {
        let args: Args = serde_json::from_value(input)
            .map_err(|error| anyhow::anyhow!("invalid exit plan mode arguments: {error}"))?;
        let context = current_tool_use_context()
            .ok_or_else(|| anyhow::anyhow!("missing active tool context"))?;
        let current = context.plan_mode_state();
        let todos = crate::app::agent::tools::todo::read_session_todos(context.session());

        let mut blockers = Vec::new();
        if !current.active {
            blockers.push("plan mode is not active".to_string());
        }
        if todos.is_empty() {
            blockers.push("at least one todo is required before execution".to_string());
        }

        // 阻断结果保留 active=true，明确告诉调用方仍处于只读计划阶段。
        if !args.force && !blockers.is_empty() {
            let data = json!({
                "active": true,
                "exited": false,
                "blockers": blockers,
                "force": false
            });
            return Ok(ToolCallResult {
                data: data.clone(),
                model_result: Value::String("Plan mode exit blocked".to_string()),
                content_blocks: vec![ToolResultContentDto::Json { value: data }],
                render_hint: Some(ToolRenderHint {
                    title: Some("ExitPlanMode".to_string()),
                    kind: Some("plan_mode".to_string()),
                    summary: Some("Plan mode remains active".to_string()),
                    metadata: json!({ "exited": false }),
                }),
                telemetry: Some(ToolCallTelemetry {
                    success: false,
                    ..ToolCallTelemetry::default()
                }),
                ..ToolCallResult::default()
            });
        }

        let state = context.exit_plan_mode();
        let data = json!({
            "active": state.active,
            "exited": true,
            "force": args.force,
            "goal": state.goal,
            "note": state.note
        });

        Ok(ToolCallResult {
            data: data.clone(),
            model_result: Value::String("Exited plan mode".to_string()),
            content_blocks: vec![ToolResultContentDto::Json { value: data }],
            render_hint: Some(ToolRenderHint {
                title: Some("ExitPlanMode".to_string()),
                kind: Some("plan_mode".to_string()),
                summary: Some("Plan mode disabled".to_string()),
                metadata: json!({ "exited": true }),
            }),
            telemetry: Some(ToolCallTelemetry { success: true, ..ToolCallTelemetry::default() }),
            ..ToolCallResult::default()
        })
    }

    async fn execute(&self, _args: Value) -> anyhow::Result<ToolResult> {
        Ok(ToolResult { success: true, output: "Exited plan mode".to_string(), error: None })
    }
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
