//! 远端触发工具占位实现。
//!
//! 当前运行时尚未接入稳定的远端触发后端，因此本模块保持显式失败语义：工具可被发现，
//! 但执行时返回结构化“不支持”结果，避免调用方误以为触发已经发送。

use super::traits::{Tool, ToolResult, ToolSpec};
use async_trait::async_trait;
use serde::Deserialize;
use serde_json::{Value, json};

#[derive(Debug, Clone, Deserialize)]
struct Args {
    target: String,
    #[serde(default)]
    payload: Option<Value>,
}

#[derive(Clone, Default)]
/// 远端任务或环境动作触发工具。
///
/// 在未配置后端的运行时中，该工具不会静默降级或伪造成功结果。
pub struct RemoteTriggerTool;

impl RemoteTriggerTool {
    /// 创建远端触发工具实例。
    ///
    /// # 返回值
    ///
    /// 返回无状态工具实例。
    pub fn new() -> Self {
        Self
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl Tool for RemoteTriggerTool {
    fn name(&self) -> &str {
        "RemoteTrigger"
    }

    fn description(&self) -> &str {
        "Trigger a remote task or environment action. Returns an explicit error when the runtime has no stable remote-trigger backend configured."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "additionalProperties": false,
            "properties": {
                "target": {
                    "type": "string",
                    "description": "Remote target identifier, such as an environment name, node name, or task routing key."
                },
                "payload": {
                    "type": "object",
                    "description": "Optional additional arguments."
                }
            },
            "required": ["target"]
        })
    }

    fn spec(&self) -> ToolSpec {
        ToolSpec::new(self.name(), self.description(), self.parameters_schema())
            .with_display_name("RemoteTrigger")
            .with_aliases(vec!["remote_trigger".to_string()])
            .with_read_only(false)
            .with_destructive(false)
            .with_concurrency_safe(false)
            .with_requires_user_interaction(false)
            .with_strict(true)
    }

    async fn execute(&self, args: Value) -> anyhow::Result<ToolResult> {
        let args: Args = serde_json::from_value(args)?;
        // 远端触发属于跨环境动作；没有稳定后端时必须显式失败，避免扩大实际能力边界。
        let detail = json!({
            "target": args.target,
            "payload": args.payload,
            "reason": "remote trigger backend is not configured in this runtime"
        });
        Ok(ToolResult {
            success: false,
            output: serde_json::to_string_pretty(&detail)?,
            error: Some("RemoteTrigger is not supported in this runtime".to_string()),
        })
    }
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
