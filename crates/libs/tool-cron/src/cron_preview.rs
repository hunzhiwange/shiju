use super::{
    failure,
    schedule::{apply_schedule, schedule_properties},
};
use agent_config::runtime::Config;
use agent_cron::{CronJobPatch, schedule::input::validate_candidates};
use async_trait::async_trait;
use chrono::Utc;
use serde_json::{Value, json};
use std::sync::Arc;
use tool_types::{Tool, ToolResult};

pub struct CronPreviewTool {
    config: Arc<Config>,
}

impl CronPreviewTool {
    pub fn new(config: Arc<Config>) -> Self {
        Self { config }
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl Tool for CronPreviewTool {
    fn name(&self) -> &str {
        "cron_preview"
    }

    fn description(&self) -> &str {
        "Preview up to five upcoming run times for a proposed scheduled task, including its execution window"
    }

    fn parameters_schema(&self) -> Value {
        json!({"type": "object", "properties": schedule_properties(), "required": ["schedule"]})
    }

    async fn execute(&self, args: Value) -> anyhow::Result<ToolResult> {
        if !self.config.cron.enabled {
            return Ok(failure("cron is disabled by config (cron.enabled=false)"));
        }
        let mut patch = CronJobPatch::default();
        if let Err(error) = apply_schedule(&args, &mut patch) {
            return Ok(failure(error));
        }
        let Some(schedule) = patch.schedule else {
            return Ok(failure("Missing 'schedule' parameter"));
        };
        let starts_at = patch.starts_at.flatten();
        let ends_at = patch.ends_at.flatten();
        match validate_candidates(&schedule, starts_at, ends_at, Utc::now()) {
            Ok(candidates) => Ok(ToolResult {
                success: true,
                output: serde_json::to_string_pretty(&json!({
                    "schedule": schedule,
                    "starts_at": starts_at,
                    "ends_at": ends_at,
                    "candidates": candidates,
                }))?,
                error: None,
            }),
            Err(error) => Ok(failure(error)),
        }
    }
}
