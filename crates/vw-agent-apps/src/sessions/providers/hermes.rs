//! Hermes JSONL transcripts under the selected execution-host application root.
use std::fs::File;
use std::io::{self, BufRead, BufReader, Seek, SeekFrom};
use std::path::Path;
use serde_json::Value;
use vw_api_types::external_session::{ContentRef, ExternalSessionId, ExternalSessionIdentity, SessionBlock, SessionCapabilities, SessionMessage, SessionMeta, SessionProvider, SessionSource, ToolStatus};
use crate::sessions::blocks::{openai_tool_calls, tool_result_block};
use crate::sessions::utils::{extract_text, truncate_summary};
const TITLE_MAX_CHARS: usize = 80;

/// Roots are resolved by the trusted execution host, never from sourcePath display data.
pub fn scan_sessions(config_dir: &Path, identity: &ExternalSessionIdentity) -> io::Result<Vec<SessionMeta>> {
    if !config_dir.is_absolute() || identity.provider != SessionProvider::Hermes {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "expected absolute Hermes instance root"));
    }
    let root = config_dir.join("sessions");
    let metadata = match std::fs::symlink_metadata(&root) {
        Ok(meta) => meta,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(error),
    };
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "invalid Hermes sessions directory"));
    }
    let mut paths = Vec::new();
    for entry in std::fs::read_dir(root)? {
        let entry = entry?;
        if entry.file_type()?.is_file() && matches!(entry.path().extension().and_then(|v| v.to_str()), Some("jsonl" | "json")) {
            paths.push(entry.path());
        }
    }
    paths.sort();
    let mut sessions = Vec::new();
    for path in paths {
        let mut bound = identity.clone();
        bound.source = SessionSource::File { relative_path: path.strip_prefix(config_dir).map_err(io::Error::other)?.to_string_lossy().into_owned() };
        if let Some(meta) = parse_jsonl_session(&path, &bound) { sessions.push(meta); }
    }
    Ok(sessions)
}

/// 一行 JSONL 里的消息角色和非空正文（扁平和 `{type:"message", message:{…}}` 两种格式）。
fn jsonl_message(value: &Value) -> Option<(&str, String)> {
    let role = value
        .get("role")
        .or_else(|| value.get("message").and_then(|m| m.get("role")))
        .and_then(Value::as_str)?;
    let content = value
        .get("content")
        .or_else(|| value.get("message").and_then(|m| m.get("content")))?;
    let text = extract_text(content);
    if text.trim().is_empty() {
        return None;
    }
    Some((role, text))
}

