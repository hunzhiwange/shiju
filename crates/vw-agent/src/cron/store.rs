//! 定时任务与运行记录的持久化操作，并统一调度校验和输出截断边界。
//!
//! 原生平台使用 SQLite；WASM 保留可编译接口但明确返回空集合或不支持错误。

use super::{
    CronJob, CronJobPatch, CronRun, DeliveryConfig, JobType, Schedule, SessionTarget,
    next_run_for_schedule, normalize_fallbacks, schedule_cron_expression, validate_schedule,
};
use crate::app::agent::config::Config;
use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
#[cfg(not(target_arch = "wasm32"))]
use rusqlite::types::{FromSqlResult, ValueRef};
#[cfg(not(target_arch = "wasm32"))]
use rusqlite::{Connection, params};
use uuid::Uuid;

/// 单条运行输出允许持久化的最大 UTF-8 字节数。
pub(crate) const MAX_CRON_OUTPUT_BYTES: usize = 16 * 1024;
/// 输出被截断时附加的稳定标记。
pub(crate) const TRUNCATED_OUTPUT_MARKER: &str = "\n...[truncated]";

#[cfg(not(target_arch = "wasm32"))]
impl rusqlite::types::FromSql for JobType {
    fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
        let text = value.as_str()?;
        JobType::try_from(text).map_err(|e| rusqlite::types::FromSqlError::Other(e.into()))
    }
}

/// 使用 cron 表达式创建默认设置的 shell 任务。
pub fn add_job(config: &Config, expression: &str, command: &str) -> Result<CronJob> {
    let schedule = Schedule::Cron { expr: expression.to_string(), tz: None };
    add_shell_job(config, None, schedule, command)
}

#[cfg(not(target_arch = "wasm32"))]
/// 校验计划并持久化一个 shell 任务。
pub fn add_shell_job(
    config: &Config,
    name: Option<String>,
    schedule: Schedule,
    command: &str,
) -> Result<CronJob> {
    let now = Utc::now();
    validate_schedule(&schedule, now)?;
    let next_run = next_run_for_schedule(&schedule, now)?;
    let id = Uuid::new_v4().to_string();
    let expression = schedule_cron_expression(&schedule).unwrap_or_default();
    let schedule_json = serde_json::to_string(&schedule)?;

    let delete_after_run = matches!(schedule, Schedule::At { .. });

    with_connection(config, |conn| {
        conn.execute(
            "INSERT INTO cron_jobs (
                id, expression, command, schedule, job_type, prompt, name, session_target, model,
                enabled, delivery, delete_after_run, created_at, next_run, agent, acp_agent,
                project_path, wake, fallbacks, full_access, task_pool
             ) VALUES (?1, ?2, ?3, ?4, 'shell', NULL, ?5, 'isolated', NULL, 1, ?6, ?7, ?8, ?9, NULL, NULL, NULL, 0, ?10, 0, 0)",
            params![
                id,
                expression,
                command,
                schedule_json,
                name,
                serde_json::to_string(&DeliveryConfig::default())?,
                if delete_after_run { 1 } else { 0 },
                now.to_rfc3339(),
                next_run.to_rfc3339(),
                serde_json::to_string(&Vec::<String>::new())?,
            ],
        )
        .context("Failed to insert cron shell job")?;
        Ok(())
    })?;

    let job = get_job(config, &id)?;
    super::scheduler::notify_schedule_changed();
    Ok(job)
}

#[cfg(target_arch = "wasm32")]
/// WASM 平台不支持持久化 shell 定时任务。
pub fn add_shell_job(
    _config: &Config,
    _name: Option<String>,
    _schedule: Schedule,
    _command: &str,
) -> Result<CronJob> {
    anyhow::bail!("Cron jobs are not supported on WASM")
}

