//! 定时任务（Cron）API 处理器模块
//!
//! 本模块提供与定时任务管理相关的 HTTP API 端点处理器，包括：
//! - 列出所有定时任务
//! - 添加新的定时任务
//! - 删除指定的定时任务
//! - 更新指定的定时任务
//!
//! 所有端点均需要身份认证，通过 `auth::require_auth` 进行验证。
//!
//! # API 端点
//!
//! - `GET /api/cron` - 获取所有定时任务列表
//! - `POST /api/cron` - 添加新的定时任务
//! - `DELETE /api/cron/:id` - 删除指定的定时任务
//! - `PATCH /api/cron/:id` - 更新指定的定时任务

use super::super::types::{CronAddBody, CronUpdateBody};
use crate::app::agent::gateway::AppState;
use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Json as JsonResponse},
};
use chrono::Utc;

#[path = "cron_schedule.rs"]
mod cron_schedule;
use cron_schedule::{cron_add_schedule, cron_update_schedule, validate_candidates, window_patch};

fn cron_job_json(job: &crate::app::agent::cron::CronJob) -> serde_json::Value {
    let job_type: &str = job.job_type.clone().into();
    let (schedule_kind, at, every_ms) = match &job.schedule {
        crate::app::agent::cron::Schedule::Cron { .. } => ("cron", None, None),
        crate::app::agent::cron::Schedule::Biweekly { .. } => ("biweekly", None, None),
        crate::app::agent::cron::Schedule::At { at } => ("at", Some(at.to_rfc3339()), None),
        crate::app::agent::cron::Schedule::Every { every_ms } => ("every", None, Some(*every_ms)),
    };
    let mut value = serde_json::json!({
        "starts_at": job.starts_at.map(|at| at.to_rfc3339()),
        "ends_at": job.ends_at.map(|at| at.to_rfc3339()),
        "id": job.id,
        "name": job.name,
        "job_type": job_type,
        "schedule_kind": schedule_kind,
        "expression": job.expression,
        "at": at,
        "every_ms": every_ms,
        "command": job.command,
        "prompt": job.prompt,
        "model": job.model,
        "agent": job.agent,
        "acp_agent": job.acp_agent,
        "project_path": job.project_path,
        "wake": job.wake,
        "fallbacks": job.fallbacks,
        "full_access": job.full_access,
        "task_pool": job.task_pool,
        "delivery_mode": job.delivery.mode,
        "delivery_channel": job.delivery.channel,
        "delivery_to": job.delivery.to,
        "delivery_best_effort": job.delivery.best_effort,
        "delete_after_run": job.delete_after_run,
        "next_run": job.next_run.to_rfc3339(),
        "last_run": job.last_run.map(|t| t.to_rfc3339()),
        "last_status": job.last_status,
        "last_output": job.last_output,
        "enabled": job.enabled,
    });
    if let Some(fields) = cron_schedule::schedule_json(&job.schedule).as_object() {
        value.as_object_mut().expect("job object").extend(fields.clone());
    }
    value
}

fn cron_run_json(run: &crate::app::agent::cron::CronRun) -> serde_json::Value {
    serde_json::json!({
        "id": run.id,
        "job_id": run.job_id,
        "started_at": run.started_at.to_rfc3339(),
        "finished_at": run.finished_at.to_rfc3339(),
        "status": run.status,
        "output": run.output,
        "duration_ms": run.duration_ms,
    })
}

fn trimmed_optional(value: Option<String>) -> Option<String> {
    value.and_then(|raw| {
        let value = raw.trim();
        (!value.is_empty()).then(|| value.to_string())
    })
}

fn cron_add_delivery(body: &CronAddBody) -> crate::app::agent::cron::DeliveryConfig {
    crate::app::agent::cron::DeliveryConfig {
        mode: body
            .delivery_mode
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .unwrap_or("none")
            .to_string(),
        channel: body
            .delivery_channel
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string),
        to: body
            .delivery_to
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string),
        best_effort: body.delivery_best_effort.unwrap_or(true),
    }
}

