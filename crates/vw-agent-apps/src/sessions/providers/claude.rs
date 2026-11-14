//! Claude Code discovery under a root resolved by the execution host.
use std::fs::File;
use std::io::{self, BufRead, BufReader, Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use serde_json::Value;
use vw_api_types::external_session::{ExternalSessionId, ExternalSessionIdentity, SessionCapabilities, SessionMeta, SessionProvider, SessionSource};
use crate::sessions::utils::{extract_text, truncate_summary};
const TITLE_MAX_CHARS: usize = 80;

/// `config_dir` must belong to the selected native application instance on this host.
/// No environment or home directory fallback is used here.
pub fn scan_sessions(config_dir: &Path, identity: &ExternalSessionIdentity) -> io::Result<Vec<SessionMeta>> {
    if !config_dir.is_absolute() || identity.provider != SessionProvider::Claude {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "expected an absolute Claude instance root"));
    }
    let root = config_dir.join("projects");
    let mut files = Vec::new();
    collect_jsonl_files(&root, &mut files)?;
    files.sort();
    let mut sessions = Vec::new();
    for path in files {
        let relative = path.strip_prefix(config_dir).map_err(io::Error::other)?;
        let mut bound = identity.clone();
        bound.source = SessionSource::File { relative_path: relative.to_string_lossy().into_owned() };
        // Unreadable transcripts are retried on the next scan; malformed records are skipped.
        if let Ok(Some(meta)) = scan_session_file(&path, &bound) { sessions.push(meta); }
    }
    Ok(sessions)
}

fn collect_jsonl_files(root: &Path, files: &mut Vec<PathBuf>) -> io::Result<()> {
    let meta = match std::fs::symlink_metadata(root) {
        Ok(meta) => meta,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(error),
    };
    if meta.file_type().is_symlink() || !meta.is_dir() { return Ok(()); }
    for entry in std::fs::read_dir(root)? {
        let entry = entry?;
        let kind = entry.file_type()?;
        let path = entry.path();
        if kind.is_dir() { collect_jsonl_files(&path, files)?; }
        else if kind.is_file() && path.extension().and_then(|v| v.to_str()) == Some("jsonl") { files.push(path); }
    }
    Ok(())
}

/// Claude Code keeps session metadata (`last-prompt`, `custom-title`, `relocated`, …)
/// within the last 64 KiB of the transcript instead of appending it after every turn,
/// so a `/cd` record can sit tens of kilobytes before EOF. The 30-line head/tail
/// window used for titles is not enough to find it.
const RELOCATED_TAIL_BYTES: u64 = 64 * 1024;

/// Last `/cd` target recorded in the transcript, read from the same 64 KiB window
/// Claude Code itself uses. `None` when the transcript has no relocated record.
fn last_relocated_cwd(path: &Path) -> Option<String> {
    let mut file = File::open(path).ok()?;
    let len = file.metadata().ok()?.len();
    let start = len.saturating_sub(RELOCATED_TAIL_BYTES);
    file.seek(SeekFrom::Start(start)).ok()?;
    let mut buf = Vec::new();
    file.read_to_end(&mut buf).ok()?;
    let text = String::from_utf8_lossy(&buf);
    let mut lines: Vec<&str> = text.lines().collect();
    // Seeking into the middle of a line leaves a partial first line.
    if start > 0 && !lines.is_empty() {
        lines.remove(0);
    }
    lines
        .into_iter()
        .rev()
        .filter(|line| line.contains("\"relocated\""))
        .filter_map(|line| serde_json::from_str::<Value>(line).ok())
        .filter(|value| value.get("type").and_then(Value::as_str) == Some("relocated"))
        .find_map(|value| {
            value
                .get("relocatedCwd")
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|cwd| !cwd.is_empty())
                .map(str::to_string)
        })
}

/// 列表扫描用：读不了（权限、文件被占用等）返回 `Err`，不进解析缓存、下轮重试；
/// 读到了但不是会话返回 `Ok(None)`，可以缓存。
pub(crate) fn scan_session_file(path: &Path, identity: &ExternalSessionIdentity) -> std::io::Result<Option<SessionMeta>> {
    if is_agent_session(path) {
        return Ok(None);
    }
    let (head, tail) = read_head_tail_lines(path, 10, 30)?;
    let relocated_cwd = last_relocated_cwd(path);
    Ok(parse_session_lines(path, head, tail, relocated_cwd, identity))
}