#[allow(clippy::too_many_arguments)]
#[cfg(not(target_arch = "wasm32"))]
/// 校验计划并持久化一个代理任务及其会话、模型和投递策略。
pub fn add_agent_job(
    config: &Config,
    name: Option<String>,
    schedule: Schedule,
    prompt: &str,
    session_target: SessionTarget,
    model: Option<String>,
    delivery: Option<DeliveryConfig>,
    delete_after_run: bool,
) -> Result<CronJob> {
    let now = Utc::now();
    validate_schedule(&schedule, now)?;
    let next_run = next_run_for_schedule(&schedule, now)?;
    let id = Uuid::new_v4().to_string();
    let expression = schedule_cron_expression(&schedule).unwrap_or_default();
    let schedule_json = serde_json::to_string(&schedule)?;
    let delivery = delivery.unwrap_or_default();

    with_connection(config, |conn| {
        conn.execute(
            "INSERT INTO cron_jobs (
                id, expression, command, schedule, job_type, prompt, name, session_target, model,
                enabled, delivery, delete_after_run, created_at, next_run, agent, acp_agent,
                project_path, wake, fallbacks, full_access, task_pool
             ) VALUES (?1, ?2, '', ?3, 'agent', ?4, ?5, ?6, ?7, 1, ?8, ?9, ?10, ?11, NULL, NULL, NULL, 0, ?12, 0, 0)",
            params![
                id,
                expression,
                schedule_json,
                prompt,
                name,
                session_target.as_str(),
                model,
                serde_json::to_string(&delivery)?,
                if delete_after_run { 1 } else { 0 },
                now.to_rfc3339(),
                next_run.to_rfc3339(),
                serde_json::to_string(&Vec::<String>::new())?,
            ],
        )
        .context("Failed to insert cron agent job")?;
        Ok(())
    })?;

    let job = get_job(config, &id)?;
    super::scheduler::notify_schedule_changed();
    Ok(job)
}

#[cfg(target_arch = "wasm32")]
/// WASM 平台不支持持久化代理定时任务。
pub fn add_agent_job(
    _config: &Config,
    _name: Option<String>,
    _schedule: Schedule,
    _prompt: &str,
    _session_target: SessionTarget,
    _model: Option<String>,
    _delivery: Option<DeliveryConfig>,
    _delete_after_run: bool,
) -> Result<CronJob> {
    anyhow::bail!("Cron jobs are not supported on WASM")
}

#[cfg(not(target_arch = "wasm32"))]
/// 按下次运行时间升序列出全部任务。
pub fn list_jobs(config: &Config) -> Result<Vec<CronJob>> {
    with_connection(config, |conn| {
        let mut stmt = conn.prepare(
            "SELECT id, expression, command, schedule, job_type, prompt, name, session_target, model,
                    enabled, delivery, delete_after_run, created_at, next_run, last_run, last_status, last_output,
                    agent, acp_agent, project_path, wake, fallbacks, full_access, task_pool
             FROM cron_jobs ORDER BY next_run ASC",
        )?;

        let rows = stmt.query_map([], map_cron_job_row)?;

        let mut jobs = Vec::new();
        for row in rows {
            jobs.push(row?);
        }
        Ok(jobs)
    })
}

#[cfg(target_arch = "wasm32")]
/// WASM 平台返回空任务列表。
pub fn list_jobs(_config: &Config) -> Result<Vec<CronJob>> {
    Ok(Vec::new())
}

#[cfg(not(target_arch = "wasm32"))]
/// 按标识读取任务；不存在时返回明确错误。
pub fn get_job(config: &Config, job_id: &str) -> Result<CronJob> {
    with_connection(config, |conn| {
        let mut stmt = conn.prepare(
            "SELECT id, expression, command, schedule, job_type, prompt, name, session_target, model,
                    enabled, delivery, delete_after_run, created_at, next_run, last_run, last_status, last_output,
                    agent, acp_agent, project_path, wake, fallbacks, full_access, task_pool
             FROM cron_jobs WHERE id = ?1",
        )?;

        let mut rows = stmt.query(params![job_id])?;
        if let Some(row) = rows.next()? {
            map_cron_job_row(row).map_err(Into::into)
        } else {
            anyhow::bail!("Cron job '{job_id}' not found")
        }
    })
}

#[cfg(target_arch = "wasm32")]
/// WASM 平台始终报告任务不存在。
pub fn get_job(_config: &Config, job_id: &str) -> Result<CronJob> {
    anyhow::bail!("Cron job '{job_id}' not found (WASM)")
}

#[cfg(not(target_arch = "wasm32"))]
/// 删除任务并通知调度器刷新；标识不存在时失败。
pub fn remove_job(config: &Config, id: &str) -> Result<()> {
    let changed = with_connection(config, |conn| {
        conn.execute("DELETE FROM cron_jobs WHERE id = ?1", params![id])
            .context("Failed to delete cron job")
    })?;

    if changed == 0 {
        anyhow::bail!("Cron job '{id}' not found");
    }

    println!("✅ Removed cron job {id}");
    super::scheduler::notify_schedule_changed();
    Ok(())
}

