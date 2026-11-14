//! Codex discovery. All roots and environment values belong to the authorized execution host.
use std::collections::HashMap;
use std::fs::File;
use std::io::{self, BufRead, BufReader, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::time::Duration;
use chrono::{DateTime, FixedOffset};
use serde_json::Value;
use vw_api_types::external_session::{ExternalSessionId, ExternalSessionIdentity, SessionCapabilities, SessionMeta, SessionProvider, SessionSource};
use crate::sessions::{codex_state::codex_state_db_paths, rollout};
use crate::sessions::utils::{extract_text, truncate_summary};
const TITLE_MAX_CHARS: usize = 80;
const VSCODE_CONTEXT_PREFIX: &str = "# Context from my IDE setup:";
const CODEX_REQUEST_MARKER: &str = "my request for codex";

/// No provider bucket filter: switching a model provider does not hide history.
/// The caller must authorize config_dir and any resolved sqlite_home before calling.
pub fn scan_sessions(config_dir: &Path, config_text: &str, execution_home: &Path,
    execution_sqlite_home: Option<&str>, identity: &ExternalSessionIdentity) -> io::Result<Vec<SessionMeta>> {
    if identity.provider != SessionProvider::Codex {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "expected a Codex instance"));
    }
    let databases = codex_state_db_paths(config_dir, config_text, execution_home, execution_sqlite_home)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))?;
    let mut titles = index_titles(&config_dir.join("session_index.jsonl"))?;
    for database in databases { titles.extend(database_titles(&database)?); }
    let mut files = Vec::new();
    collect_files(&config_dir.join("sessions"), &mut files, 0)?;
    collect_files(&config_dir.join("archived_sessions"), &mut files, 0)?;
    files.sort();
    let mut sessions: Vec<SessionMeta> = Vec::new();
    for file in rollout::dedupe_siblings(files) {
        let lines = read_session_lines(&file)?;
        let relative = file.strip_prefix(config_dir).map_err(io::Error::other)?;
        if let Some(meta) = parse_session_lines(&file, lines, &titles, identity, relative) {
            // Active tree wins over a duplicated archived thread; stable traversal.
            if let Some(existing) = sessions.iter_mut().find(|m| m.identity.session_id == meta.identity.session_id) {
                let old_archive = matches!(&existing.identity.source, SessionSource::File { relative_path } if relative_path.starts_with("archived_sessions/"));
                if old_archive && relative.starts_with("sessions") { *existing = meta; }
            } else { sessions.push(meta); }
        }
    }
    Ok(sessions)
}

fn collect_files(root: &Path, files: &mut Vec<PathBuf>, depth: usize) -> io::Result<()> {
    if depth > 32 || files.len() >= 100_000 { return Err(io::Error::other("Codex discovery limit exceeded")); }
    let metadata = match std::fs::symlink_metadata(root) {
        Ok(m) => m,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(()),
        Err(e) => return Err(e),
    };
    if metadata.file_type().is_symlink() || !metadata.is_dir() { return Ok(()); }
    for entry in std::fs::read_dir(root)? {
        let entry = entry?;
        let kind = entry.file_type()?;
        if kind.is_dir() { collect_files(&entry.path(), files, depth + 1)?; }
        else if kind.is_file() && rollout::is_rollout_file(&entry.path()) { files.push(entry.path()); }
    }
    Ok(())
}

fn index_titles(path: &Path) -> io::Result<HashMap<String, String>> {
    let mut titles = HashMap::new();
    let file = match File::open(path) {
        Ok(file) => file,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(titles),
        Err(e) => return Err(e),
    };
    for line in BufReader::new(file).lines() {
        let line = line?;
        let Ok(value) = serde_json::from_str::<Value>(&line) else { continue };
        if let (Some(id), Some(title)) = (value["id"].as_str(), value["thread_name"].as_str()) {
            if !id.trim().is_empty() && !title.trim().is_empty() {
                titles.insert(id.trim().into(), title.trim().into());
            }
        }
    }
    Ok(titles)
}

fn database_titles(path: &Path) -> io::Result<HashMap<String, String>> {
    if !path.try_exists()? { return Ok(HashMap::new()); }
    let map_error = |_| io::Error::other("Codex title database is unavailable");
    let connection = rusqlite::Connection::open_with_flags(path,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY | rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX).map_err(map_error)?;
    connection.busy_timeout(Duration::from_secs(2)).map_err(map_error)?;
    let mut query = connection.prepare("SELECT id, title FROM threads WHERE title <> '' AND (first_user_message IS NULL OR TRIM(title) <> TRIM(first_user_message))").map_err(map_error)?;
    let rows = query.query_map([], |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))).map_err(map_error)?;
    let mut titles = HashMap::new();
    for row in rows {
        let (id, title) = row.map_err(map_error)?;
        if !id.trim().is_empty() && !title.trim().is_empty() { titles.insert(id.trim().into(), title.trim().into()); }
    }
    Ok(titles)
}