pub(crate) fn parse_jsonl_session(path: &Path, identity: &ExternalSessionIdentity) -> Option<SessionMeta> {
    // Read head (metadata + first user message) and tail (last timestamp + last message)
    let (head, tail) = read_head_tail_lines(path, 30, 10).ok()?;

    let mut first_user_msg: Option<String> = None;
    let mut first_ts: Option<i64> = None;
    let mut last_ts: Option<i64> = None;
    let mut session_id: Option<String> = None;
    let mut title: Option<String> = None;
    let mut cwd: Option<String> = None;

    // Process head lines for metadata and first user message
    for line in &head {
        if line.trim().is_empty() {
            continue;
        }
        let value: Value = match serde_json::from_str(line) {
            Ok(v) => v,
            Err(_) => continue,
        };

        let ts = value
            .get("timestamp")
            .or_else(|| value.get("ts"))
            .and_then(parse_timestamp_to_ms);

        if first_ts.is_none() {
            first_ts = ts;
        }
        last_ts = ts.or(last_ts);

        let line_type = value.get("type").and_then(Value::as_str).unwrap_or("");

        // Extract session metadata from session-type lines
        if line_type == "session" || line_type == "init" {
            if session_id.is_none() {
                session_id = value
                    .get("id")
                    .or_else(|| value.get("sessionId"))
                    .and_then(Value::as_str)
                    .map(|s| s.to_string());
            }
            if title.is_none() {
                title = value
                    .get("title")
                    .and_then(Value::as_str)
                    .filter(|s| !s.is_empty())
                    .map(|s| s.to_string());
            }
            if cwd.is_none() {
                cwd = value
                    .get("cwd")
                    .or_else(|| value.get("directory"))
                    .and_then(Value::as_str)
                    .filter(|s| !s.is_empty())
                    .map(|s| s.to_string());
            }
        }

        if first_user_msg.is_none() {
            if let Some(("user", text)) = jsonl_message(&value) {
                first_user_msg = Some(truncate_summary(&text, TITLE_MAX_CHARS));
            }
        }
    }

    // Process tail lines for the most recent timestamp and the last displayable message:
    // the summary is the last user/assistant message, same as the SQLite scan.
    let mut tail_ts: Option<i64> = None;
    let mut last_msg: Option<String> = None;
    for line in tail.iter().rev() {
        if line.trim().is_empty() {
            continue;
        }
        let value: Value = match serde_json::from_str(line) {
            Ok(v) => v,
            Err(_) => continue,
        };
        if tail_ts.is_none() {
            tail_ts = value
                .get("timestamp")
                .or_else(|| value.get("ts"))
                .and_then(parse_timestamp_to_ms);
        }
        if last_msg.is_none() {
            if let Some(("user" | "assistant", text)) = jsonl_message(&value) {
                last_msg = Some(truncate_summary(&text, 160));
            }
        }
        if tail_ts.is_some() && last_msg.is_some() {
            break;
        }
    }
    let last_ts = tail_ts.or(last_ts);

    // Fall back to filename as session ID
    let session_id = session_id.unwrap_or_else(|| {
        path.file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown")
            .to_string()
    });

    let source_path = path.to_string_lossy().to_string();

    Some(SessionMeta {
        identity: ExternalSessionIdentity { session_id: ExternalSessionId(session_id), ..identity.clone() },
        capabilities: SessionCapabilities { read: true, delete: true, resume: false, export_markdown: true },
        title: title.or_else(|| first_user_msg.clone()),
        summary: last_msg.or(first_user_msg),
        project_dir: cwd,
        created_at: first_ts,
        last_active_at: last_ts.or(first_ts),
        source_path: Some(source_path),
    })
}

/// Load messages from a Hermes JSONL transcript file.
///
/// OpenAI-shaped tool fields follow the pinned upstream reader. The caller must
/// resolve this path under its authorized instance root before invoking the parser.
pub fn load_messages(path: &Path) -> Result<Vec<SessionMessage>, String> {
    let mut messages = Vec::new();

    for_each_jsonl_value(path, |span, value| {
        // Support both flat messages and nested {type:"message", message:{...}} format
        let (msg, base, ts_val) = if value.get("type").and_then(Value::as_str) == Some("message") {
            let Some(msg) = value.get("message") else {
                return Ok(());
            };
            (
                msg,
                "/message",
                value.get("timestamp").or_else(|| msg.get("ts")),
            )
        } else {
            (
                &value,
                "",
                value.get("timestamp").or_else(|| value.get("ts")),
            )
        };

        let Some(role) = msg.get("role").and_then(Value::as_str) else {
            return Ok(());
        };
        let content = msg.get("content");
        let text = content.map(extract_text).unwrap_or_default();
        let content_ref = || {
            content
                .filter(|c| c.is_string())
                .map(|_| span.content_ref(format!("{base}/content")))
        };

        let blocks = match role {
            "tool" => vec![tool_result_block(
                msg.get("tool_call_id")
                    .and_then(Value::as_str)
                    .unwrap_or_default(),
                ToolStatus::Unknown,
                &text,
                content_ref,
            )],
            "assistant" => {
                let mut blocks = Vec::new();
                if !text.trim().is_empty() {
                    blocks.push(SessionBlock::text(text));
                }
                blocks.extend(openai_tool_calls(msg.get("tool_calls"), |pointer| {
                    Some(span.content_ref(format!("{base}/tool_calls{pointer}")))
                }));
                blocks
            }
            _ if text.trim().is_empty() => Vec::new(),
            _ => vec![SessionBlock::text(text)],
        };

        let ts = ts_val.and_then(parse_timestamp_to_ms);
        let message = SessionMessage::from_blocks(role, ts, blocks);
        if !message.is_empty() {
            messages.push(message);
        }
        Ok(())
    })?;

    assign_turn_ids(&mut messages);
    Ok(messages)
}