#[cfg(target_arch = "wasm32")]
/// WASM 平台不支持删除定时任务。
pub fn remove_job(_config: &Config, _id: &str) -> Result<()> {
    anyhow::bail!("Cron jobs are not supported on WASM")
}

#[cfg(not(target_arch = "wasm32"))]
/// 返回已启用且到期的任务，数量受调度器并发上限约束。
pub fn due_jobs(config: &Config, now: DateTime<Utc>) -> Result<Vec<CronJob>> {
    let lim = i64::try_from(config.scheduler.max_tasks.max(1))
        .context("Scheduler max_tasks overflows i64")?;
    with_connection(config, |conn| {
        let mut stmt = conn.prepare(
            "SELECT id, expression, command, schedule, job_type, prompt, name, session_target, model,
                    enabled, delivery, delete_after_run, created_at, next_run, last_run, last_status, last_output,
                    agent, acp_agent, project_path, wake, fallbacks, full_access, task_pool
             FROM cron_jobs
             WHERE enabled = 1 AND next_run <= ?1
             ORDER BY next_run ASC
             LIMIT ?2",
        )?;

        let rows = stmt.query_map(params![now.to_rfc3339(), lim], map_cron_job_row)?;

        let mut jobs = Vec::new();
        for row in rows {
            jobs.push(row?);
        }
        Ok(jobs)
    })
}

#[cfg(target_arch = "wasm32")]
/// WASM 平台没有可调度任务。
pub fn due_jobs(_config: &Config, _now: DateTime<Utc>) -> Result<Vec<CronJob>> {
    Ok(Vec::new())
}

#[cfg(not(target_arch = "wasm32"))]
/// 应用任务补丁；计划变化时重新计算下次运行时间并通知调度器。
pub fn update_job(config: &Config, job_id: &str, patch: CronJobPatch) -> Result<CronJob> {
    let mut job = get_job(config, job_id)?;
    let mut schedule_changed = false;

    if let Some(schedule) = patch.schedule {
        validate_schedule(&schedule, Utc::now())?;
        job.schedule = schedule;
        job.expression = schedule_cron_expression(&job.schedule).unwrap_or_default();
        schedule_changed = true;
    }
    if let Some(job_type) = patch.job_type {
        job.job_type = job_type;
    }
    if let Some(command) = patch.command {
        job.command = command;
    }
    if let Some(prompt) = patch.prompt {
        job.prompt = Some(prompt);
    }
    if let Some(name) = patch.name {
        job.name = Some(name);
    }
    if let Some(enabled) = patch.enabled {
        job.enabled = enabled;
    }
    if let Some(delivery) = patch.delivery {
        job.delivery = delivery;
    }
    if let Some(model) = patch.model {
        job.model = Some(model);
    }
    if let Some(target) = patch.session_target {
        job.session_target = target;
    }
    if let Some(delete_after_run) = patch.delete_after_run {
        job.delete_after_run = delete_after_run;
    }
    if let Some(agent) = patch.agent {
        job.agent = Some(agent);
    }
    if let Some(acp_agent) = patch.acp_agent {
        job.acp_agent = Some(acp_agent);
    }
    if let Some(project_path) = patch.project_path {
        job.project_path = Some(project_path);
    }
    if let Some(wake) = patch.wake {
        job.wake = wake;
    }
    if let Some(fallbacks) = patch.fallbacks {
        job.fallbacks = normalize_fallbacks(fallbacks);
    }
    if let Some(full_access) = patch.full_access {
        job.full_access = full_access;
    }
    if let Some(task_pool) = patch.task_pool {
        job.task_pool = task_pool;
    }

    if schedule_changed {
        job.next_run = next_run_for_schedule(&job.schedule, Utc::now())?;
    }

    with_connection(config, |conn| {
        conn.execute(
            "UPDATE cron_jobs
             SET expression = ?1, command = ?2, schedule = ?3, job_type = ?4, prompt = ?5, name = ?6,
                 session_target = ?7, model = ?8, enabled = ?9, delivery = ?10, delete_after_run = ?11,
                 agent = ?12, acp_agent = ?13, project_path = ?14, wake = ?15, fallbacks = ?16,
                 full_access = ?17, task_pool = ?18, next_run = ?19
             WHERE id = ?20",
            params![
                job.expression,
                job.command,
                serde_json::to_string(&job.schedule)?,
                <JobType as Into<&str>>::into(job.job_type).to_string(),
                job.prompt,
                job.name,
                job.session_target.as_str(),
                job.model,
                if job.enabled { 1 } else { 0 },
                serde_json::to_string(&job.delivery)?,
                if job.delete_after_run { 1 } else { 0 },
                job.agent,
                job.acp_agent,
                job.project_path,
                if job.wake { 1 } else { 0 },
                serde_json::to_string(&job.fallbacks)?,
                if job.full_access { 1 } else { 0 },
                if job.task_pool { 1 } else { 0 },
                job.next_run.to_rfc3339(),
                job.id,
            ],
        )
        .context("Failed to update cron job")?;
        Ok(())
    })?;

    let job = get_job(config, job_id)?;
    super::scheduler::notify_schedule_changed();
    Ok(job)
}