fn path_basename(path: &str) -> Option<&str> { Path::new(path).file_name()?.to_str() }
struct SessionLines {
    head: Vec<String>,
    tail: Vec<String>,
    /// 结尾没读到时间时的兜底（毫秒）
    fallback_last_active: Option<i64>,
}

const HEAD_LINES: usize = 10;
const TAIL_LINES: usize = 30;

/// 压缩文件没法跳到结尾，整份解压又太慢（几十 GB 的会话目录每次打开都要等），
/// 所以只解压开头；最后活跃时间用文件 mtime（Codex 压缩时保留了它），不出预览。
fn read_session_lines(path: &Path) -> std::io::Result<SessionLines> {
    if !rollout::is_compressed(path) {
        let (head, tail) = read_head_tail_lines(path, HEAD_LINES, TAIL_LINES)?;
        return Ok(SessionLines {
            head,
            tail,
            fallback_last_active: None,
        });
    }
    let head = rollout::open_reader(path)?
        .lines()
        .take(HEAD_LINES)
        .map_while(Result::ok)
        .collect();
    let fallback_last_active = std::fs::metadata(path)
        .and_then(|meta| meta.modified())
        .ok()
        .and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok())
        .and_then(|duration| i64::try_from(duration.as_millis()).ok());
    Ok(SessionLines {
        head,
        tail: Vec::new(),
        fallback_last_active,
    })
}

