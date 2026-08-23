//! 在桌面配置层规范化定时任务调度参数并调用网关接口。

use super::gateway::gateway_client;
use vw_gateway_client::{CronAddRequest, CronJobDto, CronRunDto, CronUpdateRequest};

fn normalize_schedule_kind(
    value: &str,
    schedule: Option<&str>,
    at: Option<&str>,
    every_ms: Option<u64>,
) -> Result<String, String> {
    let kind = value.trim();
    if kind.is_empty() {
        return Ok("cron".to_string());
    }
    match kind.to_ascii_lowercase().as_str() {
        "at" | "指定时间" => Ok("at".to_string()),
        "every" | "固定间隔" => Ok("every".to_string()),
        "cron" => {
            if schedule.is_some_and(|value| !value.trim().is_empty()) {
                Ok("cron".to_string())
            } else if every_ms.is_some() {
                Ok("every".to_string())
            } else if at.is_some_and(|value| !value.trim().is_empty()) {
                Ok("at".to_string())
            } else {
                Ok("cron".to_string())
            }
        }
        other => Err(format!("不支持的调度类型: {other}")),
    }
}

fn legacy_schedule_expression(
    schedule_kind: &str,
    schedule: Option<&str>,
    every_ms: Option<u64>,
) -> String {
    if schedule_kind == "cron" {
        return schedule.unwrap_or_default().to_string();
    }

    if schedule_kind != "every" {
        return "0 * * * * *".to_string();
    }

    let seconds = every_ms.unwrap_or(1_000).div_ceil(1_000).clamp(1, 59);
    format!("*/{seconds} * * * * *")
}

fn requires_extended_cron_api(
    job_type: &str,
    schedule_kind: &str,
    agent: Option<&String>,
    acp_agent: Option<&String>,
    project_path: Option<&String>,
    model: Option<&String>,
    fallbacks: Option<&Vec<String>>,
    task_pool: bool,
) -> bool {
    job_type != "shell"
        || schedule_kind != "cron"
        || agent.is_some_and(|value| !value.trim().is_empty())
        || acp_agent.is_some_and(|value| !value.trim().is_empty())
        || project_path.is_some_and(|value| !value.trim().is_empty())
        || model.is_some_and(|value| !value.trim().is_empty())
        || fallbacks.is_some_and(|values| !values.is_empty())
        || task_pool
}

async fn ensure_extended_cron_api_if_needed(
    client: &vw_gateway_client::GatewayClient,
    job_type: &str,
    schedule_kind: &str,
    agent: Option<&String>,
    acp_agent: Option<&String>,
    project_path: Option<&String>,
    model: Option<&String>,
    fallbacks: Option<&Vec<String>>,
    task_pool: bool,
) -> Result<(), String> {
    if !requires_extended_cron_api(
        job_type,
        schedule_kind,
        agent,
        acp_agent,
        project_path,
        model,
        fallbacks,
        task_pool,
    ) {
        return Ok(());
    }

    if client.cron_extended_api_supported().await? {
        Ok(())
    } else {
        Err("当前网关版本过旧，不支持 Agent/固定间隔定时任务；请停止旧 gateway/daemon 后重启新版 daemon".to_string())
    }
}

/// 通过当前网关加载全部定时任务。
pub async fn load_cron_jobs_async() -> Result<Vec<CronJobDto>, String> {
    let client = gateway_client()?;
    client.cron_jobs_get().await
}

/// 加载指定定时任务的历史运行记录。
pub async fn load_cron_job_runs_async(job_id: String) -> Result<Vec<CronRunDto>, String> {
    let client = gateway_client()?;
    match client.cron_job_runs_get(&job_id).await {
        Ok(runs) => Ok(runs),
        Err(err) if err.contains("不支持定时任务历史接口") => {
            let jobs = client.cron_jobs_get().await?;
            let Some(job) = jobs.into_iter().find(|job| job.id == job_id) else {
                return Err(err);
            };
            let Some(last_run) = job.last_run.clone() else {
                return Ok(Vec::new());
            };
            Ok(vec![CronRunDto {
                id: 0,
                job_id,
                started_at: last_run.clone(),
                finished_at: last_run,
                status: job.last_status.unwrap_or_else(|| "unknown".to_string()),
                output: job.last_output,
                duration_ms: None,
            }])
        }
        Err(err) => Err(err),
    }
}

