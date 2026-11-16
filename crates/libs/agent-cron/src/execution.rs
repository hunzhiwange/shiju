//! 执行参数与模型候选的纯领域规则。
use crate::{CronJob, JobType};
use agent_config::runtime::Config;
use std::path::PathBuf;

pub fn effective_job_config(config: &Config, job: &CronJob) -> Config {
    let Some(project_path) =
        job.project_path.as_deref().map(str::trim).filter(|value| !value.is_empty())
    else {
        return config.clone();
    };

    let mut next = config.clone();
    next.workspace_dir = PathBuf::from(project_path);
    next
}

pub fn effective_job_type(job: &CronJob) -> JobType {
    if matches!(job.job_type, JobType::Shell)
        && job.command.trim().is_empty()
        && job.prompt.as_deref().map(str::trim).is_some_and(|value| !value.is_empty())
    {
        return JobType::Agent;
    }
    job.job_type.clone()
}

pub fn invalid_job_payload(job: &CronJob) -> Option<String> {
    match effective_job_type(job) {
        JobType::Shell if job.command.trim().is_empty() => {
            Some("invalid cron job: shell command is empty; job disabled".to_string())
        }
        JobType::Agent
            if job.prompt.as_deref().map(str::trim).is_none_or(|value| value.is_empty()) =>
        {
            Some("invalid cron job: agent prompt is empty; job disabled".to_string())
        }
        _ => None,
    }
}

pub fn normalize_agent_job_success_output(output: &str) -> String {
    let output = output.trim();
    if output.is_empty() { "agent job executed".to_string() } else { output.to_string() }
}

pub fn agent_model_candidates(
    primary_model: Option<String>,
    fallbacks: &[String],
) -> Vec<Option<String>> {
    let mut model_candidates = Vec::new();
    if let Some(model) = primary_model.filter(|value| !value.trim().is_empty()) {
        model_candidates.push(Some(model));
    } else {
        model_candidates.push(None);
    }
    for fallback in fallbacks {
        let fallback = fallback.trim();
        if !fallback.is_empty()
            && !model_candidates.iter().any(|candidate| candidate.as_deref() == Some(fallback))
        {
            model_candidates.push(Some(fallback.to_string()));
        }
    }
    model_candidates
}

/// Task-pool identity normalization shared by the actual host adapter and domain assertions.
pub struct TaskPoolAgents {
    pub agent: String,
    pub acp_agent: Option<String>,
}

pub fn task_pool_agents(job: &CronJob) -> TaskPoolAgents {
    let agent = job.agent.as_deref().map(str::trim).filter(|value| !value.is_empty()).unwrap_or("main").to_string();
    let acp_agent = job.acp_agent.as_deref().map(str::trim).filter(|value| !value.is_empty()).map(str::to_string);
    TaskPoolAgents { agent, acp_agent }
}
