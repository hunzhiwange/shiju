//! OpenClaw gateway history. Roots are supplied by the authorized execution host.
use std::collections::HashMap;
use std::fs::File;
use std::io::{self, BufRead, BufReader, Seek, SeekFrom};
use std::path::{Component, Path};
use serde_json::Value;
use vw_api_types::external_session::*;
use super::pi_blocks::PiTranscript;
use super::pi_jsonl::{for_each_jsonl_value, parse_timestamp_to_ms};
use crate::sessions::utils::{extract_text, truncate_summary};
const TITLE_MAX_CHARS: usize = 80;

fn strip_message_id_suffix(text: &str) -> &str {
    match text.rfind("\n[message_id:") { Some(pos) => text[..pos].trim_end(), None => text }
}

/// Agent identity remains in the relative source locator, even when session IDs coincide.
pub fn scan_sessions(root: &Path, identity: &ExternalSessionIdentity) -> io::Result<Vec<SessionMeta>> {
    if identity.provider != SessionProvider::Openclaw { return Err(io::Error::other("expected OpenClaw identity")); }
    checked(root, root)?;
    let agents = root.join("agents");
    if !agents.try_exists()? { return Ok(Vec::new()); }
    checked(root, &agents)?;
    let mut sessions = Vec::new();
    for agent in std::fs::read_dir(&agents)? {
        let agent = agent?;
        if !agent.file_type()?.is_dir() { continue; }
        let dir = agent.path().join("sessions");
        if !dir.try_exists()? { continue; }
        checked(root, &dir)?;
        let names = load_display_names(&dir);
        for entry in std::fs::read_dir(&dir)? {
            let entry = entry?;
            if !entry.file_type()?.is_file() || entry.path().extension().and_then(|e| e.to_str()) != Some("jsonl") { continue; }
            let path = entry.path();
            checked(root, &path)?;
            let mut bound = identity.clone();
            bound.source = SessionSource::File { relative_path: path.strip_prefix(root).map_err(io::Error::other)?.to_string_lossy().into_owned() };
            if let Some(meta) = parse_session(&path, Some(&names), &bound) { sessions.push(meta); }
        }
    }
    sessions.sort_by(|a,b| b.last_active_at.cmp(&a.last_active_at).then_with(|| a.source_path.cmp(&b.source_path)));
    Ok(sessions)
}

/// Read every record in file order; OpenClaw has no Pi active-branch filtering.
pub fn load_messages(root: &Path, path: &Path) -> Result<Vec<SessionMessage>, String> {
    checked(root, path).map_err(|_| "invalid OpenClaw transcript path".to_string())?;
    let relative = path.strip_prefix(root).map_err(|_| "invalid OpenClaw transcript path")?;
    let parts: Vec<_> = relative.components().collect();
    if parts.len() != 4 || parts[0].as_os_str() != "agents" || parts[2].as_os_str() != "sessions" || path.extension().and_then(|e| e.to_str()) != Some("jsonl") {
        return Err("expected OpenClaw agent transcript".into());
    }
    let mut transcript = PiTranscript::new();
    for_each_jsonl_value(path, |span, value| {
        let id = value.get("id").and_then(Value::as_str).map(str::to_owned);
        transcript.push_entry(&value, span, id);
        Ok(())
    })?;
    Ok(transcript.finish())
}

fn checked(root: &Path, path: &Path) -> io::Result<()> {
    if !root.is_absolute() || !path.starts_with(root) || path.components().any(|c| matches!(c, Component::ParentDir)) {
        return Err(io::Error::other("invalid OpenClaw root or path"));
    }
    let mut current = std::path::PathBuf::new();
    for component in path.components() {
        current.push(component);
        if std::fs::symlink_metadata(&current)?.file_type().is_symlink() { return Err(io::Error::other("OpenClaw symlinks are not allowed")); }
    }
    Ok(())
}

/// Read `sessions.json` index and build a sessionId → displayName lookup map.
/// Returns an empty map if the file does not exist or cannot be parsed.
fn load_display_names(sessions_dir: &Path) -> HashMap<String, String> {
    let index_path = sessions_dir.join("sessions.json");
    if std::fs::symlink_metadata(&index_path).map(|m| m.file_type().is_symlink()).unwrap_or(false) { return HashMap::new(); }
    let content = match std::fs::read_to_string(&index_path) {
        Ok(c) => c,
        Err(_) => return HashMap::new(),
    };
    let index: serde_json::Map<String, Value> = match serde_json::from_str(&content) {
        Ok(m) => m,
        Err(_) => return HashMap::new(),
    };

    let mut map = HashMap::new();
    for (_key, entry) in &index {
        if let (Some(id), Some(name)) = (
            entry.get("sessionId").and_then(Value::as_str),
            entry.get("displayName").and_then(Value::as_str),
        ) {
            if !name.is_empty() {
                map.insert(id.to_string(), name.to_string());
            }
        }
    }
    map
}

