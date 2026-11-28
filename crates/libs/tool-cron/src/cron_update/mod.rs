//! 更新定时任务的显式字段，检查当前会话的执行权限。

use super::{
    failure,
    options::{job_properties, parse_options},
};
use agent_config::runtime::Config;
use agent_cron::{
    self as cron, JobType, delivery::DeliveryHost, schedule::input::validate_candidates,
};
use agent_security::{AutonomyLevel, SecurityPolicy};
use async_trait::async_trait;
use chrono::Utc;
use serde_json::{Value, json};
use std::{path::Path, sync::Arc};
use tool_types::{Tool, ToolResult};

pub struct CronUpdateTool {
    config: Arc<Config>,
    security: Arc<SecurityPolicy>,
    delivery_host: Option<&'static dyn DeliveryHost>,
}

impl CronUpdateTool {
    pub fn new(config: Arc<Config>, security: Arc<SecurityPolicy>) -> Self {
        Self { config, security, delivery_host: None }
    }

    pub fn with_delivery_host(mut self, host: &'static dyn DeliveryHost) -> Self {
        self.delivery_host = Some(host);
        self
    }

    fn enforce_mutation_allowed(&self) -> Result<(), &'static str> {
        if !self.security.can_act() {
            return Err("Security policy: read-only mode, cannot perform 'cron_update'");
        }
        if self.security.is_rate_limited() {
            return Err("Rate limit exceeded: too many actions in the last hour");
        }
        if !self.security.record_action() {
            return Err("Rate limit exceeded: action budget exhausted");
        }
        Ok(())
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl Tool for CronUpdateTool {
    fn name(&self) -> &str {
        "cron_update"
    }

    fn description(&self) -> &str {
        "Edit or pause a scheduled task. Supply only changed fields in patch. An omitted window boundary is preserved; null clears it."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "job_id": {"type": "string", "description": "Scheduled task ID from cron_list"},
                "patch": {"type": "object", "properties": job_properties(true)},
                "approved": {
                    "type": "boolean",
                    "description": "Explicit approval for Shell commands that require it under the current policy",
                    "default": false
                }
            },
            "required": ["job_id", "patch"]
        })
    }

    async fn execute(&self, args: Value) -> anyhow::Result<ToolResult> {
        if !self.config.cron.enabled {
            return Ok(failure("cron is disabled by config (cron.enabled=false)"));
        }
        let Some(job_id) =
            args.get("job_id").and_then(Value::as_str).filter(|id| !id.trim().is_empty())
        else {
            return Ok(failure("Missing 'job_id' parameter"));
        };
        let Some(value) = args.get("patch") else {
            return Ok(failure("Missing 'patch' parameter"));
        };
        let (mut patch, delivery) = match parse_options(value) {
            Ok(options) => options,
            Err(error) => return Ok(failure(error)),
        };
        if let Some(path) = patch.project_path.as_deref()
            && !path.trim().is_empty()
            && (!Path::new(path).is_absolute() || !self.security.is_path_allowed(path))
        {
            return Ok(failure("Scheduled task project is outside the permitted workspace"));
        }
        let approved = args.get("approved").and_then(Value::as_bool).unwrap_or(false);
        if let Some(command) = &patch.command
            && let Err(error) = self.security.validate_command_execution(command, approved)
        {
            return Ok(failure(error));
        }
        if let Err(error) = self.enforce_mutation_allowed() {
            return Ok(failure(error));
        }
        let current = match cron::get_job(&self.config, job_id) {
            Ok(job) => job,
            Err(error) => return Ok(failure(error.to_string())),
        };
        let full_access_allowed =
            self.security.autonomy == AutonomyLevel::Full && !self.security.workspace_only;
        if patch.full_access.unwrap_or(current.full_access) && !full_access_allowed {
            return Ok(failure("A scheduled task cannot exceed the session's access level"));
        }
        if patch.job_type.as_ref().is_some_and(|kind| *kind != current.job_type) {
            match patch.job_type {
                Some(JobType::Shell)
                    if patch.command.as_deref().is_none_or(|command| command.trim().is_empty()) =>
                {
                    return Ok(failure("Supply a 'command' when switching to a Shell task"));
                }
                Some(JobType::Agent)
                    if patch.prompt.as_deref().is_none_or(|prompt| prompt.trim().is_empty()) =>
                {
                    return Ok(failure("Supply a 'prompt' when switching to an Agent task"));
                }
                _ => {}
            }
        }
        if (patch.schedule.is_some() || patch.starts_at.is_some() || patch.ends_at.is_some())
            && let Err(error) = validate_candidates(
                patch.schedule.as_ref().unwrap_or(&current.schedule),
                patch.starts_at.unwrap_or(current.starts_at),
                patch.ends_at.unwrap_or(current.ends_at),
                Utc::now(),
            )
        {
            return Ok(failure(error));
        }
        if let Some(delivery) = delivery {
            match delivery.apply(current.delivery, &self.config, self.delivery_host).await {
                Ok(delivery) => patch.delivery = Some(delivery),
                Err(error) => return Ok(failure(error.to_string())),
            }
        }
        match cron::update_job_with_access_limit(&self.config, job_id, patch, full_access_allowed) {
            Ok(mut job) => {
                job.delivery.to = cron::delivery::public_target(&job.delivery).map(str::to_owned);
                Ok(ToolResult {
                    success: true,
                    output: serde_json::to_string_pretty(&job)?,
                    error: None,
                })
            }
            Err(error) => Ok(failure(error.to_string())),
        }
    }
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