/// 规范化旧版与扩展调度字段后创建定时任务。
pub async fn add_cron_job_async(
    name: String,
    job_type: String,
    schedule_kind: String,
    schedule: String,
    at: String,
    every_ms: String,
    command: String,
    prompt: String,
    session_target: String,
    agent: String,
    acp_agent: String,
    project_path: String,
    wake: bool,
    model: String,
    fallbacks: String,
    full_access: bool,
    task_pool: bool,
    delivery_enabled: bool,
    delivery_channel: String,
    delivery_to: String,
    delivery_best_effort: bool,
    delete_after_run: bool,
) -> Result<(), String> {
    let name = Some(name.trim().to_string()).filter(|value| !value.is_empty());
    let job_type = job_type.trim().to_string();
    let schedule_value = schedule.trim().to_string();
    let schedule = Some(schedule_value.clone()).filter(|value| !value.is_empty());
    let at_value = at.trim().to_string();
    let at = Some(at_value.clone()).filter(|value| !value.is_empty());
    let every_ms = if every_ms.trim().is_empty() {
        None
    } else {
        Some(every_ms.trim().parse::<u64>().map_err(|err| format!("固定间隔毫秒数无效: {err}"))?)
    };
    let schedule_kind =
        normalize_schedule_kind(&schedule_kind, schedule.as_deref(), at.as_deref(), every_ms)?;
    let command_value = command.trim().to_string();
    let command = Some(command_value.clone()).filter(|value| !value.is_empty());
    let prompt_value = prompt.trim().to_string();
    let prompt = Some(prompt_value.clone()).filter(|value| !value.is_empty());
    let session_target_value = session_target.trim().to_string();
    let session_target = Some(session_target_value.clone()).filter(|value| !value.is_empty());
    let agent = if job_type == "agent" {
        Some(agent.trim().to_string()).filter(|value| !value.is_empty())
    } else {
        None
    };
    let acp_agent = if job_type == "agent" { Some(acp_agent.trim().to_string()) } else { None };
    let task_pool = job_type == "agent" && task_pool;
    let project_path = Some(project_path.trim().to_string()).filter(|value| !value.is_empty());
    let model = Some(model.trim().to_string()).filter(|value| !value.is_empty());
    let fallbacks = fallbacks
        .split([',', '\n'])
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .collect::<Vec<_>>();
    let fallbacks = (!fallbacks.is_empty()).then_some(fallbacks);
    let delivery_channel =
        Some(delivery_channel.trim().to_string()).filter(|value| !value.is_empty());
    let delivery_to = Some(delivery_to.trim().to_string()).filter(|value| !value.is_empty());

    if schedule_kind == "cron" && schedule.is_none() {
        return Err("Cron 表达式不能为空".to_string());
    }
    if schedule_kind == "at" && at.is_none() {
        return Err("指定时间不能为空".to_string());
    }
    if schedule_kind == "every" && every_ms.is_none() {
        return Err("固定间隔毫秒数不能为空".to_string());
    }
    if job_type == "shell" && command.is_none() {
        return Err("执行命令不能为空".to_string());
    }
    if job_type == "agent" && prompt.is_none() {
        return Err("Agent 提示词不能为空".to_string());
    }
    if delivery_enabled && delivery_channel.is_none() {
        return Err("投递通道不能为空".to_string());
    }
    if delivery_enabled && delivery_to.is_none() {
        return Err("投递目标不能为空".to_string());
    }

    let client = gateway_client()?;
    ensure_extended_cron_api_if_needed(
        &client,
        &job_type,
        &schedule_kind,
        agent.as_ref(),
        acp_agent.as_ref(),
        project_path.as_ref(),
        model.as_ref(),
        fallbacks.as_ref(),
        task_pool,
    )
    .await?;
    tracing::info!(
        target: "vw_desktop_cron",
        job_type = %job_type,
        schedule_kind = %schedule_kind,
        schedule_present = schedule.is_some(),
        schedule_fields = schedule.as_deref().map(|value| value.split_whitespace().count()).unwrap_or(0),
        at_present = at.is_some(),
        every_ms_present = every_ms.is_some(),
        command_present = command.is_some(),
        prompt_present = prompt.is_some(),
        agent_present = agent.is_some(),
        project_path_present = project_path.is_some(),
        delivery_enabled,
        "submitting cron job"
    );
    let request = CronAddRequest {
        name,
        job_type,
        schedule_kind: schedule_kind.clone(),
        schedule: legacy_schedule_expression(&schedule_kind, schedule.as_deref(), every_ms),
        at: if schedule_kind == "at" { at } else { None },
        every_ms: if schedule_kind == "every" { every_ms } else { None },
        command: Some(command.unwrap_or_default()),
        prompt: Some(prompt.unwrap_or_default()),
        session_target: Some(session_target.unwrap_or_default()),
        model,
        agent,
        acp_agent,
        project_path,
        wake: Some(wake),
        fallbacks,
        full_access: Some(full_access),
        task_pool: Some(task_pool),
        delivery_mode: Some(if delivery_enabled { "announce" } else { "none" }.to_string()),
        delivery_channel,
        delivery_to,
        delivery_best_effort: Some(delivery_best_effort),
        delete_after_run: Some(delete_after_run),
    };
    client.cron_job_add(&request).await.map(|_| ())
}