fn cron_add_patch(body: &CronAddBody) -> crate::app::agent::cron::CronJobPatch {
    crate::app::agent::cron::CronJobPatch {
        delivery: Some(cron_add_delivery(body)),
        delete_after_run: body.delete_after_run,
        agent: trimmed_optional(body.agent.clone()),
        acp_agent: body.acp_agent.clone(),
        project_path: trimmed_optional(body.project_path.clone()),
        wake: body.wake,
        fallbacks: body.fallbacks.clone().map(crate::app::agent::cron::normalize_fallbacks),
        full_access: body.full_access,
        task_pool: body.task_pool,
        ..crate::app::agent::cron::CronJobPatch::default()
    }
}

fn cron_add_job_type(body: &CronAddBody) -> String {
    let raw = body.job_type.as_deref().map(str::trim).filter(|value| !value.is_empty());
    if let Some(value) = raw {
        return value.to_ascii_lowercase();
    }

    let has_prompt = body.prompt.as_deref().is_some_and(|value| !value.trim().is_empty());
    let has_command = body.command.as_deref().is_some_and(|value| !value.trim().is_empty());
    if has_prompt && !has_command { "agent".to_string() } else { "shell".to_string() }
}

fn effective_schedule_kind(body: &CronAddBody) -> String {
    let raw = body
        .schedule_kind
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("cron");
    match raw {
        "at" | "指定时间" => "at".to_string(),
        "biweekly" => "biweekly".to_string(),
        "every" | "固定间隔" => "every".to_string(),
        "cron" | "Cron" => {
            let has_schedule =
                body.schedule.as_deref().is_some_and(|value| !value.trim().is_empty());
            if has_schedule {
                "cron".to_string()
            } else if body.every_ms.is_some_and(|value| value > 0) {
                "every".to_string()
            } else if body.local_at.is_some()
                || body.at.as_deref().is_some_and(|value| !value.trim().is_empty())
            {
                "at".to_string()
            } else {
                "cron".to_string()
            }
        }
        other => other.to_string(),
    }
}

fn cron_add_body_log_fields(body: &CronAddBody) -> (&str, String, usize, bool, bool, bool, bool) {
    let job_type = body.job_type.as_deref().map(str::trim).unwrap_or("shell");
    let schedule_kind = effective_schedule_kind(body);
    let schedule_fields =
        body.schedule.as_deref().map(|value| value.split_whitespace().count()).unwrap_or(0);
    (
        job_type,
        schedule_kind,
        schedule_fields,
        body.schedule.as_deref().is_some_and(|value| !value.trim().is_empty()),
        body.at.as_deref().is_some_and(|value| !value.trim().is_empty()),
        body.command.as_deref().is_some_and(|value| !value.trim().is_empty()),
        body.prompt.as_deref().is_some_and(|value| !value.trim().is_empty()),
    )
}

fn cron_field_error(error: String) -> axum::response::Response {
    (StatusCode::BAD_REQUEST, JsonResponse(serde_json::json!({"error": error}))).into_response()
}

/// 计算最多五次执行候选；身份认证先于校验，不创建任务或运行记录。
pub async fn handle_api_cron_preview(
    State(state): State<AppState>,
    headers: HeaderMap,
    JsonResponse(body): JsonResponse<CronAddBody>,
) -> axum::response::Response {
    if let Err(error) = super::super::auth::require_auth(&state, &headers) {
        return error.into_response();
    }
    match cron_schedule::preview(&body, Utc::now()) {
        Ok(value) => JsonResponse(value).into_response(),
        Err(error) => cron_field_error(error),
    }
}