#[cfg(target_arch = "wasm32")]
/// WASM 平台不支持更新定时任务。
pub fn update_job(_config: &Config, _job_id: &str, _patch: CronJobPatch) -> Result<CronJob> {
    anyhow::bail!("Cron jobs are not supported on WASM")
}

#[cfg(not(target_arch = "wasm32"))]
/// 更新任务最近一次执行摘要，输出会按 UTF-8 边界截断。
pub fn record_last_run(
    config: &Config,
    job_id: &str,
    finished_at: DateTime<Utc>,
    success: bool,
    output: &str,
) -> Result<()> {
    let status = if success { "ok" } else { "error" };
    let bounded_output = truncate_cron_output(output);
    with_connection(config, |conn| {
        conn.execute(
            "UPDATE cron_jobs
             SET last_run = ?1, last_status = ?2, last_output = ?3
             WHERE id = ?4",
            params![finished_at.to_rfc3339(), status, bounded_output, job_id],
        )
        .context("Failed to update cron last run fields")?;
        Ok(())
    })
}

#[cfg(target_arch = "wasm32")]
/// WASM 兼容实现不持久化最近运行摘要。
pub fn record_last_run(
    _config: &Config,
    _job_id: &str,
    _finished_at: DateTime<Utc>,
    _success: bool,
    _output: &str,
) -> Result<()> {
    Ok(())
}

#[cfg(not(target_arch = "wasm32"))]
/// 记录本次结果并按计划推进任务的下次运行时间。
pub fn reschedule_after_run(
    config: &Config,
    job: &CronJob,
    success: bool,
    output: &str,
) -> Result<()> {
    let now = Utc::now();
    let next_run = next_run_for_schedule(&job.schedule, now)?;
    let status = if success { "ok" } else { "error" };
    let bounded_output = truncate_cron_output(output);

    with_connection(config, |conn| {
        conn.execute(
            "UPDATE cron_jobs
             SET next_run = ?1, last_run = ?2, last_status = ?3, last_output = ?4
             WHERE id = ?5",
            params![next_run.to_rfc3339(), now.to_rfc3339(), status, bounded_output, job.id],
        )
        .context("Failed to update cron job run state")?;
        Ok(())
    })
}

#[cfg(target_arch = "wasm32")]
/// WASM 兼容实现不持久化重新调度结果。
pub fn reschedule_after_run(
    _config: &Config,
    _job: &CronJob,
    _success: bool,
    _output: &str,
) -> Result<()> {
    Ok(())
}