fn parse_session_lines(
    path: &Path,
    lines: SessionLines,
    thread_titles: &HashMap<String, String>,
    identity: &ExternalSessionIdentity,
    relative_path: &Path,
) -> Option<SessionMeta> {
    let SessionLines {
        head,
        tail,
        fallback_last_active,
    } = lines;
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
        if created_at.is_none() {
            created_at = value.get("timestamp").and_then(parse_timestamp_to_ms);
        }
        if value.get("type").and_then(Value::as_str) == Some("session_meta") {
            if let Some(payload) = value.get("payload") {
                if is_subagent_source(payload.get("source")) {
                    return None;
                }
                if session_id.is_none() {
                    session_id = payload
                        .get("id")
                        .and_then(Value::as_str)
                        .map(|s| s.to_string());
                }
                if project_dir.is_none() {
                    project_dir = payload
                        .get("cwd")
                        .and_then(Value::as_str)
                        .map(|s| s.to_string());
                }
                if let Some(ts) = payload.get("timestamp").and_then(parse_timestamp_to_ms) {
                    created_at.get_or_insert(ts);
                }
            }
        }
        // Extract first user message as title candidate
        if first_user_message.is_none()
            && value.get("type").and_then(Value::as_str) == Some("response_item")
        {
            if let Some(payload) = value.get("payload") {
                if payload.get("type").and_then(Value::as_str) == Some("message")
                    && payload.get("role").and_then(Value::as_str) == Some("user")
                {
                    let text = payload.get("content").map(extract_text).unwrap_or_default();
                    if let Some(title) = title_candidate_from_user_message(&text) {
                        first_user_message = Some(title);
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

    // Extract last_active_at and summary from tail lines (reverse order)
    let mut last_active_at: Option<i64> = None;
    let mut summary: Option<String> = None;

    for line in tail.iter().rev() {
        let value: Value = match serde_json::from_str(line) {
            Ok(parsed) => parsed,
            Err(_) => continue,
        };
        if last_active_at.is_none() {
            last_active_at = value.get("timestamp").and_then(parse_timestamp_to_ms);
        }
        if summary.is_none() && value.get("type").and_then(Value::as_str) == Some("response_item") {
            if let Some(payload) = value.get("payload") {
                if payload.get("type").and_then(Value::as_str) == Some("message") {
                    let text = payload.get("content").map(extract_text).unwrap_or_default();
                    if !text.trim().is_empty() {
                        summary = Some(text);
                    }
                }
            }
        }
        if last_active_at.is_some() && summary.is_some() {
            break;
        }
    }

    let last_active_at = last_active_at.or(fallback_last_active);
    let session_id = session_id.or_else(|| infer_session_id_from_filename(path));
    let session_id = session_id?;

    let title = thread_titles
        .get(&session_id)
        .map(|t| truncate_summary(t, TITLE_MAX_CHARS))
        .or_else(|| first_user_message.map(|t| truncate_summary(&t, TITLE_MAX_CHARS)))
        .or_else(|| {
            project_dir
                .as_deref()
                .and_then(path_basename)
                .map(|v| v.to_string())
        });

    let summary = summary.map(|text| truncate_summary(&text, 160));

    let mut bound = identity.clone();
    bound.session_id = ExternalSessionId(session_id);
    bound.source = SessionSource::File { relative_path: relative_path.to_string_lossy().into_owned() };
    Some(SessionMeta {
        identity: bound,
        capabilities: SessionCapabilities { read: true, delete: true, resume: true, export_markdown: true },
        title,
        summary,
        project_dir,
        created_at,
        last_active_at,
        source_path: Some(path.to_string_lossy().to_string()),
    })
}

fn is_subagent_source(source: Option<&Value>) -> bool {
    source
        .and_then(|value| value.as_object())
        .map(|source| source.contains_key("subagent"))
        .unwrap_or(false)
}

fn title_candidate_from_user_message(text: &str) -> Option<String> {
    let trimmed = text.trim();
    if trimmed.is_empty()
        || trimmed.starts_with("# AGENTS.md")
        || trimmed.starts_with("<environment_context>")
    {
        return None;
    }

    if trimmed.starts_with(VSCODE_CONTEXT_PREFIX) {
        return extract_codex_prompt_from_ide_context(trimmed);
    }

    Some(trimmed.to_string())
}

fn extract_codex_prompt_from_ide_context(text: &str) -> Option<String> {
    let normalized = text.replace("\r\n", "\n");
    let lines = normalized.lines().collect::<Vec<_>>();

    // VS Code injects the real prompt as the LAST "## My request for Codex:"
    // section, so keep the final matching heading. Earlier matches can be
    // headings that live inside the active selection / open file content.
    // Trade-off: if the request body itself repeats the heading, the title
    // truncates to its trailing part (rare; covered by tests below).
    let mut prompt: Option<String> = None;
    for (index, line) in lines.iter().enumerate() {
        let Some(inline_prompt) = codex_request_heading_payload(line) else {
            continue;
        };

        if !inline_prompt.is_empty() {
            prompt = Some(inline_prompt.to_string());
            continue;
        }

        let following_prompt = lines[index + 1..].join("\n").trim().to_string();
        prompt = (!following_prompt.is_empty()).then_some(following_prompt);
    }

    prompt
}

fn codex_request_heading_payload(line: &str) -> Option<&str> {
    let trimmed = line.trim();
    if !trimmed.starts_with('#') {
        return None;
    }

    let heading = trimmed.trim_start_matches('#').trim_start();
    let lowered = heading.to_ascii_lowercase();
    if !lowered.starts_with(CODEX_REQUEST_MARKER) {
        return None;
    }

    let suffix = heading[CODEX_REQUEST_MARKER.len()..].trim_start();
    if suffix.is_empty() {
        return Some("");
    }

    let Some(separator) = suffix.chars().next() else {
        return Some("");
    };
    if !matches!(separator, ':' | '：' | '-' | '—') {
        return None;
    }

    Some(
        suffix
            .trim_start_matches(|c: char| c.is_whitespace() || matches!(c, ':' | '：' | '-' | '—'))
            .trim(),
    )
}

fn infer_session_id_from_filename(path: &Path) -> Option<String> {
    let name = path.file_name()?.to_str()?;
    name.as_bytes().windows(36).filter_map(|v| std::str::from_utf8(v).ok())
        .find(|s| uuid::Uuid::parse_str(s).is_ok()).map(str::to_owned)
}

pub fn read_head_tail_lines(
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

pub fn parse_timestamp_to_ms(value: &Value) -> Option<i64> {
    // Integer: milliseconds (>1e12) or seconds
    if let Some(n) = value.as_i64() {
        return Some(if n > 1_000_000_000_000 { n } else { n.checked_mul(1000)? });
    }
    if let Some(n) = value.as_f64() {
        let n = n as i64;
        return Some(if n > 1_000_000_000_000 { n } else { n.checked_mul(1000)? });
    }
    // RFC3339 string
    let raw = value.as_str()?;
    DateTime::parse_from_rfc3339(raw)
        .ok()
        .map(|dt: DateTime<FixedOffset>| dt.timestamp_millis())
}


/// Read an already-authorized rollout, including compressed siblings.
pub use super::codex_messages::load_messages;

/// Re-read one host-authorized rollout before destructive operations.
pub(crate) fn scan_session_file(
    path: &Path,
    identity: &ExternalSessionIdentity,
) -> io::Result<Option<SessionMeta>> {
    if identity.provider != SessionProvider::Codex || !rollout::is_rollout_file(path) {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "expected a Codex rollout"));
    }
    let SessionSource::File { relative_path } = &identity.source else {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "expected a file session"));
    };
    let head = rollout::open_reader(path)?.lines().take(HEAD_LINES)
        .collect::<io::Result<Vec<_>>>()?;
    Ok(parse_session_lines(path, SessionLines {
        head, tail: Vec::new(), fallback_last_active: None,
    }, &HashMap::new(), identity, Path::new(relative_path)))
}