/// 处理定时任务列表请求
///
/// 返回当前系统中所有已配置的定时任务，包括其 ID、名称、命令、
/// 下次执行时间、上次执行时间、上次执行状态以及是否启用等信息。
///
/// # 参数
///
/// - `State(state)` - 应用状态，包含配置和运行时信息
/// - `headers` - HTTP 请求头，用于身份认证
///
/// # 返回值
///
/// 成功时返回 JSON 格式的任务列表：
/// ```json
/// {
///   "jobs": [
///     {
///       "id": "job_123",
///       "name": "备份任务",
///       "command": "/usr/bin/backup.sh",
///       "next_run": "2024-01-01T00:00:00Z",
///       "last_run": "2023-12-31T00:00:00Z",
///       "last_status": "success",
///       "enabled": true
///     }
///   ]
/// }
/// ```
///
/// 失败时返回 500 状态码和错误信息。
///
/// # 认证
///
/// 需要有效的认证令牌，否则返回 401 未授权错误。
pub async fn handle_api_cron_list(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    // 验证请求的身份认证信息
    if let Err(e) = super::super::auth::require_auth(&state, &headers) {
        return e.into_response();
    }

    // 获取配置的克隆副本以避免长时间持有锁
    let config = state.config.lock().clone();

    // 尝试列出所有定时任务
    match crate::app::agent::cron::list_jobs(&config) {
        Ok(jobs) => {
            // 将任务列表转换为 JSON 格式
            let jobs_json: Vec<serde_json::Value> = jobs.iter().map(cron_job_json).collect();
            JsonResponse(serde_json::json!({"jobs": jobs_json})).into_response()
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            JsonResponse(serde_json::json!({"error": format!("Failed to list cron jobs: {e}")})),
        )
            .into_response(),
    }
}

/// 当前进程工作区的运行快照与最近结束记录；执行器必须与 API 同进程。
pub async fn handle_api_cron_history(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> axum::response::Response {
    if let Err(error) = super::super::auth::require_auth(&state, &headers) {
        return error.into_response();
    }
    use crate::app::agent::cron;
    let config = state.config.lock().clone();
    // 先复制快照，再查询结束记录；交接窗口优先展示真实持久化结果。
    let active = cron::active_runs::snapshot(&config);
    let result = (|| -> anyhow::Result<serde_json::Value> {
        let mut history = cron::list_history(&config)?;
        let jobs = cron::list_jobs(&config)?;
        let truncated = history.len() > cron::CRON_HISTORY_LIMIT;
        let completed: std::collections::HashSet<_> =
            history.iter().map(|entry| (entry.run.job_id.clone(), entry.run.started_at)).collect();
        history.truncate(cron::CRON_HISTORY_LIMIT);
        let mut rows = Vec::new();
        for entry in history {
            let run = entry.run;
            rows.push((
                run.started_at,
                Some(run.id),
                String::new(),
                serde_json::json!({
                    "id": run.id, "execution_id": null, "job_id": run.job_id,
                    "name": entry.name, "status": run.status,
                    "started_at": run.started_at.to_rfc3339(),
                    "finished_at": run.finished_at.to_rfc3339(),
                    "duration_ms": run.duration_ms, "output": run.output,
                }),
            ));
        }
        for run in active {
            if !jobs.iter().any(|job| job.id == run.job_id)
                || completed.contains(&(run.job_id.clone(), run.started_at))
            {
                continue;
            }
            rows.push((
                run.started_at,
                None,
                run.execution_id.clone(),
                serde_json::json!({
                    "id": null, "execution_id": run.execution_id, "job_id": run.job_id,
                    "name": run.name, "status": "running",
                    "started_at": run.started_at.to_rfc3339(),
                    "finished_at": null, "duration_ms": null, "output": null,
                }),
            ));
        }
        rows.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| b.1.cmp(&a.1)).then_with(|| b.2.cmp(&a.2)));
        let runs: Vec<_> = rows.into_iter().map(|(_, _, _, value)| value).collect();
        Ok(
            serde_json::json!({"runs": runs, "finished_limit": cron::CRON_HISTORY_LIMIT, "truncated": truncated}),
        )
    })();
    match result {
        Ok(value) => JsonResponse(value).into_response(),
        // 数据库错误可能携带路径或损坏的载荷；不向客户端或日志透出原值。
        Err(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            JsonResponse(serde_json::json!({"error": "Failed to query cron history"})),
        )
            .into_response(),
    }
}

