//! Grok Build active and archived history on the selected execution host.
use std::path::{Path, PathBuf};
use serde::Deserialize;
use serde_json::Value;
use vw_api_types::external_session::*;
use crate::sessions::blocks::{openai_tool_calls, tool_result_block};
use crate::sessions::utils::{extract_text, truncate_summary};
use super::pi_jsonl::{for_each_jsonl_value, parse_timestamp_to_ms};
#[derive(Debug, Deserialize)]
struct GrokSessionInfo {
    id: String,
    #[serde(default)]
    cwd: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GrokSessionSummary {
    info: GrokSessionInfo,
    #[serde(default)]
    session_summary: Option<String>,
    #[serde(default)]
    generated_title: Option<String>,
    #[serde(default)]
    created_at: Option<Value>,
    #[serde(default)]
    updated_at: Option<Value>,
    #[serde(default)]
    last_active_at: Option<Value>,
}

/// The trusted application resolver supplies this instance's absolute configuration root.
pub fn session_roots(config_dir: &Path) -> Vec<PathBuf> {
    vec![config_dir.join("sessions"), config_dir.join("archived_sessions")]
}

pub fn scan_sessions(config_dir: &Path, identity: &ExternalSessionIdentity) -> Result<Vec<SessionMeta>, String> {
    if !config_dir.is_absolute() || identity.provider != SessionProvider::Grokbuild {
        return Err("Expected an absolute Grok Build instance root".into());
    }
    let mut paths = Vec::new();
    for root in session_roots(config_dir) { collect_summary_files(&root, 0, &mut paths)?; }
    paths.sort();
    let mut result = Vec::new();
    for path in paths {
        let summary = read_summary(&path)?;
        let id = summary.info.id;
        if id.trim().is_empty() { continue; }
        let title = summary.generated_title.as_deref().filter(|v| !v.trim().is_empty())
            .or(summary.session_summary.as_deref().filter(|v| !v.trim().is_empty()))
            .map(|v| truncate_summary(v, 80));
        let created_at = summary.created_at.as_ref().and_then(parse_timestamp_to_ms);
        let last_active_at = summary.last_active_at.as_ref().or(summary.updated_at.as_ref()).and_then(parse_timestamp_to_ms);
        let mut bound = identity.clone();
        bound.session_id = ExternalSessionId(id);
        bound.source = SessionSource::File { relative_path: path.strip_prefix(config_dir)
            .map_err(|_| "Grok Build source escaped instance root")?.to_string_lossy().into_owned() };
        result.push(SessionMeta {
            identity: bound,
            capabilities: SessionCapabilities { read: true, delete: true, resume: true, export_markdown: true },
            title, summary: summary.session_summary.as_deref().filter(|v| !v.trim().is_empty()).map(|v| truncate_summary(v, 160)),
            project_dir: summary.info.cwd, created_at, last_active_at,
            source_path: Some(path.to_string_lossy().into_owned()),
        });
    }
    Ok(result)
}

fn collect_summary_files(root: &Path, depth: usize, files: &mut Vec<PathBuf>) -> Result<(), String> {
    let meta = match std::fs::symlink_metadata(root) {
        Ok(meta) => meta,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(_) => return Err("Unable to inspect Grok Build sessions".into()),
    };
    if meta.file_type().is_symlink() { return Ok(()); }
    if !meta.is_dir() { return Err("Invalid Grok Build sessions directory".into()); }
    if depth > 32 || files.len() >= 100_000 { return Err("Grok Build session discovery limit exceeded".into()); }
    for entry in std::fs::read_dir(root).map_err(|_| "Unable to list Grok Build sessions")? {
        if files.len() >= 100_000 { return Err("Grok Build session discovery limit exceeded".into()); }
        let entry = entry.map_err(|_| "Unable to inspect Grok Build session")?;
        let kind = entry.file_type().map_err(|_| "Unable to inspect Grok Build session type")?;
        if kind.is_dir() { collect_summary_files(&entry.path(), depth + 1, files)?; }
        else if kind.is_file() && entry.file_name() == "summary.json" { files.push(entry.path()); }
    }
    Ok(())
}

fn read_summary(path: &Path) -> Result<GrokSessionSummary, String> {
    if std::fs::metadata(path).map_err(|_| "Unable to inspect Grok Build summary")?.len() > 4 * 1024 * 1024 {
        return Err("Grok Build summary exceeds size limit".into());
    }
    let text = std::fs::read_to_string(path).map_err(|_| "Unable to read Grok Build summary")?;
    serde_json::from_str(&text).map_err(|_| "Invalid Grok Build summary".into())
}

/// `chat_history.jsonl`：`type ∈ {system, user, assistant, tool}`。
///
/// 会话的 sourcePath 是同目录的 `summary.json`；大内容的 Jsonl 引用指向 chat_history.jsonl
/// 的行（`content::resolve_content_ref` 对 grokbuild 固定改读该文件）。`tool` 记录的配对字段与
/// assistant 的 `tool_calls` 都是按 OpenAI 形状推断（待核实），缺失时 `callId` 为空串。
pub fn load_messages(path: &Path) -> Result<Vec<SessionMessage>, String> {
    if path.file_name().and_then(|v| v.to_str()) != Some("summary.json") {
        return Err("Unexpected Grok Build session source".into());
    }
    let session_dir = path
        .parent()
        .ok_or_else(|| format!("Invalid Grok Build session path: {}", path.display()))?;
    let chat_path = session_dir.join("chat_history.jsonl");
    if !std::fs::symlink_metadata(&chat_path).map(|m| m.is_file() && !m.file_type().is_symlink()).unwrap_or(false) {
        return Err(format!(
            "Failed to open Grok Build chat history: {}",
            chat_path.display()
        ));
    }
    let mut messages = Vec::new();

    for_each_jsonl_value(&chat_path, |span, value| {
        let kind = value.get("type").and_then(Value::as_str).unwrap_or("");
        let role = match kind {
            "system" | "user" | "assistant" | "tool" => kind,
            // Reasoning records can contain encrypted/internal state and are not
            // conversation messages shown by Grok's own history view.
            _ => return Ok(()),
        };
        let text = value.get("content").map(extract_text).unwrap_or_default();
        let blocks = match role {
            "tool" => vec![tool_result_block(
                value
                    .get("tool_call_id")
                    .or_else(|| value.get("call_id"))
                    .and_then(Value::as_str)
                    .unwrap_or_default(),
                ToolStatus::Unknown,
                &text,
                || Some(span.content_ref("/content")),
            )],
            "assistant" => {
                let mut blocks = Vec::new();
                if !text.trim().is_empty() {
                    blocks.push(SessionBlock::text(text));
                }
                blocks.extend(openai_tool_calls(value.get("tool_calls"), |pointer| {
                    Some(span.content_ref(format!("/tool_calls{pointer}")))
                }));
                blocks
            }
            _ if text.trim().is_empty() => Vec::new(),
            _ => vec![SessionBlock::text(text)],
        };
        let ts = value
            .get("timestamp")
            .or_else(|| value.get("ts"))
            .and_then(parse_timestamp_to_ms);
        let mut message = SessionMessage::from_blocks(role, ts, blocks);
        if message.is_empty() {
            return Ok(());
        }
        // system prompt 属于注入内容，默认隐藏
        message.injected = role == "system";
        messages.push(message);
        Ok(())
    })?;

    assign_turn_ids(&mut messages);
    Ok(messages)
}

fn assign_turn_ids(messages: &mut [SessionMessage]) {
    let mut turn = 0u32;
    for message in messages {
        if message.role == "user" && !message.injected && message.blocks.iter().any(|block| matches!(block, SessionBlock::Text { .. })) {
            turn = turn.saturating_add(1);
        }
        message.turn_id = Some(format!("t{turn}"));
    }
}
