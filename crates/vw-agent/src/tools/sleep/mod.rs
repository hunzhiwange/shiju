//! Sleep 工具实现。
//!
//! 该工具提供短时间等待能力，用于外部状态收敛或退避。等待上限固定为 60 秒，避免
//! 自动代理通过 sleep 长时间占用执行槽。

use super::traits::{Tool, ToolResult, ToolSpec};
use async_trait::async_trait;
use serde::Deserialize;
use serde_json::{Value, json};
use std::time::Duration;

const MAX_SLEEP_MS: u64 = 60_000;

/// Sleep 工具入参。
#[derive(Debug, Clone, Deserialize)]
struct Args {
    /// 毫秒单位等待时长，优先于 `seconds`。
    #[serde(default)]
    duration_ms: Option<u64>,
    /// 秒单位等待时长，会转换为毫秒。
    #[serde(default)]
    seconds: Option<f64>,
}

/// 短等待工具。
#[derive(Clone, Default)]
pub struct SleepTool;

impl SleepTool {
    /// 创建 Sleep 工具实例。
    ///
    /// 返回值：无状态工具实例。
    /// 错误处理：该函数不返回错误。
    pub fn new() -> Self {
        Self
    }

    fn resolve_duration(args: &Args) -> anyhow::Result<u64> {
        let duration_ms = match (args.duration_ms, args.seconds) {
            (Some(duration_ms), _) => duration_ms,
            (None, Some(seconds)) if seconds.is_finite() && seconds >= 0.0 => {
                (seconds * 1000.0).round() as u64
            }
            (None, Some(_)) => anyhow::bail!("seconds must be a finite positive number"),
            (None, None) => anyhow::bail!("either 'duration_ms' or 'seconds' is required"),
        };

        if duration_ms > MAX_SLEEP_MS {
            // sleep 是调度辅助而不是长期任务挂起机制，固定上限能保护运行时吞吐。
            anyhow::bail!("sleep duration exceeds {MAX_SLEEP_MS} ms");
        }

        Ok(duration_ms)
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl Tool for SleepTool {
    fn name(&self) -> &str {
        "Sleep"
    }

    fn description(&self) -> &str {
        "Pause execution briefly while waiting for external state to settle or applying a short backoff."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "additionalProperties": false,
            "properties": {
                "duration_ms": {
                    "type": "integer",
                    "minimum": 0,
                    "maximum": MAX_SLEEP_MS,
                    "description": "Pause duration in milliseconds."
                },
                "seconds": {
                    "type": "number",
                    "minimum": 0,
                    "maximum": 60,
                    "description": "Pause duration in seconds."
                }
            }
        })
    }

    fn spec(&self) -> ToolSpec {
        ToolSpec::new(self.name(), self.description(), self.parameters_schema())
            .with_display_name("Sleep")
            .with_aliases(vec!["sleep".to_string()])
            .with_read_only(true)
            .with_destructive(false)
            .with_concurrency_safe(false)
            .with_requires_user_interaction(false)
            .with_strict(true)
    }

    async fn execute(&self, args: Value) -> anyhow::Result<ToolResult> {
        let args: Args = serde_json::from_value(args)?;
        let duration_ms = Self::resolve_duration(&args)?;
        tokio::time::sleep(Duration::from_millis(duration_ms)).await;

        Ok(ToolResult { success: true, output: format!("Slept for {duration_ms} ms"), error: None })
    }
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;

#[cfg(test)]
#[path = "mod_tests.rs"]
mod mod_tests;
