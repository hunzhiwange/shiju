//! Read-only Antigravity exports and summary index; encrypted binary bodies stay opaque.
use serde_json::Value;
use std::{collections::BTreeMap, fs, io::{BufRead, BufReader}, path::{Component, Path, PathBuf}};
use vw_api_types::external_session::*;
use crate::sessions::utils::truncate_summary;
use super::pi_jsonl::parse_timestamp_to_ms;

struct IndexEntry {
    session_id: String, title: Option<String>, summary: Option<String>,
    project_dir: Option<String>, created_at: Option<i64>, last_active_at: Option<i64>,
    source_path: Option<String>,
}

/// The CLI/IDE and provider-profile roots belong to the authorized execution host.
/// Callers resolve application configuration, never the gateway's process HOME.
pub fn session_roots(gemini_dir: &Path, profiles_root: &Path) -> Result<Vec<PathBuf>, String> {
    checked_path(gemini_dir)?;
    checked_path(profiles_root)?;
    let mut roots = vec![gemini_dir.join("antigravity-cli/brain"), gemini_dir.join("antigravity/brain")];
    match fs::read_dir(profiles_root) {
        Ok(entries) => {
            let mut profiles = Vec::new();
            for entry in entries {
                let entry = entry.map_err(|_| "Cannot enumerate Antigravity profiles")?;
                if entry.file_type().map_err(|_| "Cannot inspect Antigravity profile")?.is_dir() {
                    profiles.push(entry.path().join("antigravity-cli/brain"));
                }
            }
            profiles.sort(); roots.extend(profiles);
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {},
        Err(_) => return Err("Cannot enumerate Antigravity profiles".into()),
    }
    Ok(roots)
}

pub fn scan_sessions(roots: &[PathBuf], identity: &ExternalSessionIdentity) -> Result<Vec<SessionMeta>, String> {
    if identity.provider != SessionProvider::Agy { return Err("Expected Antigravity identity".into()); }
    for root in roots { checked_path(root)?; }
    let sources: Vec<_> = roots.iter().map(|root| (root.clone(), None)).collect();
    Ok(scan_roots(&sources)?.into_iter().map(|entry| {
        let source = match &entry.source_path {
            Some(path) => SessionSource::File { relative_path: roots.iter().find_map(|root| Path::new(path).strip_prefix(root).ok()).map(|p| p.to_string_lossy().into_owned()).unwrap_or_default() },
            None => SessionSource::Sqlite { database: "conversation_summaries.db".into(), record_id: entry.session_id.clone() },
        };
        SessionMeta {
            identity: ExternalSessionIdentity { session_id: ExternalSessionId(entry.session_id), source, ..identity.clone() },
            capabilities: SessionCapabilities { read: entry.source_path.is_some(), delete: false, resume: true, export_markdown: entry.source_path.is_some() },
            title: entry.title, summary: entry.summary, project_dir: entry.project_dir,
            created_at: entry.created_at, last_active_at: entry.last_active_at, source_path: entry.source_path,
        }
    }).collect())
}

pub fn delete_session(_path: &Path) -> Result<(), String> {
    Err("Antigravity session deletion is unsupported".into())
}

pub(crate) fn checked_path(path: &Path) -> Result<(), String> {
    if !path.is_absolute() || path.components().any(|c| matches!(c, Component::ParentDir)) {
        return Err("Invalid Antigravity host path".into());
    }
    let mut current = PathBuf::new();
    for component in path.components() {
        current.push(component);
        match fs::symlink_metadata(&current) {
            Ok(meta) if meta.file_type().is_symlink() => return Err("Antigravity symbolic links are forbidden".into()),
            Ok(_) => {},
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => break,
            Err(_) => return Err("Cannot inspect Antigravity path".into()),
        }
    }
    Ok(())
}

pub(crate) fn records(path: &Path) -> Result<Vec<Value>, String> {
    checked_path(path)?;
    let file =
        fs::File::open(path).map_err(|e| { let _ = e; "Cannot read Antigravity transcript".to_string() })?;
    // The export can rewrite steps as they finish. The latest record for a step wins.
    let mut steps = BTreeMap::new();
    for line in BufReader::new(file).lines() {
        let line = line.map_err(|e| { let _ = e; "Cannot read Antigravity transcript".to_string() })?;
        let Ok(value) = serde_json::from_str::<Value>(&line) else {
            continue;
        };
        if let Some(index) = value["step_index"].as_u64() {
            steps.insert(index, value);
        }
    }
    Ok(steps.into_values().collect())
}



/// Missing roots are tolerated; a session id found in several roots is reported once. The first
/// root wins, except that an index-only entry yields to a later root holding the transcript: the
/// IDE can continue a conversation the CLI index also lists, and only the transcript has usage.
fn scan_roots(sources: &[(PathBuf, Option<PathBuf>)]) -> Result<Vec<IndexEntry>, String> {
    let mut seen: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    let mut sessions: Vec<IndexEntry> = Vec::new();
    for (root, gemini_dir) in sources {
        for meta in scan_root(root, gemini_dir.as_deref())? {
            match seen.get(&meta.session_id) {
                None => {
                    seen.insert(meta.session_id.clone(), sessions.len());
                    sessions.push(meta);
                }
                // Replace the whole entry so its resume command matches the transcript's root.
                Some(&i) if sessions[i].source_path.is_none() && meta.source_path.is_some() => {
                    sessions[i] = meta;
                }
                Some(_) => {}
            }
        }
    }
    Ok(sessions)
}

fn valid_session_id(id: &str) -> bool {
    !id.is_empty() && id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
}

fn scan_root(root: &Path, gemini_dir: Option<&Path>) -> Result<Vec<IndexEntry>, String> {
    let mut summaries = read_summaries(
        &root
            .parent()
            .unwrap_or(root)
            .join("conversation_summaries.db"),
        gemini_dir,
    )?;
    let canonical_root = root.canonicalize().ok();
    let mut sessions = Vec::new();
    let entries = match fs::read_dir(root) {
        Ok(entries) => Some(entries),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
        Err(_) => return Err("Cannot enumerate Antigravity brain".into()),
    };
    if let Some(entries) = entries {
        for entry in entries {
            let entry = entry.map_err(|_| "Cannot inspect Antigravity session")?;
            if !entry.file_type().is_ok_and(|t| t.is_dir()) {
                continue;
            }
            let Some(id) = entry
                .file_name()
                .to_str()
                .map(str::to_string)
                .filter(|id| valid_session_id(id))
            else {
                continue;
            };
            let path = entry
                .path()
                .join(".system_generated/logs/transcript_full.jsonl");
            if !path.is_file()
                || !path
                    .canonicalize()
                    .ok()
                    .zip(canonical_root.as_ref())
                    .is_some_and(|(path, root)| path.starts_with(root))
            {
                continue;
            }
            let summary = summaries.remove(&id);
            // Avoid parsing every transcript on list refresh when agy's own index is available.
            let mut meta = if let Some(meta) = summary {
                meta
            } else {
                let rows = records(&path)?;
                let first = rows
                    .iter()
                    .find(|v| v["type"] == "USER_INPUT")
                    .and_then(|v| v["content"].as_str())
                    .map(user_request);
                IndexEntry {
                    session_id: id.clone(),
                    title: first.map(|s| truncate_summary(s, 80)),
                    summary: first.map(|s| truncate_summary(s, 160)),
                    project_dir: None,
                    created_at: rows.iter().find_map(timestamp),
                    last_active_at: rows.iter().filter_map(timestamp).max(),
                    source_path: None,
                }
            };
            meta.source_path = Some(path.to_string_lossy().into_owned());
            sessions.push(meta);
        }
    }
    // A binary-only conversation can still be found and resumed, but has no readable transcript.
    sessions.extend(summaries.into_values());
    Ok(sessions)
}

fn read_summaries(path: &Path, _gemini_dir: Option<&Path>) -> Result<BTreeMap<String, IndexEntry>, String> {
    let mut result = BTreeMap::new();
    if !path.exists() { return Ok(result); }
    checked_path(path)?;
    let conn = rusqlite::Connection::open_with_flags(path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
        .map_err(|_| "Cannot open Antigravity summary database")?;
    let mut query = conn.prepare("SELECT conversation_id, title, workspace_uris, preview, last_modified_time FROM conversation_summaries")
        .map_err(|_| "Unsupported Antigravity summary schema")?;
    let rows = query.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?, r.get::<_, String>(2)?, r.get::<_, String>(3)?, r.get::<_, String>(4)?)))
        .map_err(|_| "Cannot read Antigravity summaries")?;
    for row in rows {
        let (id, title, uris, preview, modified) = row.map_err(|_| "Invalid Antigravity summary row")?;
        if !valid_session_id(&id) {
            continue;
        }
        let project_dir = serde_json::from_str::<Vec<String>>(&uris)
            .ok()
            .and_then(|v| v.into_iter().next())
            .and_then(|uri| reqwest::Url::parse(&uri).ok()?.to_file_path().ok())
            .map(|p| p.to_string_lossy().into_owned());
        let last_active_at =
            parse_timestamp_to_ms(&Value::String(modified.clone())).or_else(|| {
                chrono::NaiveDateTime::parse_from_str(&modified, "%Y-%m-%d %H:%M:%S%.f")
                    .ok()
                    .map(|t| t.and_utc().timestamp_millis())
            });
        result.insert(
            id.clone(),
            IndexEntry {
                session_id: id.clone(),
                title: (!title.is_empty()).then_some(title),
                summary: (!preview.is_empty()).then(|| truncate_summary(&preview, 160)),
                project_dir,
                created_at: None,
                last_active_at,
                source_path: None,
            },
        );
    }
    Ok(result)
}

