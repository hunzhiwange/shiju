//! Cron 任务列表工具
//!
//! 列出所有已计划的定时任务。

use super::traits::{Tool, ToolResult};
use crate::app::agent::config::Config;
use crate::app::agent::cron;
use async_trait::async_trait;
use serde_json::json;
use std::sync::Arc;

pub struct CronListTool {
    config: Arc<Config>,
}

impl CronListTool {
    pub fn new(config: Arc<Config>) -> Self {
        Self { config }
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl Tool for CronListTool {
    fn name(&self) -> &str {
        "cron_list"
    }

    fn description(&self) -> &str {
        "List all scheduled tasks"
    }

    fn parameters_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {},
            "additionalProperties": false
        })
    }

    async fn execute(&self, _args: serde_json::Value) -> anyhow::Result<ToolResult> {
        if !self.config.cron.enabled {
            return Ok(ToolResult {
                success: false,
                output: String::new(),
                error: Some("cron is disabled by config (cron.enabled=false)".to_string()),
            });
        }

        match cron::list_jobs(&self.config) {
            Ok(jobs) => Ok(ToolResult {
                success: true,
                output: serde_json::to_string_pretty(&jobs)?,
                error: None,
            }),
            Err(e) => {
                Ok(ToolResult { success: false, output: String::new(), error: Some(e.to_string()) })
            }
        }
    }
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
