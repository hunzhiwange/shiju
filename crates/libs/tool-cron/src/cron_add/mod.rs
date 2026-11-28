//! 创建 Shell 或 Agent 定时任务，沿用当前工具调用的权限与项目范围。

use super::{
    failure,
    options::{job_properties, parse_options},
};
use agent_config::runtime::Config;
use agent_cron::{self as cron, JobType, delivery::DeliveryHost};
use agent_security::{AutonomyLevel, SecurityPolicy};
use async_trait::async_trait;
use serde_json::{Value, json};
use std::{
    path::{Path, PathBuf},
    sync::Arc,
};
use tool_types::{Tool, ToolResult};

pub struct CronAddTool {
    config: Arc<Config>,
    security: Arc<SecurityPolicy>,
    default_project_path: Option<PathBuf>,
    default_full_access: Option<bool>,
    delivery_host: Option<&'static dyn DeliveryHost>,
}

impl CronAddTool {
    pub fn new(config: Arc<Config>, security: Arc<SecurityPolicy>) -> Self {
        Self {
            config,
            security,
            default_project_path: None,
            default_full_access: None,
            delivery_host: None,
        }
    }

    /// 显式参数优先；新任务的权限受当前会话约束。
    pub fn with_execution_defaults(
        mut self,
        project_path: impl AsRef<Path>,
        full_access: bool,
    ) -> Self {
        self.default_project_path = Some(project_path.as_ref().to_path_buf());
        self.default_full_access = Some(full_access);
        self
    }

    pub fn with_delivery_host(mut self, host: &'static dyn DeliveryHost) -> Self {
        self.delivery_host = Some(host);
        self
    }

    fn enforce_mutation_allowed(&self) -> Result<(), &'static str> {
        if !self.security.can_act() {
            return Err("Security policy: read-only mode, cannot perform 'cron_add'");
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
impl Tool for CronAddTool {
    fn name(&self) -> &str {
        "cron_add"
    }

    fn description(&self) -> &str {
        "Create a Shell or Agent scheduled task with a calendar, biweekly, one-time, or elapsed interval rule. Configure the execution window, project, Agent models, and optional channel delivery. Use cron_preview to check upcoming run times."
    }

    fn parameters_schema(&self) -> Value {
        let mut properties = job_properties(false);
        properties.insert("approved".into(), json!({
            "type": "boolean",
            "description": "Explicit approval for Shell commands that require it under the current policy",
            "default": false
        }));
        json!({"type": "object", "properties": properties, "required": ["schedule"]})
    }

    async fn execute(&self, args: Value) -> anyhow::Result<ToolResult> {
        if !self.config.cron.enabled {
            return Ok(failure("cron is disabled by config (cron.enabled=false)"));
        }
        let job_type = match args.get("job_type").and_then(Value::as_str) {
            Some("agent") => JobType::Agent,
            Some("shell") => JobType::Shell,
            Some(other) => return Ok(failure(format!("Invalid job_type: {other}"))),
            None if args.get("prompt").is_some() => JobType::Agent,
            None => JobType::Shell,
        };
        let (mut options, delivery) = match parse_options(&args) {
            Ok(options) => options,
            Err(error) => return Ok(failure(error)),
        };
        let Some(schedule) = options.schedule.take() else {
            return Ok(failure("Missing 'schedule' parameter"));
        };
        options.project_path = options
            .project_path
            .map(|path| path.trim().to_owned())
            .filter(|path| !path.is_empty())
            .or_else(|| {
                self.default_project_path.as_ref().map(|path| path.to_string_lossy().into_owned())
            });
        let full_access = options.full_access.or(self.default_full_access).unwrap_or(false);
        if let Some(path) = options.project_path.as_deref()
            && self.default_full_access == Some(false)
            && (!Path::new(path).is_absolute() || !self.security.is_path_allowed(path))
        {
            return Ok(failure("Scheduled task project is outside the permitted workspace"));
        }
        let full_access_allowed = self.default_full_access.unwrap_or(
            self.security.autonomy == AutonomyLevel::Full && !self.security.workspace_only,
        );
        if full_access && !full_access_allowed {
            return Ok(failure("A scheduled task cannot exceed the session's access level"));
        }
        options.full_access = Some(full_access);

        if job_type == JobType::Shell {
            let Some(command) =
                options.command.as_deref().filter(|command| !command.trim().is_empty())
            else {
                return Ok(failure("Missing 'command' for shell job"));
            };
            let approved = args.get("approved").and_then(Value::as_bool).unwrap_or(false);
            if let Err(error) = self.security.validate_command_execution(command, approved) {
                return Ok(failure(error));
            }
            options.prompt = None;
            options.model = None;
            options.session_target = None;
        } else {
            if options.prompt.as_deref().is_none_or(|prompt| prompt.trim().is_empty()) {
                return Ok(failure("Missing 'prompt' for agent job"));
            }
            options.command = None;
        }
        options.job_type = Some(job_type);
        if let Err(error) = self.enforce_mutation_allowed() {
            return Ok(failure(error));
        }
        if let Some(delivery) = delivery {
            match delivery.apply(Default::default(), &self.config, self.delivery_host).await {
                Ok(delivery) => options.delivery = Some(delivery),
                Err(error) => return Ok(failure(error.to_string())),
            }
        }

        match cron::create_job(&self.config, schedule, options) {
            Ok(mut job) => {
                job.delivery.to = cron::delivery::public_target(&job.delivery).map(str::to_owned);
                Ok(ToolResult {
                    success: true,
                    output: serde_json::to_string_pretty(&json!({
                        "id": job.id,
                        "name": job.name,
                        "job_type": job.job_type,
                        "schedule": job.schedule,
                        "starts_at": job.starts_at,
                        "ends_at": job.ends_at,
                        "next_run": job.next_run,
                        "enabled": job.enabled,
                        "agent": job.agent,
                        "acp_agent": job.acp_agent,
                        "model": job.model,
                        "fallbacks": job.fallbacks,
                        "project_path": job.project_path,
                        "full_access": job.full_access,
                        "task_pool": job.task_pool,
                        "wake": job.wake,
                        "delivery": job.delivery,
                        "delete_after_run": job.delete_after_run,
                    }))?,
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