#[cfg(not(target_arch = "wasm32"))]
/// 原子写入运行记录并按配置裁剪该任务的旧历史。
pub fn record_run(
    config: &Config,
    job_id: &str,
    started_at: DateTime<Utc>,
    finished_at: DateTime<Utc>,
    status: &str,
    output: Option<&str>,
    duration_ms: i64,
) -> Result<()> {
    let bounded_output = output.map(truncate_cron_output);
    with_connection(config, |conn| {
        // 插入与裁剪必须处于同一事务；裁剪失败时回滚插入，避免历史表无界增长。
        let tx = conn.unchecked_transaction()?;

        tx.execute(
            "INSERT INTO cron_runs (job_id, started_at, finished_at, status, output, duration_ms)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                job_id,
                started_at.to_rfc3339(),
                finished_at.to_rfc3339(),
                status,
                bounded_output.as_deref(),
                duration_ms,
            ],
        )
        .context("Failed to insert cron run")?;

        let keep = i64::from(config.cron.max_run_history.max(1));
        tx.execute(
            "DELETE FROM cron_runs
             WHERE job_id = ?1
               AND id NOT IN (
                 SELECT id FROM cron_runs
                 WHERE job_id = ?1
                 ORDER BY started_at DESC, id DESC
                 LIMIT ?2
               )",
            params![job_id, keep],
        )
        .context("Failed to prune cron run history")?;

        tx.commit().context("Failed to commit cron run transaction")?;
        Ok(())
    })
}

#[cfg(target_arch = "wasm32")]
/// WASM 兼容实现不持久化运行历史。
pub fn record_run(
    _config: &Config,
    _job_id: &str,
    _started_at: DateTime<Utc>,
    _finished_at: DateTime<Utc>,
    _status: &str,
    _output: Option<&str>,
    _duration_ms: i64,
) -> Result<()> {
    Ok(())
}

fn truncate_cron_output(output: &str) -> String {
    if output.len() <= MAX_CRON_OUTPUT_BYTES {
        return output.to_string();
    }

    if MAX_CRON_OUTPUT_BYTES <= TRUNCATED_OUTPUT_MARKER.len() {
        return TRUNCATED_OUTPUT_MARKER.to_string();
    }

    let mut cutoff = MAX_CRON_OUTPUT_BYTES - TRUNCATED_OUTPUT_MARKER.len();
    while cutoff > 0 && !output.is_char_boundary(cutoff) {
        cutoff -= 1;
    }

    let mut truncated = output[..cutoff].to_string();
    truncated.push_str(TRUNCATED_OUTPUT_MARKER);
    truncated
}

#[cfg(not(target_arch = "wasm32"))]
/// 按开始时间倒序列出指定任务的运行历史，最少查询一条。
pub fn list_runs(config: &Config, job_id: &str, limit: usize) -> Result<Vec<CronRun>> {
    with_connection(config, |conn| {
        let lim = i64::try_from(limit.max(1)).context("Run history limit overflow")?;
        let mut stmt = conn.prepare(
            "SELECT id, job_id, started_at, finished_at, status, output, duration_ms
             FROM cron_runs
             WHERE job_id = ?1
             ORDER BY started_at DESC, id DESC
             LIMIT ?2",
        )?;

        let rows = stmt.query_map(params![job_id, lim], |row| {
            Ok(CronRun {
                id: row.get(0)?,
                job_id: row.get(1)?,
                started_at: parse_rfc3339(&row.get::<_, String>(2)?)
                    .map_err(sql_conversion_error)?,
                finished_at: parse_rfc3339(&row.get::<_, String>(3)?)
                    .map_err(sql_conversion_error)?,
                status: row.get(4)?,
                output: row.get(5)?,
                duration_ms: row.get(6)?,
            })
        })?;

        let mut runs = Vec::new();
        for row in rows {
            runs.push(row?);
        }
        Ok(runs)
    })
}

#[cfg(target_arch = "wasm32")]
/// WASM 平台返回空运行历史。
pub fn list_runs(_config: &Config, _job_id: &str, _limit: usize) -> Result<Vec<CronRun>> {
    Ok(Vec::new())
}

#[cfg(not(target_arch = "wasm32"))]
fn parse_rfc3339(raw: &str) -> Result<DateTime<Utc>> {
    let parsed = DateTime::parse_from_rfc3339(raw)
        .with_context(|| format!("Invalid RFC3339 timestamp in cron DB: {raw}"))?;
    Ok(parsed.with_timezone(&Utc))
}

#[cfg(not(target_arch = "wasm32"))]
fn sql_conversion_error(err: anyhow::Error) -> rusqlite::Error {
    rusqlite::Error::ToSqlConversionFailure(err.into())
}