pub(crate) fn parse_session(
    path: &Path,
    display_names: Option<&HashMap<String, String>>,
    identity: &ExternalSessionIdentity,
) -> Option<SessionMeta> {
    let (head, tail) = read_head_tail_lines(path, 10, 30).ok()?;

    let mut session_id: Option<String> = None;
    let mut cwd: Option<String> = None;
    let mut created_at: Option<i64> = None;
    let mut summary: Option<String> = None;
    let mut first_user_message: Option<String> = None;

    // Extract metadata, summary, and first user message from head lines
    for line in &head {
        let value: Value = match serde_json::from_str(line) {
            Ok(parsed) => parsed,
            Err(_) => continue,
        };

        if created_at.is_none() {
            created_at = value.get("timestamp").and_then(parse_timestamp_to_ms);
        }

        let event_type = value.get("type").and_then(Value::as_str).unwrap_or("");

        if event_type == "session" {
            if session_id.is_none() {
                session_id = value
                    .get("id")
                    .and_then(Value::as_str)
                    .map(|s| s.to_string());
            }
            if cwd.is_none() {
                cwd = value
                    .get("cwd")
                    .and_then(Value::as_str)
                    .map(|s| s.to_string());
            }
            if let Some(ts) = value.get("timestamp").and_then(parse_timestamp_to_ms) {
                created_at.get_or_insert(ts);
            }
            continue;
        }

        if event_type == "message" {
            if let Some(message) = value.get("message") {
                let text = message.get("content").map(extract_text).unwrap_or_default();
                let cleaned = strip_message_id_suffix(&text);
                if !cleaned.trim().is_empty() {
                    if first_user_message.is_none()
                        && message.get("role").and_then(Value::as_str) == Some("user")
                    {
                        first_user_message = Some(cleaned.trim().to_string());
                    }
                    if summary.is_none() {
                        summary = Some(cleaned.trim().to_string());
                    }
                }
            }
        }

        if session_id.is_some()
            && cwd.is_some()
            && created_at.is_some()
            && summary.is_some()
            && first_user_message.is_some()
        {
            break;
        }
    }

    // Extract last_active_at from tail lines (reverse order)
    let mut last_active_at: Option<i64> = None;
    for line in tail.iter().rev() {
        let value: Value = match serde_json::from_str(line) {
            Ok(parsed) => parsed,
            Err(_) => continue,
        };
        if let Some(ts) = value.get("timestamp").and_then(parse_timestamp_to_ms) {
            last_active_at = Some(ts);
            break;
        }
    }

    // Fall back to filename as session ID
    let session_id = session_id.or_else(|| {
        path.file_stem()
            .and_then(|s| s.to_str())
            .map(|s| s.to_string())
    });
    let session_id = session_id?;

    // Title priority: displayName (from sessions.json) > first user message > dir basename
    let title = display_names
        .and_then(|m| m.get(&session_id))
        .filter(|s| !s.is_empty())
        .map(|t| truncate_summary(t, TITLE_MAX_CHARS))
        .or_else(|| first_user_message.map(|t| truncate_summary(&t, TITLE_MAX_CHARS)))
        .or_else(|| {
            cwd.as_deref()
                .and_then(path_basename)
                .map(|s| s.to_string())
        });

    let summary = summary.map(|text| truncate_summary(&text, 160));

    Some(SessionMeta {
        identity: ExternalSessionIdentity { session_id: ExternalSessionId(session_id.clone()), ..identity.clone() },
        capabilities: SessionCapabilities { read: true, delete: true, resume: false, export_markdown: true },
        title,
        summary,
        project_dir: cwd,
        created_at,
        last_active_at,
        source_path: Some(path.to_string_lossy().to_string()),
    })
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

fn path_basename(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return None;
    }
    let normalized = trimmed.trim_end_matches(['/', '\\']);
    let last = normalized
        .split(['/', '\\'])
        .next_back()
        .filter(|segment| !segment.is_empty())?;
    Some(last.to_string())
}