fn read_head_tail_lines(
    path: &Path,
    head_n: usize,
    tail_n: usize,
) -> io::Result<(Vec<String>, Vec<String>)> {
    let file = File::open(path)?;
    let file_len = file.metadata()?.len();

    // For small files, read all lines once and split
    if file_len < 16_384 {
        let reader = BufReader::new(file);
        let all: Vec<String> = reader.lines().map_while(Result::ok).collect();
        let head = all.iter().take(head_n).cloned().collect();
        let skip = all.len().saturating_sub(tail_n);
        let tail = all.into_iter().skip(skip).collect();
        return Ok((head, tail));
    }

    // Read head lines from the beginning
    let reader = BufReader::new(file);
    let head: Vec<String> = reader.lines().take(head_n).map_while(Result::ok).collect();

    // Seek to last ~16 KB for tail lines
    let seek_pos = file_len.saturating_sub(16_384);
    let mut file2 = File::open(path)?;
    file2.seek(SeekFrom::Start(seek_pos))?;
    let tail_reader = BufReader::new(file2);
    let all_tail: Vec<String> = tail_reader.lines().map_while(Result::ok).collect();

    // Skip first partial line if we seeked into the middle of a line
    let skip_first = if seek_pos > 0 { 1 } else { 0 };
    let usable: Vec<String> = all_tail.into_iter().skip(skip_first).collect();
    let skip = usable.len().saturating_sub(tail_n);
    let tail = usable.into_iter().skip(skip).collect();

    Ok((head, tail))
}

fn parse_timestamp_to_ms(value: &Value) -> Option<i64> {
    // Integer: milliseconds (>1e12) or seconds
    if let Some(n) = value.as_i64() {
        return if n > 1_000_000_000_000 { Some(n) } else { n.checked_mul(1000) };
    }
    if let Some(n) = value.as_f64() {
        let n = n as i64;
        return if n > 1_000_000_000_000 { Some(n) } else { n.checked_mul(1000) };
    }
    // RFC3339 string
    let raw = value.as_str()?;
    chrono::DateTime::parse_from_rfc3339(raw)
        .ok()
        .map(|dt: chrono::DateTime<chrono::FixedOffset>| dt.timestamp_millis())
}

struct JsonlSpan { offset: u64, len: u32 }
impl JsonlSpan {
    fn content_ref(&self, pointer: String) -> ContentRef {
        ContentRef::Jsonl { offset: self.offset, len: self.len, pointer }
    }
}
fn for_each_jsonl_value(path: &Path, mut visit: impl FnMut(JsonlSpan, Value) -> Result<(), String>) -> Result<(), String> {
    let file = File::open(path).map_err(|_| "Failed to open Hermes transcript".to_string())?;
    let mut reader = BufReader::new(file);
    let mut offset = 0u64;
    let mut bytes = Vec::new();
    loop {
        bytes.clear();
        let count = reader.read_until(b'\n', &mut bytes).map_err(|_| "Failed to read Hermes transcript".to_string())?;
        if count == 0 { break; }
        let len = u32::try_from(count).map_err(|_| "Hermes transcript record too large".to_string())?;
        let span = JsonlSpan { offset, len };
        offset = offset.checked_add(count as u64).ok_or("Hermes transcript too large")?;
        if let Ok(value) = serde_json::from_slice(bytes.trim_ascii()) { visit(span, value)?; }
    }
    Ok(())
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