fn parse_session_lines(
    path: &Path,
    head: Vec<String>,
    tail: Vec<String>,
    relocated_cwd: Option<String>,
    identity: &ExternalSessionIdentity,
) -> Option<SessionMeta> {
    let mut session_id: Option<String> = None;
    let mut project_dir: Option<String> = None;
    let mut created_at: Option<i64> = None;
    let mut first_user_message: Option<String> = None;

    // Extract metadata and first user message from head lines
    for line in &head {
        let value: Value = match serde_json::from_str(line) {
            Ok(parsed) => parsed,
            Err(_) => continue,
        };
        if session_id.is_none() {
            session_id = value
                .get("sessionId")
                .and_then(Value::as_str)
                .map(|s| s.to_string());
        }
        if project_dir.is_none() {
            project_dir = value
                .get("cwd")
                .and_then(Value::as_str)
                .map(|s| s.to_string());
        }
        if created_at.is_none() {
            created_at = value.get("timestamp").and_then(parse_timestamp_to_ms);
        }
        // Extract first real user message as title candidate
        // Skip system-injected caveats and slash commands (e.g. /clear, /compact)
        if first_user_message.is_none() {
            let is_user = value.get("type").and_then(Value::as_str) == Some("user")
                || value
                    .get("message")
                    .and_then(|m| m.get("role"))
                    .and_then(Value::as_str)
                    == Some("user");
            if is_user {
                if let Some(message) = value.get("message") {
                    let text = message.get("content").map(extract_text).unwrap_or_default();
                    let trimmed = text.trim();
                    if !trimmed.is_empty()
                        && !trimmed.starts_with("<command-name>")
                        && !is_injected_text(trimmed)
                    {
                        first_user_message = Some(trimmed.to_string());
                    }
                }
            }
        }
        if session_id.is_some()
            && project_dir.is_some()
            && created_at.is_some()
            && first_user_message.is_some()
        {
            break;
        }
    }

    // Extract last_active_at, summary, and custom-title from tail lines (reverse order).
    let mut last_active_at: Option<i64> = None;
    let mut summary: Option<String> = None;
    let mut custom_title: Option<String> = None;

    for line in tail.iter().rev() {
        let value: Value = match serde_json::from_str(line) {
            Ok(parsed) => parsed,
            Err(_) => continue,
        };
        if last_active_at.is_none() {
            last_active_at = value.get("timestamp").and_then(parse_timestamp_to_ms);
        }
        // Look for custom-title entry (take the last one, i.e. first in reverse)
        if custom_title.is_none()
            && value.get("type").and_then(Value::as_str) == Some("custom-title")
        {
            custom_title = value
                .get("customTitle")
                .and_then(Value::as_str)
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty());
        }
        if summary.is_none() {
            if value.get("isMeta").and_then(Value::as_bool) == Some(true) {
                continue;
            }
            if let Some(message) = value.get("message") {
                let text = message.get("content").map(extract_text).unwrap_or_default();
                let trimmed = text.trim();
                // A local command run last, such as `/model`, ends the file with its command and output.
                if !trimmed.is_empty()
                    && !trimmed.starts_with("<command-name>")
                    && !is_injected_text(trimmed)
                {
                    summary = Some(text);
                }
            }
        }
        if last_active_at.is_some() && summary.is_some() && custom_title.is_some() {
            break;
        }
    }

    // `/cd` moved the session; the last relocated record wins over the cwd the
    // transcript started with. Sessions without a relocated record keep that cwd.
    //
    // Ordinary records are deliberately ignored: their `cwd` is the Bash tool's
    // tracked working directory and drifts into subdirectories as commands run,
    // which must not move the session.
    if relocated_cwd.is_some() {
        project_dir = relocated_cwd;
    }

    let session_id = session_id.or_else(|| infer_session_id_from_filename(path));
    let session_id = session_id?;

    // Title priority: custom-title > first user message > directory basename
    let title = custom_title
        .map(|t| truncate_summary(&t, TITLE_MAX_CHARS))
        .or_else(|| first_user_message.map(|t| truncate_summary(&t, TITLE_MAX_CHARS)))
        .or_else(|| {
            project_dir
                .as_deref()
                .and_then(path_basename)
                .map(|v| v.to_string())
        });

    let summary = summary.map(|text| truncate_summary(&text, 160));

    Some(SessionMeta {
        identity: ExternalSessionIdentity { session_id: ExternalSessionId(session_id), ..identity.clone() },
        capabilities: SessionCapabilities { read: true, delete: true, resume: true, export_markdown: true },
        title,
        summary,
        project_dir,
        created_at,
        last_active_at,
        source_path: Some(path.to_string_lossy().to_string()),
    })
}

fn is_agent_session(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .map(|name| name.starts_with("agent-") || name == "journal.jsonl")
        .unwrap_or(false)
}

fn infer_session_id_from_filename(path: &Path) -> Option<String> {
    path.file_stem()
        .and_then(|stem| stem.to_str())
        .map(|stem| stem.to_string())
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


fn path_basename(value: &str) -> Option<String> {
    value.trim_end_matches(['/', '\\']).rsplit(['/', '\\']).find(|part| !part.is_empty()).map(str::to_owned)
}
fn is_injected_text(text: &str) -> bool {
    text.contains("<local-command-caveat>") || text.starts_with("<local-command-stdout>") || text.starts_with("<local-command-stderr>") || text.starts_with("<task-notification>")
}

#[path = "claude_messages.rs"]
mod messages;
/// Read a Claude transcript selected by the authorized source resolver.
pub use messages::load_messages;
