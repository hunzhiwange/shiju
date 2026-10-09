//! CLI 文本输出的渲染与格式化辅助。

use std::fmt;
use std::io::{self, Write};
use std::path::{Component, MAIN_SEPARATOR, Path, PathBuf};

use serde::Serialize;
use serde_json::{Value, json};

use crate::cli::json_output::emit_json_result;
use crate::{
    OutputFormat, SessionAgentContent, SessionEnqueueResult, SessionMessage, SessionRecord,
    SessionToolResult, SessionToolResultContent, SessionUserContent, normalize_runtime_session_id,
    probe_queue_owner_health,
};

fn format_session_label(record: &SessionRecord) -> &str {
    record.name.as_deref().unwrap_or("cwd")
}

fn resolve_display_path(value: &str) -> PathBuf {
    let path = Path::new(value);
    if path.is_absolute() {
        return path.to_path_buf();
    }

    std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")).join(path)
}

fn relative_path(from: &Path, to: &Path) -> PathBuf {
    let from_components = from.components().collect::<Vec<_>>();
    let to_components = to.components().collect::<Vec<_>>();
    let common_prefix_len = from_components
        .iter()
        .zip(to_components.iter())
        .take_while(|(left, right)| left == right)
        .count();

    if common_prefix_len == 0
        && matches!(from_components.first(), Some(Component::RootDir))
        && matches!(to_components.first(), Some(Component::RootDir))
    {
        return to.to_path_buf();
    }

    let mut relative = PathBuf::new();

    for component in &from_components[common_prefix_len..] {
        if matches!(component, Component::Normal(_) | Component::ParentDir) {
            relative.push("..");
        }
    }

    for component in &to_components[common_prefix_len..] {
        match component {
            Component::Normal(value) => relative.push(value),
            Component::ParentDir => relative.push(".."),
            Component::CurDir => relative.push("."),
            Component::RootDir | Component::Prefix(_) => {}
        }
    }

    relative
}

fn format_routed_from(session_cwd: &Path, current_cwd: &Path) -> Option<String> {
    let relative = relative_path(session_cwd, current_cwd);
    if relative.as_os_str().is_empty() || relative == Path::new(".") {
        return None;
    }

    let rendered = relative.to_string_lossy().into_owned();
    if rendered.starts_with('.') {
        return Some(rendered);
    }

    Some(format!(".{MAIN_SEPARATOR}{rendered}"))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionConnectionStatus {
    Connected,
    NeedsReconnect,
}

impl fmt::Display for SessionConnectionStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Connected => write!(f, "connected"),
            Self::NeedsReconnect => write!(f, "needs reconnect"),
        }
    }
}

async fn resolve_session_connection_status(record: &SessionRecord) -> SessionConnectionStatus {
    let health = probe_queue_owner_health(&record.vwacp_record_id).await;
    if health.healthy {
        SessionConnectionStatus::Connected
    } else {
        SessionConnectionStatus::NeedsReconnect
    }
}

pub fn print_sessions_by_format(
    sessions: &[SessionRecord],
    format: OutputFormat,
) -> io::Result<()> {
    let stdout = io::stdout();
    let mut stdout = stdout.lock();
    write_sessions_by_format(&mut stdout, sessions, format)
}

pub(crate) fn write_sessions_by_format<W: Write>(
    writer: &mut W,
    sessions: &[SessionRecord],
    format: OutputFormat,
) -> io::Result<()> {
    if format == OutputFormat::Json {
        serde_json::to_writer(&mut *writer, sessions).map_err(io::Error::other)?;
        writer.write_all(b"\n")?;
        return Ok(());
    }

    if format == OutputFormat::Quiet {
        for session in sessions {
            let closed_marker = if session.closed.unwrap_or(false) { " [closed]" } else { "" };
            writeln!(writer, "{}{}", session.vwacp_record_id, closed_marker)?;
        }
        return Ok(());
    }

    if sessions.is_empty() {
        writeln!(writer, "No sessions")?;
        return Ok(());
    }

    for session in sessions {
        let closed_marker = if session.closed.unwrap_or(false) { " [closed]" } else { "" };
        writeln!(
            writer,
            "{}{}\t{}\t{}\t{}",
            session.vwacp_record_id,
            closed_marker,
            session.name.as_deref().unwrap_or("-"),
            session.cwd,
            session.last_used_at
        )?;
    }

    Ok(())
}

