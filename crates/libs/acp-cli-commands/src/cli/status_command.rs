//! 状态与会话查询命令的处理逻辑。

use std::io::{self, Write};

use serde::Serialize;
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

use acp_cli_model::cli::flags::{
    FlagsError, GlobalFlags, StatusFlags, resolve_agent_invocation, resolve_session_name_from_flags,
};
use acp_cli_render::cli::json_output::emit_json_result;
use acp_cli_render::cli::output_render::AgentSessionIdPayload;
use acp_cli_render::cli::output_render::agent_session_id_payload;
use acp_config::ResolvedAcpxConfig;
use acp_types::SessionRecord;
use acp_storage::{FindSessionOptions, SessionRepositoryError, find_session};
use acp_queue::probe_queue_owner_health;

#[derive(Debug, thiserror::Error)]
pub enum StatusCommandError {
    #[error(transparent)]
    Io(#[from] io::Error),
    #[error(transparent)]
    Flags(#[from] FlagsError),
    #[error(transparent)]
    SessionRepository(#[from] SessionRepositoryError),
    #[error("{0}")]
    Operational(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct NoSessionSnapshot {
    action: &'static str,
    status: &'static str,
    summary: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
struct StatusSnapshot {
    action: &'static str,
    status: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pid: Option<u32>,
    summary: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    mode: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    available_models: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    uptime: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    last_prompt_time: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    exit_code: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    signal: Option<String>,
    vwacp_record_id: String,
    vwacp_session_id: String,
    #[serde(flatten)]
    agent_session: AgentSessionIdPayload,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
struct StatusPayload {
    session_id: String,
    agent_command: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pid: Option<u32>,
    status: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    mode: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    available_models: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    uptime: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    last_prompt_time: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    exit_code: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    signal: Option<String>,
    #[serde(flatten)]
    agent_session: AgentSessionIdPayload,
}

fn format_uptime(started_at: Option<&str>) -> Option<String> {
    let started_at = started_at?;
    let started = OffsetDateTime::parse(started_at, &Rfc3339).ok()?;
    let elapsed_seconds = (OffsetDateTime::now_utc() - started).whole_seconds().max(0);
    let hours = elapsed_seconds / 3_600;
    let minutes = (elapsed_seconds % 3_600) / 60;
    let seconds = elapsed_seconds % 60;

    Some(format!("{hours:02}:{minutes:02}:{seconds:02}"))
}

fn build_status_payload(record: &SessionRecord, running: bool, pid: Option<u32>) -> StatusPayload {
    StatusPayload {
        session_id: record.vwacp_record_id.clone(),
        agent_command: record.agent_command.clone(),
        pid,
        status: if running { "running" } else { "dead" },
        model: record.vwacp.as_ref().and_then(|state| state.current_model_id.clone()),
        mode: record.vwacp.as_ref().and_then(|state| state.current_mode_id.clone()),
        available_models: record.vwacp.as_ref().and_then(|state| state.available_models.clone()),
        uptime: if running { format_uptime(record.agent_started_at.as_deref()) } else { None },
        last_prompt_time: record.last_prompt_at.clone(),
        exit_code: if running { None } else { record.last_agent_exit_code },
        signal: if running { None } else { record.last_agent_exit_signal.clone() },
        agent_session: agent_session_id_payload(record.agent_session_id.as_deref()),
    }
}

fn build_status_snapshot(
    record: &SessionRecord,
    payload: &StatusPayload,
    running: bool,
) -> StatusSnapshot {
    StatusSnapshot {
        action: "status_snapshot",
        status: if running { "alive" } else { "dead" },
        pid: payload.pid,
        summary: if running { "queue owner healthy" } else { "queue owner unavailable" },
        model: payload.model.clone(),
        mode: payload.mode.clone(),
        available_models: payload.available_models.clone(),
        uptime: payload.uptime.clone(),
        last_prompt_time: payload.last_prompt_time.clone(),
        exit_code: payload.exit_code,
        signal: payload.signal.clone(),
        vwacp_record_id: record.vwacp_record_id.clone(),
        vwacp_session_id: record.acp_session_id.clone(),
        agent_session: agent_session_id_payload(record.agent_session_id.as_deref()),
    }
}

#[allow(clippy::result_large_err)]
fn write_no_session_status<W: Write>(
    stdout: &mut W,
    global_flags: &GlobalFlags,
    agent_command: &str,
) -> Result<(), StatusCommandError> {
    if emit_json_result(
        stdout,
        global_flags.format,
        &NoSessionSnapshot {
            action: "status_snapshot",
            status: "no-session",
            summary: "no active session",
        },
    )? {
        return Ok(());
    }

    if global_flags.format == acp_types::OutputFormat::Quiet {
        writeln!(stdout, "no-session")?;
        return Ok(());
    }

    writeln!(stdout, "session: -")?;
    writeln!(stdout, "agent: {agent_command}")?;
    writeln!(stdout, "pid: -")?;
    writeln!(stdout, "status: no-session")?;
    writeln!(stdout, "model: -")?;
    writeln!(stdout, "mode: -")?;
    writeln!(stdout, "uptime: -")?;
    writeln!(stdout, "lastPromptTime: -")?;
    Ok(())
}

#[allow(clippy::result_large_err)]
fn write_status_payload<W: Write>(
    stdout: &mut W,
    global_flags: &GlobalFlags,
    record: &SessionRecord,
    payload: &StatusPayload,
    running: bool,
) -> Result<(), StatusCommandError> {
    if emit_json_result(
        stdout,
        global_flags.format,
        &build_status_snapshot(record, payload, running),
    )? {
        return Ok(());
    }

    if global_flags.format == acp_types::OutputFormat::Quiet {
        writeln!(stdout, "{}", payload.status)?;
        return Ok(());
    }

    writeln!(stdout, "session: {}", payload.session_id)?;
    if let Some(agent_session_id) = payload.agent_session.agent_session_id.as_deref() {
        writeln!(stdout, "agentSessionId: {agent_session_id}")?;
    }
    writeln!(stdout, "agent: {}", payload.agent_command)?;
    writeln!(
        stdout,
        "pid: {}",
        payload.pid.map(|value| value.to_string()).unwrap_or_else(|| "-".to_string())
    )?;
    writeln!(stdout, "status: {}", payload.status)?;
    writeln!(stdout, "model: {}", payload.model.as_deref().unwrap_or("-"))?;
    writeln!(stdout, "mode: {}", payload.mode.as_deref().unwrap_or("-"))?;
    writeln!(stdout, "uptime: {}", payload.uptime.as_deref().unwrap_or("-"))?;
    writeln!(stdout, "lastPromptTime: {}", payload.last_prompt_time.as_deref().unwrap_or("-"))?;
    if payload.status == "dead" {
        writeln!(
            stdout,
            "exitCode: {}",
            payload.exit_code.map(|value| value.to_string()).unwrap_or_else(|| "-".to_string())
        )?;
        writeln!(stdout, "signal: {}", payload.signal.as_deref().unwrap_or("-"))?;
    }
    Ok(())
}

pub async fn handle_status(
    explicit_agent_name: Option<&str>,
    flags: &StatusFlags,
    global_flags: &GlobalFlags,
    config: &ResolvedAcpxConfig,
) -> Result<(), StatusCommandError> {
    let agent = resolve_agent_invocation(explicit_agent_name, global_flags, config)?;
    let record = find_session(&FindSessionOptions {
        agent_command: agent.agent_command.clone(),
        cwd: agent.cwd.clone(),
        name: resolve_session_name_from_flags(flags, None)?,
        include_closed: false,
    })
    .await?;

    let stdout = io::stdout();
    let mut stdout = stdout.lock();

    let Some(record) = record else {
        return write_no_session_status(&mut stdout, global_flags, &agent.agent_command);
    };

    let health = probe_queue_owner_health(&record.vwacp_record_id).await;
    let running = health.healthy;
    let payload = build_status_payload(&record, running, health.pid.or(record.pid));
    write_status_payload(&mut stdout, global_flags, &record, &payload, running)
}

pub async fn handle_sessions_show(
    explicit_agent_name: Option<&str>,
    session_name: Option<&str>,
    global_flags: &GlobalFlags,
    config: &ResolvedAcpxConfig,
) -> Result<(), StatusCommandError> {
    let agent = resolve_agent_invocation(explicit_agent_name, global_flags, config)?;
    let record = find_session(&FindSessionOptions {
        agent_command: agent.agent_command.clone(),
        cwd: agent.cwd.clone(),
        name: session_name.map(ToString::to_string),
        include_closed: true,
    })
    .await?;

    let Some(record) = record else {
        let msg = if let Some(name) = session_name {
            format!(
                "No named session \"{}\" for cwd {} and agent {}",
                name, agent.cwd, agent.agent_name
            )
        } else {
            format!("No cwd session for {} and agent {}", agent.cwd, agent.agent_name)
        };
        return Err(StatusCommandError::Operational(msg));
    };

    acp_cli_render::cli::output_render::print_session_details_by_format(&record, global_flags.format)
        .map_err(|e| StatusCommandError::Operational(e.to_string()))
}

pub async fn handle_sessions_history(
    explicit_agent_name: Option<&str>,
    session_name: Option<&str>,
    limit: usize,
    global_flags: &GlobalFlags,
    config: &ResolvedAcpxConfig,
) -> Result<(), StatusCommandError> {
    let agent = resolve_agent_invocation(explicit_agent_name, global_flags, config)?;
    let record = find_session(&FindSessionOptions {
        agent_command: agent.agent_command.clone(),
        cwd: agent.cwd.clone(),
        name: session_name.map(ToString::to_string),
        include_closed: true,
    })
    .await?;

    let Some(record) = record else {
        let msg = if let Some(name) = session_name {
            format!(
                "No named session \"{}\" for cwd {} and agent {}",
                name, agent.cwd, agent.agent_name
            )
        } else {
            format!("No cwd session for {} and agent {}", agent.cwd, agent.agent_name)
        };
        return Err(StatusCommandError::Operational(msg));
    };

    acp_cli_render::cli::output_render::print_session_history_by_format(&record, global_flags.format, limit)
        .map_err(|e| StatusCommandError::Operational(e.to_string()))
}

#[cfg(test)]
#[path = "status_command_tests.rs"]
mod status_command_tests;