#[cfg(not(target_arch = "wasm32"))]
fn map_cron_job_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<CronJob> {
    let expression: String = row.get(1)?;
    let schedule_raw: Option<String> = row.get(3)?;
    let schedule =
        decode_schedule(schedule_raw.as_deref(), &expression).map_err(sql_conversion_error)?;

    let delivery_raw: Option<String> = row.get(10)?;
    let delivery = decode_delivery(delivery_raw.as_deref()).map_err(sql_conversion_error)?;
    let fallbacks_raw: Option<String> = row.get(21)?;
    let fallbacks = decode_fallbacks(fallbacks_raw.as_deref()).map_err(sql_conversion_error)?;
    let full_access = row.get::<_, i64>(22)? != 0;
    let task_pool = row.get::<_, i64>(23)? != 0;

    let next_run_raw: String = row.get(13)?;
    let last_run_raw: Option<String> = row.get(14)?;
    let created_at_raw: String = row.get(12)?;

    Ok(CronJob {
        id: row.get(0)?,
        expression,
        schedule,
        command: row.get(2)?,
        job_type: row.get(4)?,
        prompt: row.get(5)?,
        name: row.get(6)?,
        session_target: SessionTarget::parse(&row.get::<_, String>(7)?),
        model: row.get(8)?,
        agent: row.get(17)?,
        acp_agent: row.get(18)?,
        project_path: row.get(19)?,
        wake: row.get::<_, i64>(20)? != 0,
        fallbacks,
        full_access,
        task_pool,
        enabled: row.get::<_, i64>(9)? != 0,
        delivery,
        delete_after_run: row.get::<_, i64>(11)? != 0,
        created_at: parse_rfc3339(&created_at_raw).map_err(sql_conversion_error)?,
        next_run: parse_rfc3339(&next_run_raw).map_err(sql_conversion_error)?,
        last_run: match last_run_raw {
            Some(raw) => Some(parse_rfc3339(&raw).map_err(sql_conversion_error)?),
            None => None,
        },
        last_status: row.get(15)?,
        last_output: row.get(16)?,
    })
}

#[cfg(not(target_arch = "wasm32"))]
fn decode_schedule(schedule_raw: Option<&str>, expression: &str) -> Result<Schedule> {
    if let Some(raw) = schedule_raw {
        let trimmed = raw.trim();
        if !trimmed.is_empty() {
            return serde_json::from_str(trimmed)
                .with_context(|| format!("Failed to parse cron schedule JSON: {trimmed}"));
        }
    }

    if expression.trim().is_empty() {
        anyhow::bail!("Missing schedule and legacy expression for cron job")
    }

    Ok(Schedule::Cron { expr: expression.to_string(), tz: None })
}

#[cfg(not(target_arch = "wasm32"))]
fn decode_delivery(delivery_raw: Option<&str>) -> Result<DeliveryConfig> {
    if let Some(raw) = delivery_raw {
        let trimmed = raw.trim();
        if !trimmed.is_empty() {
            return serde_json::from_str(trimmed)
                .with_context(|| format!("Failed to parse cron delivery JSON: {trimmed}"));
        }
    }
    Ok(DeliveryConfig::default())
}

#[cfg(not(target_arch = "wasm32"))]
fn decode_fallbacks(fallbacks_raw: Option<&str>) -> Result<Vec<String>> {
    if let Some(raw) = fallbacks_raw {
        let trimmed = raw.trim();
        if !trimmed.is_empty() {
            let parsed: Vec<String> = serde_json::from_str(trimmed)
                .with_context(|| format!("Failed to parse cron fallbacks JSON: {trimmed}"))?;
            return Ok(normalize_fallbacks(parsed));
        }
    }
    Ok(Vec::new())
}

#[cfg(not(target_arch = "wasm32"))]
fn add_column_if_missing(conn: &Connection, name: &str, sql_type: &str) -> Result<()> {
    let mut stmt = conn.prepare("PRAGMA table_info(cron_jobs)")?;
    let mut rows = stmt.query([])?;
    while let Some(row) = rows.next()? {
        let col_name: String = row.get(1)?;
        if col_name == name {
            return Ok(());
        }
    }
    // Drop the statement/rows before executing ALTER to release any locks
    drop(rows);
    drop(stmt);

    // Tolerate "duplicate column name" errors to handle the race where
    // another process adds the column between our PRAGMA check and ALTER.
    match conn.execute(&format!("ALTER TABLE cron_jobs ADD COLUMN {name} {sql_type}"), []) {
        Ok(_) => Ok(()),
        Err(rusqlite::Error::SqliteFailure(err, Some(ref msg)))
            if msg.contains("duplicate column name") =>
        {
            tracing::debug!("Column cron_jobs.{name} already exists (concurrent migration): {err}");
            Ok(())
        }
        Err(e) => Err(e).with_context(|| format!("Failed to add cron_jobs.{name}")),
    }
}