/// 规范化调度字段后更新指定定时任务。
pub async fn update_cron_job_async(
    job_id: String,
    name: String,
    job_type: String,
    schedule_kind: String,
    schedule: String,
    at: String,
    every_ms: String,
    command: String,
    prompt: String,
    session_target: String,
    agent: String,
    acp_agent: String,
    project_path: String,
    wake: bool,
    model: String,
    fallbacks: String,
    full_access: bool,
    task_pool: bool,
    delivery_enabled: bool,
    delivery_channel: String,
    delivery_to: String,
    delivery_best_effort: bool,
    delete_after_run: bool,
) -> Result<(), String> {
    let name = Some(name.trim().to_string()).filter(|value| !value.is_empty());
    let job_type = job_type.trim().to_string();
    let schedule_value = schedule.trim().to_string();
    let schedule = Some(schedule_value.clone()).filter(|value| !value.is_empty());
    let at_value = at.trim().to_string();
    let at = Some(at_value.clone()).filter(|value| !value.is_empty());
    let every_ms = if every_ms.trim().is_empty() {
        None
    } else {
        Some(every_ms.trim().parse::<u64>().map_err(|err| format!("固定间隔毫秒数无效: {err}"))?)
    };
    let schedule_kind =
        normalize_schedule_kind(&schedule_kind, schedule.as_deref(), at.as_deref(), every_ms)?;
    let command_value = command.trim().to_string();
    let command = Some(command_value.clone()).filter(|value| !value.is_empty());
    let prompt_value = prompt.trim().to_string();
    let prompt = Some(prompt_value.clone()).filter(|value| !value.is_empty());
    let session_target_value = session_target.trim().to_string();
    let session_target = Some(session_target_value.clone()).filter(|value| !value.is_empty());
    let agent = if job_type == "agent" {
        Some(agent.trim().to_string()).filter(|value| !value.is_empty())
    } else {
        None
    };
    let acp_agent = if job_type == "agent" { Some(acp_agent.trim().to_string()) } else { None };
    let task_pool = job_type == "agent" && task_pool;
    let project_path = Some(project_path.trim().to_string()).filter(|value| !value.is_empty());
    let model = Some(model.trim().to_string()).filter(|value| !value.is_empty());
    let fallbacks = fallbacks
        .split([',', '\n'])
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .collect::<Vec<_>>();
    let fallbacks = (!fallbacks.is_empty()).then_some(fallbacks);
    let delivery_channel =
        Some(delivery_channel.trim().to_string()).filter(|value| !value.is_empty());
    let delivery_to = Some(delivery_to.trim().to_string()).filter(|value| !value.is_empty());

    if schedule_kind == "cron" && schedule.is_none() {
        return Err("Cron 表达式不能为空".to_string());
    }
    if schedule_kind == "at" && at.is_none() {
        return Err("指定时间不能为空".to_string());
    }
    if schedule_kind == "every" && every_ms.is_none() {
        return Err("固定间隔毫秒数不能为空".to_string());
    }
    if job_type == "shell" && command.is_none() {
        return Err("执行命令不能为空".to_string());
    }
    if job_type == "agent" && prompt.is_none() {
        return Err("Agent 提示词不能为空".to_string());
    }
    if delivery_enabled && delivery_channel.is_none() {
        return Err("投递通道不能为空".to_string());
    }
    if delivery_enabled && delivery_to.is_none() {
        return Err("投递目标不能为空".to_string());
    }

    let client = gateway_client()?;
    ensure_extended_cron_api_if_needed(
        &client,
        &job_type,
        &schedule_kind,
        agent.as_ref(),
        acp_agent.as_ref(),
        project_path.as_ref(),
        model.as_ref(),
        fallbacks.as_ref(),
        task_pool,
    )
    .await?;
    let request = CronUpdateRequest {
        name,
        job_type: Some(job_type),
        schedule_kind: Some(schedule_kind.clone()),
        schedule: if schedule_kind == "cron" { schedule } else { None },
        at: if schedule_kind == "at" { at } else { None },
        every_ms: if schedule_kind == "every" { every_ms } else { None },
        command: Some(command.unwrap_or_default()),
        prompt: Some(prompt.unwrap_or_default()),
        session_target: Some(session_target.unwrap_or_default()),
        model,
        agent,
        acp_agent,
        project_path,
        wake: Some(wake),
        fallbacks,
        full_access: Some(full_access),
        task_pool: Some(task_pool),
        delivery_mode: Some(if delivery_enabled { "announce" } else { "none" }.to_string()),
        delivery_channel,
        delivery_to,
        delivery_best_effort: Some(delivery_best_effort),
        delete_after_run: Some(delete_after_run),
        enabled: None,
    };
    client.cron_job_update(&job_id, &request).await.map(|_| ())
}