pub fn print_closed_session_by_format(
    record: &SessionRecord,
    format: OutputFormat,
) -> io::Result<()> {
    let stdout = io::stdout();
    let mut stdout = stdout.lock();
    write_closed_session_by_format(&mut stdout, record, format)
}

pub(crate) fn write_closed_session_by_format<W: Write>(
    writer: &mut W,
    record: &SessionRecord,
    format: OutputFormat,
) -> io::Result<()> {
    if emit_json_result(
        writer,
        format,
        &json!({
            "action": "session_closed",
            "vwacpRecordId": record.vwacp_record_id,
            "vwacpSessionId": record.acp_session_id,
            "agentSessionId": record.agent_session_id,
        }),
    )? {
        return Ok(());
    }

    if format == OutputFormat::Quiet {
        return Ok(());
    }

    writeln!(writer, "{}", record.vwacp_record_id)?;
    Ok(())
}

pub fn print_new_session_by_format(
    record: &SessionRecord,
    replaced: Option<&SessionRecord>,
    format: OutputFormat,
) -> io::Result<()> {
    let stdout = io::stdout();
    let mut stdout = stdout.lock();
    write_new_session_by_format(&mut stdout, record, replaced, format)
}

pub(crate) fn write_new_session_by_format<W: Write>(
    writer: &mut W,
    record: &SessionRecord,
    replaced: Option<&SessionRecord>,
    format: OutputFormat,
) -> io::Result<()> {
    if emit_json_result(
        writer,
        format,
        &json!({
            "action": "session_ensured",
            "created": true,
            "vwacpRecordId": record.vwacp_record_id,
            "vwacpSessionId": record.acp_session_id,
            "agentSessionId": record.agent_session_id,
            "name": record.name,
            "replacedSessionId": replaced.map(|session| session.vwacp_record_id.clone()),
        }),
    )? {
        return Ok(());
    }

    if format == OutputFormat::Quiet {
        writeln!(writer, "{}", record.vwacp_record_id)?;
        return Ok(());
    }

    if let Some(replaced) = replaced {
        writeln!(writer, "{}\t(replaced {})", record.vwacp_record_id, replaced.vwacp_record_id)?;
        return Ok(());
    }

    writeln!(writer, "{}", record.vwacp_record_id)?;
    Ok(())
}

pub fn print_ensured_session_by_format(
    record: &SessionRecord,
    created: bool,
    format: OutputFormat,
) -> io::Result<()> {
    let stdout = io::stdout();
    let mut stdout = stdout.lock();
    write_ensured_session_by_format(&mut stdout, record, created, format)
}

pub(crate) fn write_ensured_session_by_format<W: Write>(
    writer: &mut W,
    record: &SessionRecord,
    created: bool,
    format: OutputFormat,
) -> io::Result<()> {
    if emit_json_result(
        writer,
        format,
        &json!({
            "action": "session_ensured",
            "created": created,
            "vwacpRecordId": record.vwacp_record_id,
            "vwacpSessionId": record.acp_session_id,
            "agentSessionId": record.agent_session_id,
            "name": record.name,
        }),
    )? {
        return Ok(());
    }

    if format == OutputFormat::Quiet {
        writeln!(writer, "{}", record.vwacp_record_id)?;
        return Ok(());
    }

    let action = if created { "created" } else { "existing" };
    writeln!(writer, "{}\t({action})", record.vwacp_record_id)?;
    Ok(())
}

pub fn print_queued_prompt_by_format(
    result: &SessionEnqueueResult,
    format: OutputFormat,
) -> io::Result<()> {
    let stdout = io::stdout();
    let mut stdout = stdout.lock();
    write_queued_prompt_by_format(&mut stdout, result, format)
}