/// agy stores each prompt wrapped in `<USER_REQUEST>`, followed by context it adds itself
/// (`<ADDITIONAL_METADATA>`, `<USER_SETTINGS_CHANGE>`, ...). Only the part the user typed is shown.
fn user_request(content: &str) -> &str {
    content
        .split_once("<USER_REQUEST>")
        .and_then(|(_, rest)| rest.split_once("</USER_REQUEST>"))
        .map(|(inner, _)| inner.trim())
        .filter(|inner| !inner.is_empty())
        .unwrap_or(content)
}

pub(crate) fn timestamp(value: &Value) -> Option<i64> {
    parse_timestamp_to_ms(&value["created_at"])
}

pub fn load_messages(path: &Path) -> Result<Vec<SessionMessage>, String> {
    checked_path(path)?;
    let mut messages = Vec::new();
    for value in records(path)? {
        let role = match value["type"].as_str() {
            Some("USER_INPUT") => "user",
            Some("PLANNER_RESPONSE") => "assistant",
            _ if value["source"] == "MODEL" => "assistant",
            _ => "system",
        };
        let content = value["content"].as_str().unwrap_or("");
        let content = if role == "user" {
            user_request(content)
        } else {
            content
        };
        // Keep unrecognized steps visible instead of silently dropping future tool/event types.
        let content = if content.is_empty() {
            serde_json::to_string_pretty(&value).unwrap_or_default()
        } else {
            content.into()
        };
        let mut message = SessionMessage::from_blocks(
            role,
            timestamp(&value),
            vec![SessionBlock::Text {
                text: content,
                full: None,
            }],
        );
        message.id = value["step_index"].as_u64().map(|n| n.to_string());
        message.meta = Some(MessageMeta {
            model: value["model"].as_str().map(String::from),
            input_tokens: value["input_tokens"].as_u64(),
            output_tokens: value["output_tokens"].as_u64(),
            cache_read_tokens: value["cache_read_tokens"].as_u64(),
            cache_write_tokens: value["cache_write_tokens"].as_u64(),
            reasoning_tokens: value["reasoning_tokens"].as_u64(),
            ..Default::default()
        });
        messages.push(message);
    }
    let mut turn = 0;
    for message in &mut messages {
        if message.role == "user" { turn += 1; }
        if turn > 0 { message.turn_id = Some(format!("t{turn}")); }
    }
    Ok(messages)
}