pub async fn set_cron_job_enabled_async(job_id: String, enabled: bool) -> Result<(), String> {
    let client = gateway_client()?;
    let request = CronUpdateRequest { enabled: Some(enabled), ..CronUpdateRequest::default() };
    client.cron_job_update(&job_id, &request).await.map(|_| ())
}

/// 批量更新启用状态；任一请求失败时返回对应错误。
pub async fn set_cron_jobs_enabled_async(
    job_ids: Vec<String>,
    enabled: bool,
) -> Result<(), String> {
    let client = gateway_client()?;
    let request = CronUpdateRequest { enabled: Some(enabled), ..CronUpdateRequest::default() };
    for job_id in job_ids {
        client.cron_job_update(&job_id, &request).await?;
    }
    Ok(())
}

pub async fn delete_cron_job_async(job_id: String) -> Result<(), String> {
    let client = gateway_client()?;
    client.cron_job_delete(&job_id).await
}

/// 逐个删除所给任务，保留网关返回的首个失败原因。
pub async fn delete_cron_jobs_async(job_ids: Vec<String>) -> Result<(), String> {
    let client = gateway_client()?;
    for job_id in job_ids {
        client.cron_job_delete(&job_id).await?;
    }
    Ok(())
}

#[cfg(test)]
#[path = "config_cron_jobs_tests.rs"]
mod config_cron_jobs_tests;