#[cfg(not(target_arch = "wasm32"))]
/// 打开定时任务数据库、确保表结构存在，并把连接交给闭包。
pub(crate) fn with_connection<T>(
    config: &Config,
    f: impl FnOnce(&Connection) -> Result<T>,
) -> Result<T> {
    let db_path = config.workspace_dir.join("cron").join("jobs.db");
    if let Some(parent) = db_path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create cron directory: {}", parent.display()))?;
    }

    let conn = Connection::open(&db_path)
        .with_context(|| format!("Failed to open cron DB: {}", db_path.display()))?;

    conn.execute_batch(
        "PRAGMA foreign_keys = ON;
         CREATE TABLE IF NOT EXISTS cron_jobs (
            id               TEXT PRIMARY KEY,
            expression       TEXT NOT NULL,
            command          TEXT NOT NULL,
            schedule         TEXT,
            job_type         TEXT NOT NULL DEFAULT 'shell',
            prompt           TEXT,
            name             TEXT,
            session_target   TEXT NOT NULL DEFAULT 'isolated',
            model            TEXT,
            enabled          INTEGER NOT NULL DEFAULT 1,
            delivery         TEXT,
            delete_after_run INTEGER NOT NULL DEFAULT 0,
            created_at       TEXT NOT NULL,
            next_run         TEXT NOT NULL,
            last_run         TEXT,
            last_status      TEXT,
            last_output      TEXT,
            agent            TEXT,
            acp_agent        TEXT,
            project_path     TEXT,
            wake             INTEGER NOT NULL DEFAULT 0,
            fallbacks        TEXT,
            full_access      INTEGER NOT NULL DEFAULT 0,
            task_pool        INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_cron_jobs_next_run ON cron_jobs(next_run);

        CREATE TABLE IF NOT EXISTS cron_runs (
            id          INTEGER PRIMARY KEY AUTOINCREMENT,
            job_id      TEXT NOT NULL,
            started_at  TEXT NOT NULL,
            finished_at TEXT NOT NULL,
            status      TEXT NOT NULL,
            output      TEXT,
            duration_ms INTEGER,
            FOREIGN KEY (job_id) REFERENCES cron_jobs(id) ON DELETE CASCADE
        );
        CREATE INDEX IF NOT EXISTS idx_cron_runs_job_id ON cron_runs(job_id);
        CREATE INDEX IF NOT EXISTS idx_cron_runs_started_at ON cron_runs(started_at);
        CREATE INDEX IF NOT EXISTS idx_cron_runs_job_started ON cron_runs(job_id, started_at);",
    )
    .context("Failed to initialize cron schema")?;

    add_column_if_missing(&conn, "schedule", "TEXT")?;
    add_column_if_missing(&conn, "job_type", "TEXT NOT NULL DEFAULT 'shell'")?;
    add_column_if_missing(&conn, "prompt", "TEXT")?;
    add_column_if_missing(&conn, "name", "TEXT")?;
    add_column_if_missing(&conn, "session_target", "TEXT NOT NULL DEFAULT 'isolated'")?;
    add_column_if_missing(&conn, "model", "TEXT")?;
    add_column_if_missing(&conn, "enabled", "INTEGER NOT NULL DEFAULT 1")?;
    add_column_if_missing(&conn, "delivery", "TEXT")?;
    add_column_if_missing(&conn, "delete_after_run", "INTEGER NOT NULL DEFAULT 0")?;
    add_column_if_missing(&conn, "agent", "TEXT")?;
    add_column_if_missing(&conn, "acp_agent", "TEXT")?;
    add_column_if_missing(&conn, "project_path", "TEXT")?;
    add_column_if_missing(&conn, "wake", "INTEGER NOT NULL DEFAULT 0")?;
    add_column_if_missing(&conn, "fallbacks", "TEXT")?;
    add_column_if_missing(&conn, "full_access", "INTEGER NOT NULL DEFAULT 0")?;
    add_column_if_missing(&conn, "task_pool", "INTEGER NOT NULL DEFAULT 0")?;

    f(&conn)
}
