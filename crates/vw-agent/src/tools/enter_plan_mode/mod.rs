//! 进入计划模式的工具实现。
//!
//! 计划模式把当前会话切换到只读规划阶段，并向模型返回后续应遵循的探索步骤。
//! 它通过 `ToolUseContext` 保存状态，供工具执行器在计划模式下阻断写入型工具。

use super::context::current_tool_use_context;
use super::traits::{
    Tool, ToolCallResult, ToolCallTelemetry, ToolRenderHint, ToolResult, ToolSpec,
};
use async_trait::async_trait;
use serde::Deserialize;
use serde_json::{Value, json};
use vw_api_types::tools::ToolResultContentDto;

mod prompt;

use self::prompt::{
    ENTER_PLAN_MODE_DESCRIPTION, enter_plan_mode_instruction_lines, enter_plan_mode_message,
    enter_plan_mode_result_text,
};

#[derive(Debug, Clone, Default, Deserialize)]
struct Args {
    #[serde(default)]
    goal: Option<String>,
    #[serde(default)]
    note: Option<String>,
}

/// 将当前会话标记为计划模式的工具。
///
/// 工具本身不写文件、不启动外部进程，只更新会话上下文中的计划状态并返回
/// 面向模型的执行约束说明。
#[derive(Clone, Default)]
pub struct EnterPlanModeTool;

impl EnterPlanModeTool {
    /// 创建进入计划模式工具。
    ///
    /// 返回值为无状态工具实例；错误处理由后续 `Tool::call` 在解析输入和读取
    /// 当前工具上下文时完成。
    pub fn new() -> Self {
        Self
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl Tool for EnterPlanModeTool {
    fn name(&self) -> &str {
        crate::app::agent::tools::ENTER_PLAN_MODE_TOOL_ID
    }

    fn description(&self) -> &str {
        ENTER_PLAN_MODE_DESCRIPTION
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "additionalProperties": false,
            "properties": {
                "goal": {
                    "type": "string",
                    "description": "本轮计划的目标摘要。"
                },
                "note": {
                    "type": "string",
                    "description": "补充说明，可选。"
                }
            }
        })
    }

    fn spec(&self) -> ToolSpec {
        ToolSpec::new(self.name(), self.description(), self.parameters_schema())
            .with_display_name("EnterPlanMode")
            .with_aliases(
                crate::app::agent::tools::ENTER_PLAN_MODE_TOOL_ALIASES
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
        if !input.is_object() {
            anyhow::bail!("invalid enter plan mode arguments: expected object");
        }
        let args: Args = serde_json::from_value(input)
            .map_err(|error| anyhow::anyhow!("invalid enter plan mode arguments: {error}"))?;
        let context = current_tool_use_context()
            .ok_or_else(|| anyhow::anyhow!("missing active tool context"))?;
        let already_active = context.plan_mode_state().active;
        // 即使已经处于计划模式，也刷新目标和备注，让调用方可以修正当前计划范围。
        let state = context.enter_plan_mode(args.goal.clone(), args.note.clone());
        let message = enter_plan_mode_message(already_active);
        let instructions = enter_plan_mode_instruction_lines();
        let result_text = enter_plan_mode_result_text(already_active);
        let data = json!({
            "active": state.active,
            "already_active": already_active,
            "goal": state.goal,
            "note": state.note,
            "entered_at_ms": state.entered_at_ms,
            "allowed_mode": "read_only_plus_planning",
            "message": message,
            "instructions": instructions
        });

        Ok(ToolCallResult {
            data: data.clone(),
            model_result: Value::String(result_text.clone()),
            content_blocks: vec![
                ToolResultContentDto::Json { value: data },
                ToolResultContentDto::Text { text: result_text },
            ],
            render_hint: Some(ToolRenderHint {
                title: Some("EnterPlanMode".to_string()),
                kind: Some("plan_mode".to_string()),
                summary: Some(if already_active {
                    "Plan mode remains active".to_string()
                } else {
                    "Plan mode enabled".to_string()
                }),
                metadata: json!({ "active": true }),
            }),
            telemetry: Some(ToolCallTelemetry { success: true, ..ToolCallTelemetry::default() }),
            ..ToolCallResult::default()
        })
    }

    async fn execute(&self, _args: Value) -> anyhow::Result<ToolResult> {
        Ok(ToolResult { success: true, output: "Entered plan mode".to_string(), error: None })
    }
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