pub(crate) fn write_queued_prompt_by_format<W: Write>(
    writer: &mut W,
    result: &SessionEnqueueResult,
    format: OutputFormat,
) -> io::Result<()> {
    if emit_json_result(
        writer,
        format,
        &json!({
            "action": "prompt_queued",
            "vwacpRecordId": result.session_id,
            "requestId": result.request_id,
        }),
    )? {
        return Ok(());
    }

    if format == OutputFormat::Quiet {
        return Ok(());
    }

    writeln!(writer, "[queued] {}", result.request_id)?;
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConversationHistoryEntry {
    pub role: &'static str,
    pub timestamp: String,
    pub text_preview: String,
}

fn summarize_user_content(content: &SessionUserContent) -> Option<String> {
    match content {
        SessionUserContent::Text(text) => {
            let trimmed = text.trim();
            (!trimmed.is_empty()).then(|| trimmed.to_string())
        }
        SessionUserContent::Mention(mention) => {
            let trimmed = mention.content.trim();
            if trimmed.is_empty() { Some(mention.uri.clone()) } else { Some(trimmed.to_string()) }
        }
        SessionUserContent::Image(_) => Some("[image]".to_string()),
    }
}

fn summarize_agent_content(content: &SessionAgentContent) -> Option<String> {
    match content {
        SessionAgentContent::Text(text) => {
            let trimmed = text.trim();
            (!trimmed.is_empty()).then(|| trimmed.to_string())
        }
        SessionAgentContent::Thinking(_) => Some("[thinking]".to_string()),
        SessionAgentContent::RedactedThinking(_) => Some("[thinking redacted]".to_string()),
        SessionAgentContent::ToolUse(tool_use) => Some(format!("[tool:{}]", tool_use.name)),
    }
}

fn summarize_tool_result_content(content: &SessionToolResultContent) -> Option<String> {
    match content {
        SessionToolResultContent::Text(text) => {
            let trimmed = text.trim();
            (!trimmed.is_empty()).then(|| trimmed.to_string())
        }
        SessionToolResultContent::Image(_) => Some("[image]".to_string()),
    }
}

fn summarize_tool_result(result: &SessionToolResult) -> Option<String> {
    summarize_tool_result_content(&result.content).or_else(|| {
        result
            .result
            .as_ref()
            .and_then(|dto| dto.render_hint.as_ref())
            .and_then(|hint| hint.summary.as_deref())
            .map(str::trim)
            .filter(|summary| !summary.is_empty())
            .map(ToString::to_string)
    })
}

fn join_segments(segments: Vec<String>) -> Option<String> {
    let joined = segments.join("\n");
    let trimmed = joined.trim();
    (!trimmed.is_empty()).then(|| trimmed.to_string())
}

pub fn conversation_history_entries(record: &SessionRecord) -> Vec<ConversationHistoryEntry> {
    record
        .messages
        .iter()
        .filter_map(|message| match message {
            SessionMessage::User(message) => {
                join_segments(message.content.iter().filter_map(summarize_user_content).collect())
                    .map(|text_preview| ConversationHistoryEntry {
                        role: "user",
                        timestamp: record.updated_at.clone(),
                        text_preview,
                    })
            }
            SessionMessage::Agent(message) => {
                let mut segments =
                    message.content.iter().filter_map(summarize_agent_content).collect::<Vec<_>>();
                let mut tool_results = message.tool_results.iter().collect::<Vec<_>>();
                tool_results.sort_by_key(|(name, _)| *name);
                segments.extend(
                    tool_results
                        .into_iter()
                        .filter_map(|(_, result)| summarize_tool_result(result)),
                );
                join_segments(segments).map(|text_preview| ConversationHistoryEntry {
                    role: "assistant",
                    timestamp: record.updated_at.clone(),
                    text_preview,
                })
            }
            SessionMessage::Resume => None,
        })
        .collect()
}

fn write_field_line<W: Write>(writer: &mut W, label: &str, value: Option<&str>) -> io::Result<()> {
    writeln!(writer, "{label}: {}", value.unwrap_or("-"))
}

pub(crate) fn write_session_details_by_format<W: Write>(
    writer: &mut W,
    record: &SessionRecord,
    format: OutputFormat,
) -> io::Result<()> {
    if emit_json_result(
        writer,
        format,
        &json!({
            "action": "session_details",
            "session": record,
        }),
    )? {
        return Ok(());
    }

    if format == OutputFormat::Quiet {
        writeln!(writer, "{}", record.vwacp_record_id)?;
        return Ok(());
    }

    let vwacp = record.vwacp.as_ref();
    writeln!(writer, "session: {}", record.vwacp_record_id)?;
    write_field_line(writer, "acpSessionId", Some(&record.acp_session_id))?;
    write_field_line(writer, "agentSessionId", record.agent_session_id.as_deref())?;
    write_field_line(writer, "agent", Some(&record.agent_command))?;
    write_field_line(writer, "cwd", Some(&record.cwd))?;
    write_field_line(writer, "name", record.name.as_deref())?;
    write_field_line(writer, "title", record.title.as_deref())?;
    write_field_line(
        writer,
        "status",
        Some(if record.closed.unwrap_or(false) { "closed" } else { "open" }),
    )?;
    write_field_line(writer, "model", vwacp.and_then(|state| state.current_model_id.as_deref()))?;
    write_field_line(writer, "mode", vwacp.and_then(|state| state.current_mode_id.as_deref()))?;
    write_field_line(
        writer,
        "availableModels",
        vwacp
            .and_then(|state| state.available_models.as_ref())
            .map(|models| models.join(", "))
            .as_deref(),
    )?;
    write_field_line(writer, "createdAt", Some(&record.created_at))?;
    write_field_line(writer, "lastUsedAt", Some(&record.last_used_at))?;
    write_field_line(writer, "updatedAt", Some(&record.updated_at))?;
    write_field_line(writer, "lastPromptAt", record.last_prompt_at.as_deref())?;
    write_field_line(writer, "pid", record.pid.map(|value| value.to_string()).as_deref())?;
    write_field_line(writer, "closedAt", record.closed_at.as_deref())?;
    write_field_line(
        writer,
        "lastAgentExitCode",
        record.last_agent_exit_code.map(|value| value.to_string()).as_deref(),
    )?;
    write_field_line(writer, "lastAgentExitSignal", record.last_agent_exit_signal.as_deref())?;
    write_field_line(
        writer,
        "lastAgentDisconnectReason",
        record.last_agent_disconnect_reason.as_deref(),
    )?;
    Ok(())
}

pub fn print_session_details_by_format(
    record: &SessionRecord,
    format: OutputFormat,
) -> io::Result<()> {
    let stdout = io::stdout();
    let mut stdout = stdout.lock();
    write_session_details_by_format(&mut stdout, record, format)
}

pub(crate) fn write_session_history_by_format<W: Write>(
    writer: &mut W,
    record: &SessionRecord,
    format: OutputFormat,
    limit: usize,
) -> io::Result<()> {
    let entries = conversation_history_entries(record);
    let start = entries.len().saturating_sub(limit);
    let visible_entries = &entries[start..];

    if emit_json_result(
        writer,
        format,
        &json!({
            "action": "session_history",
            "vwacpRecordId": record.vwacp_record_id,
            "acpSessionId": record.acp_session_id,
            "entries": visible_entries,
        }),
    )? {
        return Ok(());
    }

    if format == OutputFormat::Quiet {
        for entry in visible_entries {
            writeln!(writer, "{}", entry.text_preview)?;
        }
        return Ok(());
    }

    if visible_entries.is_empty() {
        writeln!(writer, "No conversation history")?;
        return Ok(());
    }

    for entry in visible_entries {
        writeln!(writer, "[{}] {}", entry.role, entry.text_preview)?;
    }
    Ok(())
}

pub fn print_session_history_by_format(
    record: &SessionRecord,
    format: OutputFormat,
    limit: usize,
) -> io::Result<()> {
    let stdout = io::stdout();
    let mut stdout = stdout.lock();
    write_session_history_by_format(&mut stdout, record, format, limit)
}

pub fn format_prompt_session_banner_line(
    record: &SessionRecord,
    current_cwd: &str,
    connection_status: SessionConnectionStatus,
) -> String {
    let label = format_session_label(record);
    let normalized_session_cwd = resolve_display_path(&record.cwd);
    let normalized_current_cwd = resolve_display_path(current_cwd);
    let routed_from = if normalized_session_cwd == normalized_current_cwd {
        None
    } else {
        format_routed_from(&normalized_session_cwd, &normalized_current_cwd)
    };

    if let Some(routed_from) = routed_from {
        return format!(
            "[vwacp] session {label} ({}) · {} (routed from {routed_from}) · agent {connection_status}",
            record.vwacp_record_id,
            normalized_session_cwd.display()
        );
    }

    format!(
        "[vwacp] session {label} ({}) · {} · agent {connection_status}",
        record.vwacp_record_id,
        normalized_session_cwd.display()
    )
}

pub async fn print_prompt_session_banner(
    record: &SessionRecord,
    current_cwd: &str,
    format: OutputFormat,
    json_strict: bool,
) -> io::Result<()> {
    if format == OutputFormat::Quiet || (json_strict && format == OutputFormat::Json) {
        return Ok(());
    }

    let status = resolve_session_connection_status(record).await;
    let stderr = io::stderr();
    let mut stderr = stderr.lock();
    writeln!(stderr, "{}", format_prompt_session_banner_line(record, current_cwd, status))?;
    Ok(())
}

pub fn print_created_session_banner(
    record: &SessionRecord,
    agent_name: &str,
    format: OutputFormat,
    json_strict: bool,
) -> io::Result<()> {
    if format == OutputFormat::Quiet || (json_strict && format == OutputFormat::Json) {
        return Ok(());
    }

    let stderr = io::stderr();
    let mut stderr = stderr.lock();
    write_created_session_banner(&mut stderr, record, agent_name)
}

pub(crate) fn write_created_session_banner<W: Write>(
    writer: &mut W,
    record: &SessionRecord,
    agent_name: &str,
) -> io::Result<()> {
    let label = format_session_label(record);
    writeln!(writer, "[vwacp] created session {label} ({})", record.vwacp_record_id)?;
    writeln!(writer, "[vwacp] agent: {agent_name}")?;
    writeln!(writer, "[vwacp] cwd: {}", record.cwd)?;
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentSessionIdPayload {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent_session_id: Option<String>,
}

pub fn agent_session_id_payload(agent_session_id: Option<&str>) -> AgentSessionIdPayload {
    let normalized = agent_session_id
        .map(|value| Value::String(value.to_string()))
        .as_ref()
        .and_then(normalize_runtime_session_id);

    AgentSessionIdPayload { agent_session_id: normalized }
}

pub fn print_cancel_result_by_format(result: bool, format: OutputFormat) -> io::Result<()> {
    let stdout = io::stdout();
    let mut stdout = stdout.lock();
    write_cancel_result_by_format(&mut stdout, result, format)
}

pub(crate) fn write_cancel_result_by_format<W: Write>(
    writer: &mut W,
    result: bool,
    format: OutputFormat,
) -> io::Result<()> {
    if emit_json_result(
        writer,
        format,
        &json!({
            "action": "cancel",
            "cancelled": result,
        }),
    )? {
        return Ok(());
    }

    if format == OutputFormat::Quiet {
        return Ok(());
    }

    writeln!(writer, "{}", if result { "Cancelled" } else { "Not cancelled" })?;
    Ok(())
}

pub fn print_set_mode_result_by_format(mode_id: &str, format: OutputFormat) -> io::Result<()> {
    let stdout = io::stdout();
    let mut stdout = stdout.lock();
    write_set_mode_result_by_format(&mut stdout, mode_id, format)
}

pub(crate) fn write_set_mode_result_by_format<W: Write>(
    writer: &mut W,
    mode_id: &str,
    format: OutputFormat,
) -> io::Result<()> {
    if emit_json_result(
        writer,
        format,
        &json!({
            "action": "set_mode",
            "mode": mode_id,
        }),
    )? {
        return Ok(());
    }

    if format == OutputFormat::Quiet {
        return Ok(());
    }

    writeln!(writer, "Mode set to {mode_id}")?;
    Ok(())
}

pub fn print_set_model_result_by_format(model_id: &str, format: OutputFormat) -> io::Result<()> {
    let stdout = io::stdout();
    let mut stdout = stdout.lock();
    write_set_model_result_by_format(&mut stdout, model_id, format)
}

pub(crate) fn write_set_model_result_by_format<W: Write>(
    writer: &mut W,
    model_id: &str,
    format: OutputFormat,
) -> io::Result<()> {
    if emit_json_result(
        writer,
        format,
        &json!({
            "action": "set_model",
            "model": model_id,
        }),
    )? {
        return Ok(());
    }

    if format == OutputFormat::Quiet {
        return Ok(());
    }

    writeln!(writer, "Model set to {model_id}")?;
    Ok(())
}

pub fn print_set_config_option_result_by_format(
    config_id: &str,
    value: &str,
    format: OutputFormat,
) -> io::Result<()> {
    let stdout = io::stdout();
    let mut stdout = stdout.lock();
    write_set_config_option_result_by_format(&mut stdout, config_id, value, format)
}

pub(crate) fn write_set_config_option_result_by_format<W: Write>(
    writer: &mut W,
    config_id: &str,
    value: &str,
    format: OutputFormat,
) -> io::Result<()> {
    if emit_json_result(
        writer,
        format,
        &json!({
            "action": "set_config_option",
            "config": config_id,
            "value": value,
        }),
    )? {
        return Ok(());
    }

    if format == OutputFormat::Quiet {
        return Ok(());
    }

    writeln!(writer, "Config option {config_id} set to {value}")?;
    Ok(())
}
