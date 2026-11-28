//! Cron 任务运行历史工具
//!
//! 查看定时任务的运行历史记录，包括开始时间、结束时间、状态和输出。

use super::failure;
use agent_config::runtime::Config;
use agent_cron as cron;
use async_trait::async_trait;
use serde::Deserialize;
use serde_json::json;
use std::sync::Arc;
use tool_types::{Tool, ToolResult};

const MAX_RUN_OUTPUT_CHARS: usize = 500;

pub struct CronRunsTool {
    config: Arc<Config>,
}

impl CronRunsTool {
    pub fn new(config: Arc<Config>) -> Self {
        Self { config }
    }
}

#[derive(Deserialize)]
struct RunQuery {
    job_id: Option<String>,
    limit: Option<usize>,
    scope: Option<Scope>,
    run_id: Option<i64>,
}

#[derive(Deserialize)]
#[serde(rename_all = "lowercase")]
enum Scope {
    Workspace,
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl Tool for CronRunsTool {
    fn name(&self) -> &str {
        "cron_runs"
    }

    fn description(&self) -> &str {
        "List recent runs for one scheduled task, inspect a retained run by ID, or view workspace history including running tasks"
    }

    fn parameters_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "job_id": {"type": "string", "description": "Task ID; optionally filters a workspace snapshot or run detail"},
                "scope": {"type": "string", "enum": ["workspace"], "description": "Include current runs and recent finished runs in the workspace"},
                "run_id": {"type": "integer", "minimum": 1, "description": "Persisted numeric run ID from history; returns the retained output for that run"},
                "limit": {"type": "integer", "minimum": 1, "description": "Maximum list entries, default 10"}
            }
        })
    }

    async fn execute(&self, args: serde_json::Value) -> anyhow::Result<ToolResult> {
        if !self.config.cron.enabled {
            return Ok(ToolResult {
                success: false,
                output: String::new(),
                error: Some("cron is disabled by config (cron.enabled=false)".to_string()),
            });
        }

        let query: RunQuery = match serde_json::from_value(args) {
            Ok(query) => query,
            Err(error) => return Ok(failure(format!("Invalid run query: {error}"))),
        };
        let job_id = query.job_id.as_deref().filter(|id| !id.trim().is_empty());
        let limit = query.limit.unwrap_or(10).max(1);
        let output = if let Some(run_id) = query.run_id {
            match cron::get_run(&self.config, run_id, job_id) {
                Ok(Some(run)) => serde_json::to_string_pretty(&run)?,
                Ok(None) => return Ok(failure("Run not found in retained history")),
                Err(_) => return Ok(failure("Failed to query cron history")),
            }
        } else if query.scope.is_some() {
            let mut history = match cron::history::snapshot(&self.config) {
                Ok(history) => history,
                Err(_) => return Ok(failure("Failed to query cron history")),
            };
            if let Some(job_id) = job_id {
                history.runs.retain(|run| run.job_id == job_id);
            }
            history.truncated |= history.runs.len() > limit;
            history.runs.truncate(limit);
            for run in &mut history.runs {
                run.output = run.output.take().map(|out| truncate(&out, MAX_RUN_OUTPUT_CHARS));
            }
            serde_json::to_string_pretty(&history)?
        } else {
            let Some(job_id) = job_id else {
                return Ok(failure(
                    "Missing 'job_id' parameter; use scope='workspace' for all tasks",
                ));
            };
            let mut runs = match cron::list_runs(&self.config, job_id, limit) {
                Ok(runs) => runs,
                Err(_) => return Ok(failure("Failed to query cron history")),
            };
            for run in &mut runs {
                run.output = run.output.take().map(|out| truncate(&out, MAX_RUN_OUTPUT_CHARS));
            }
            serde_json::to_string_pretty(&runs)?
        };
        Ok(ToolResult { success: true, output, error: None })
    }
}

fn truncate(input: &str, max_chars: usize) -> String {
    if input.chars().count() <= max_chars {
        return input.to_string();
    }
    let mut out: String = input.chars().take(max_chars).collect();
    out.push_str("...");
    out
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