/// 处理定时任务历史执行记录请求。
pub async fn handle_api_cron_runs(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> impl IntoResponse {
    if let Err(e) = super::super::auth::require_auth(&state, &headers) {
        return e.into_response();
    }

    let config = state.config.lock().clone();
    let limit = config.cron.max_run_history.clamp(1, 10_000) as usize;
    match crate::app::agent::cron::list_runs(&config, &id, limit) {
        Ok(runs) => {
            let runs_json: Vec<serde_json::Value> = runs.iter().map(cron_run_json).collect();
            JsonResponse(serde_json::json!({"runs": runs_json})).into_response()
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            JsonResponse(serde_json::json!({"error": format!("Failed to list cron runs: {e}")})),
        )
            .into_response(),
    }
}

/// 处理定时任务更新请求。
///
/// 该接口只暴露桌面管理界面当前需要的字段；不支持的任务类型与投递策略保持原值。
pub async fn handle_api_cron_update(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
    JsonResponse(body): JsonResponse<CronUpdateBody>,
) -> impl IntoResponse {
    if let Err(e) = super::super::auth::require_auth(&state, &headers) {
        return e.into_response();
    }

    let config = state.config.lock().clone();
    tracing::info!(
        target: "vw_gateway_cron",
        action = "update",
        job_id = %id,
        name_present = body.name.as_deref().is_some_and(|value| !value.trim().is_empty()),
        job_type_present = body.job_type.as_deref().is_some_and(|value| !value.trim().is_empty()),
        schedule_kind_present = body.schedule_kind.as_deref().is_some_and(|value| !value.trim().is_empty()),
        schedule_present = body.schedule.as_deref().is_some_and(|value| !value.trim().is_empty()),
        at_present = body.at.as_deref().is_some_and(|value| !value.trim().is_empty()),
        every_ms_present = body.every_ms.is_some(),
        command_present = body.command.as_deref().is_some_and(|value| !value.trim().is_empty()),
        prompt_present = body.prompt.as_deref().is_some_and(|value| !value.trim().is_empty()),
        enabled = ?body.enabled,
        "cron update request received"
    );
    let current = crate::app::agent::cron::get_job(&config, &id).ok();
    let schedule = match cron_update_schedule(&body, current.as_ref()) {
        Ok(schedule) => schedule,
        Err(err) => {
            return (StatusCode::BAD_REQUEST, JsonResponse(serde_json::json!({"error": err})))
                .into_response();
        }
    };
    let (starts_at, ends_at) = match window_patch(
        &body.starts_at,
        &body.ends_at,
        &body.start_date,
        &body.end_date,
        body.timezone.as_deref(),
    ) {
        Ok(window) => window,
        Err(error) => return cron_field_error(error),
    };
    if let Some(current) = &current {
        if schedule.is_some() || starts_at.is_some() || ends_at.is_some() {
            if let Err(error) = validate_candidates(
                schedule.as_ref().unwrap_or(&current.schedule),
                starts_at.unwrap_or(current.starts_at),
                ends_at.unwrap_or(current.ends_at),
                Utc::now(),
            ) {
                return cron_field_error(error);
            }
        }
    }
    let job_type = match body.job_type.as_deref().map(str::trim).filter(|value| !value.is_empty()) {
        Some(value) => match crate::app::agent::cron::JobType::try_from(value) {
            Ok(job_type) => Some(job_type),
            Err(err) => {
                return (StatusCode::BAD_REQUEST, JsonResponse(serde_json::json!({"error": err})))
                    .into_response();
            }
        },
        None => None,
    };
    let delivery = if body.delivery_mode.is_some()
        || body.delivery_channel.is_some()
        || body.delivery_to.is_some()
        || body.delivery_best_effort.is_some()
    {
        Some(crate::app::agent::cron::DeliveryConfig {
            mode: body
                .delivery_mode
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .unwrap_or("none")
                .to_string(),
            channel: trimmed_optional(body.delivery_channel.clone()),
            to: trimmed_optional(body.delivery_to.clone()),
            best_effort: body.delivery_best_effort.unwrap_or(true),
        })
    } else {
        None
    };
    let patch = crate::app::agent::cron::CronJobPatch {
        starts_at,
        ends_at,
        job_type,
        schedule,
        command: body.command,
        prompt: body.prompt,
        name: body.name,
        enabled: body.enabled,
        delivery,
        model: body.model,
        session_target: body
            .session_target
            .as_deref()
            .map(crate::app::agent::cron::SessionTarget::parse),
        delete_after_run: body.delete_after_run,
        agent: body.agent,
        acp_agent: body.acp_agent,
        project_path: body.project_path,
        wake: body.wake,
        fallbacks: body.fallbacks.map(crate::app::agent::cron::normalize_fallbacks),
        full_access: body.full_access,
        task_pool: body.task_pool,
        ..crate::app::agent::cron::CronJobPatch::default()
    };

    match crate::app::agent::cron::update_job(&config, &id, patch) {
        Ok(job) => JsonResponse(serde_json::json!({
            "status": "ok",
            "job": cron_job_json(&job),
        }))
        .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            JsonResponse(serde_json::json!({"error": format!("Failed to update cron job: {e}")})),
        )
            .into_response(),
    }
}

/// 处理添加定时任务请求
///
/// 根据请求体中的信息创建新的定时任务。任务创建后将按照
/// 指定的 cron 表达式自动执行。
///
/// # 参数
///
/// - `State(state)` - 应用状态，包含配置和运行时信息
/// - `headers` - HTTP 请求头，用于身份认证
/// - `Json(body)` - 请求体，包含任务名称、调度表达式和要执行的命令
///
/// # 返回值
///
/// 成功时返回新创建的任务信息：
/// ```json
/// {
///   "status": "ok",
///   "job": {
///     "id": "job_456",
///     "name": "日志清理",
///     "command": "/usr/bin/clean-logs.sh",
///     "enabled": true
///   }
/// }
/// ```
///
/// 失败时返回 500 状态码和错误信息。
///
/// # 认证
///
/// 需要有效的认证令牌，否则返回 401 未授权错误。
pub async fn handle_api_cron_add(
    State(state): State<AppState>,
    headers: HeaderMap,
    JsonResponse(body): JsonResponse<CronAddBody>,
) -> impl IntoResponse {
    // 验证请求的身份认证信息
    if let Err(e) = super::super::auth::require_auth(&state, &headers) {
        return e.into_response();
    }

    // 获取配置的克隆副本以避免长时间持有锁
    let config = state.config.lock().clone();
    let (
        log_job_type,
        log_schedule_kind,
        log_schedule_fields,
        log_has_schedule,
        log_has_at,
        log_has_command,
        log_has_prompt,
    ) = cron_add_body_log_fields(&body);
    tracing::info!(
        target: "vw_gateway_cron",
        action = "add",
        job_type = %log_job_type,
        schedule_kind = %log_schedule_kind,
        schedule_present = log_has_schedule,
        schedule_fields = log_schedule_fields,
        at_present = log_has_at,
        every_ms_present = body.every_ms.is_some(),
        command_present = log_has_command,
        prompt_present = log_has_prompt,
        agent_present = body.agent.as_deref().is_some_and(|value| !value.trim().is_empty()),
        project_path_present = body.project_path.as_deref().is_some_and(|value| !value.trim().is_empty()),
        delivery_mode = %body.delivery_mode.as_deref().unwrap_or("none"),
        "cron add request received"
    );

    let schedule = match cron_add_schedule(&body) {
        Ok(schedule) => schedule,
        Err(err) => {
            tracing::warn!(
                target: "vw_gateway_cron",
                action = "add",
                job_type = %log_job_type,
                schedule_kind = %log_schedule_kind,
                error = %err,
                "cron add request rejected"
            );
            return (
                StatusCode::BAD_REQUEST,
                JsonResponse(
                    serde_json::json!({"error": format!("Failed to add cron job: {err}")}),
                ),
            )
                .into_response();
        }
    };

    let (starts_at, ends_at) = match window_patch(
        &body.starts_at,
        &body.ends_at,
        &body.start_date,
        &body.end_date,
        body.timezone.as_deref(),
    ) {
        Ok((start, end)) => (start.flatten(), end.flatten()),
        Err(error) => return cron_field_error(error),
    };
    if let Err(error) = validate_candidates(&schedule, starts_at, ends_at, Utc::now()) {
        return cron_field_error(format!("Failed to add cron job: {error}"));
    }
    let job_type = cron_add_job_type(&body);
    let result = match job_type.as_str() {
        "shell" => {
            let Some(command) =
                body.command.as_deref().map(str::trim).filter(|value| !value.is_empty())
            else {
                return (
                    StatusCode::BAD_REQUEST,
                    JsonResponse(serde_json::json!({"error": "Shell command is required"})),
                )
                    .into_response();
            };
            let job = crate::app::agent::cron::add_shell_job_with_window(
                &config,
                body.name.clone(),
                schedule,
                command,
                starts_at,
                ends_at,
            );
            match job {
                Ok(job) => {
                    crate::app::agent::cron::update_job(&config, &job.id, cron_add_patch(&body))
                }
                Err(err) => Err(err),
            }
        }
        "agent" => {
            let Some(prompt) =
                body.prompt.as_deref().map(str::trim).filter(|value| !value.is_empty())
            else {
                return (
                    StatusCode::BAD_REQUEST,
                    JsonResponse(serde_json::json!({"error": "Agent prompt is required"})),
                )
                    .into_response();
            };
            let session_target = body
                .session_target
                .as_deref()
                .map(crate::app::agent::cron::SessionTarget::parse)
                .unwrap_or_default();
            let job = crate::app::agent::cron::add_agent_job_with_window(
                &config,
                body.name.clone(),
                schedule,
                prompt,
                session_target,
                trimmed_optional(body.model.clone()),
                Some(cron_add_delivery(&body)),
                body.delete_after_run.unwrap_or(false),
                starts_at,
                ends_at,
            );
            match job {
                Ok(job) => {
                    crate::app::agent::cron::update_job(&config, &job.id, cron_add_patch(&body))
                }
                Err(err) => Err(err),
            }
        }
        other => {
            return (
                StatusCode::BAD_REQUEST,
                JsonResponse(
                    serde_json::json!({"error": format!("Unsupported job_type: {other}")}),
                ),
            )
                .into_response();
        }
    };

    match result {
        Ok(job) => JsonResponse(serde_json::json!({
            "status": "ok",
            "job": cron_job_json(&job),
        }))
        .into_response(),
        Err(e) => {
            tracing::warn!(
                target: "vw_gateway_cron",
                action = "add",
                job_type = %log_job_type,
                schedule_kind = %log_schedule_kind,
                error = %e,
                "cron add failed"
            );
            let error = e.to_string();
            let status = if error.contains("Invalid cron expression")
                || error.contains("schedule")
                || error.contains("Cron expression")
            {
                StatusCode::BAD_REQUEST
            } else {
                StatusCode::INTERNAL_SERVER_ERROR
            };
            (
                status,
                JsonResponse(
                    serde_json::json!({"error": format!("Failed to add cron job: {error}")}),
                ),
            )
                .into_response()
        }
    }
}

/// 处理删除定时任务请求
///
/// 根据 ID 删除指定的定时任务。删除后任务将不再执行。
///
/// # 参数
///
/// - `State(state)` - 应用状态，包含配置和运行时信息
/// - `headers` - HTTP 请求头，用于身份认证
/// - `Path(id)` - URL 路径参数，指定要删除的任务 ID
///
/// # 返回值
///
/// 成功时返回：
/// ```json
/// {
///   "status": "ok"
/// }
/// ```
///
/// 失败时返回 500 状态码和错误信息。
///
/// # 认证
///
/// 需要有效的认证令牌，否则返回 401 未授权错误。
pub async fn handle_api_cron_delete(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> impl IntoResponse {
    // 验证请求的身份认证信息
    if let Err(e) = super::super::auth::require_auth(&state, &headers) {
        return e.into_response();
    }

    // 获取配置的克隆副本以避免长时间持有锁
    let config = state.config.lock().clone();

    // 尝试删除指定的任务
    match crate::app::agent::cron::remove_job(&config, &id) {
        Ok(()) => JsonResponse(serde_json::json!({"status": "ok"})).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            JsonResponse(serde_json::json!({"error": format!("Failed to remove cron job: {e}")})),
        )
            .into_response(),
    }
}

#[cfg(test)]
#[path = "cron_tests.rs"]
mod cron_tests;
