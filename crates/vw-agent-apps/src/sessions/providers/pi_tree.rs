//! Pi active-branch tree reconstruction from the pinned native format.
use std::collections::{HashMap, HashSet};
use std::fs::{self, File};
use std::io::{BufRead, BufReader};
use std::path::Path;
use serde_json::Value;
use vw_api_types::external_session::*;
use crate::sessions::utils::{extract_text, truncate_summary};
use super::pi::{PiSessionContext, session_files, resolve_session_root, SessionRootResolution};
use super::pi_blocks::PiTranscript;
use super::pi_jsonl::{for_each_jsonl_value, parse_timestamp_to_ms};
const MAX_TREE_ENTRIES: usize = 500_000;
const MAX_TREE_ID_BYTES: usize = 256;
const MAX_SESSION_BYTES: u64 = 128 * 1024 * 1024;
pub(super) struct Dialect { pub label: &'static str, pub title_slot: bool, pub model_selector: bool }
const PI: Dialect = Dialect { label: "Pi", title_slot: false, model_selector: false };

/// Reuse the native tree parser before a destructive operation. The caller
/// must first resolve the path through the authorized source registry.
pub(crate) fn verify_pi_session_id(path: &Path, session_id: &str) -> Result<(), String> {
    if path.extension().and_then(|value| value.to_str()) != Some("jsonl") {
        return Err("Expected a Pi JSONL session".into());
    }
    let tree = read_tree(&PI, path)?;
    if tree.header.id != session_id {
        return Err("Pi session ID mismatch".into());
    }
    Ok(())
}

pub(super) fn verify_omp_session_id(path: &Path, session_id: &str) -> Result<(), String> {
    if path.extension().and_then(|value| value.to_str()) != Some("jsonl") {
        return Err("Expected an OMP JSONL session".into());
    }
    let dialect = Dialect { label: "Oh My Pi", title_slot: true, model_selector: true };
    if read_tree(&dialect, path)?.header.id != session_id {
        return Err("OMP session ID mismatch".into());
    }
    Ok(())
}

/// `context` is resolved on the authorized host. A request path alone is insufficient.
pub fn load_messages(context: &PiSessionContext<'_>, path: &Path) -> Result<Vec<SessionMessage>, String> {
    let files = session_files(context).map_err(|e| e.to_string())?;
    if !files.iter().any(|file| file == path) { return Err("Pi session is outside the authorized layout".into()); }
    load_messages_for(&PI, path)
}

pub fn scan_sessions(context: &PiSessionContext<'_>, identity: &ExternalSessionIdentity) -> Result<Vec<SessionMeta>, String> {
    if identity.provider != SessionProvider::Pi { return Err("Expected a Pi application identity".into()); }
    let root = match resolve_session_root(context) {
        SessionRootResolution::Available { root, .. } => root,
        _ => return Err("Pi session root is unavailable".into()),
    };
    let mut sessions = Vec::new();
    for path in session_files(context).map_err(|e| e.to_string())? {
        let tree = read_tree(&PI, &path)?;
        let mut session_identity = identity.clone();
        session_identity.session_id = ExternalSessionId(tree.header.id);
        session_identity.source = SessionSource::File { relative_path: path.strip_prefix(&root).map_err(|_| "Pi source escaped root")?.to_str().ok_or("Pi source path is not UTF-8")?.to_owned() };
        let title = tree.summary.explicit_name.flatten().or_else(|| tree.summary.first_user_message.as_deref().map(|s| truncate_summary(s, 80)));
        sessions.push(SessionMeta {
            identity: session_identity,
            capabilities: SessionCapabilities { read: true, delete: true, resume: true, export_markdown: true },
            title, summary: tree.summary.last_message.as_deref().map(|s| truncate_summary(s, 160)),
            project_dir: (!tree.header.cwd.is_empty()).then_some(tree.header.cwd),
            created_at: tree.header.timestamp,
            last_active_at: tree.summary.last_active_at.or(tree.header.timestamp),
            source_path: path.to_str().map(str::to_owned),
        });
    }
    Ok(sessions)
}

// OMP shares the pinned tree format, with native title slots and model selectors.
pub(super) fn load_messages_for(d: &Dialect, path: &Path) -> Result<Vec<SessionMessage>, String> {
    let tree = read_tree(d, path)?;
    read_active_messages(d, path, &tree)
}

pub(super) fn scan_files_for(d: &Dialect, root: &Path, files: Vec<std::path::PathBuf>, identity: &ExternalSessionIdentity) -> Result<Vec<SessionMeta>, String> {
    let mut sessions = Vec::new();
    for path in files {
        let tree = read_tree(d, &path)?;
        let mut session_identity = identity.clone();
        session_identity.session_id = ExternalSessionId(tree.header.id);
        session_identity.source = SessionSource::File { relative_path: path.strip_prefix(root).map_err(|_| "Session escaped authorized root")?.to_str().ok_or("Session path is not UTF-8")?.to_owned() };
        let title = tree.summary.explicit_name.flatten().or_else(|| tree.summary.first_user_message.as_deref().map(|s| truncate_summary(s, 80)));
        sessions.push(SessionMeta {
            identity: session_identity,
            capabilities: SessionCapabilities { read: true, delete: false, resume: true, export_markdown: true },
            title, summary: tree.summary.last_message.as_deref().map(|s| truncate_summary(s, 160)),
            project_dir: (!tree.header.cwd.is_empty()).then_some(tree.header.cwd),
            created_at: tree.header.timestamp,
            last_active_at: tree.summary.last_active_at.or(tree.header.timestamp),
            source_path: path.to_str().map(str::to_owned),
        });
    }
    Ok(sessions)
}

#[derive(Debug)]
struct SessionHeader {
    id: String,
    cwd: String,
    timestamp: Option<i64>,
    version: u64,
    title: Option<String>,
}

#[derive(Debug)]
struct SessionTree {
    header: SessionHeader,
    active_entry_indexes: HashSet<usize>,
    summary: SessionSummary,
}

#[derive(Debug, Default)]
struct SessionSummary {
    first_user_message: Option<String>,
    last_message: Option<String>,
    explicit_name: Option<Option<String>>,
    last_active_at: Option<i64>,
}

fn read_tree(d: &Dialect, path: &Path) -> Result<SessionTree, String> {
    let label = d.label;
    validate_file_size(d, path)?;
    let reader = BufReader::new(
        File::open(path).map_err(|error| format!("Failed to open {label} session: {error}"))?,
    );
    let mut header = None;
    let mut parents = HashMap::<String, (Option<String>, usize)>::new();
    let mut latest_id = None;
    let mut legacy_previous_id = None;
    let mut entry_index = 0usize;
    let mut summary = SessionSummary::default();
    for line in reader.lines() {
        let line = line.map_err(|error| format!("Failed to read {label} session: {error}"))?;
        if line.trim().is_empty() {
            continue;
        }
        let Ok(value) = serde_json::from_str::<Value>(&line) else {
            continue;
        };
        if header.is_none() {
            if d.title_slot && value.get("type").and_then(Value::as_str) == Some("title") {
                summary.explicit_name = Some(non_empty_text(value.get("title")));
                continue;
            }
            let parsed = parse_header(d, &value)?;
            if summary.explicit_name.is_none() && parsed.title.is_some() {
                summary.explicit_name = Some(parsed.title.clone());
            }
            header = Some(parsed);
            continue;
        }
        entry_index += 1;
        if entry_index > MAX_TREE_ENTRIES {
            return Err(format!(
                "{label} session exceeds the {MAX_TREE_ENTRIES}-entry safety limit"
            ));
        }
        update_session_summary(&mut summary, &value);
        let version = header
            .as_ref()
            .map_or(1, |item: &SessionHeader| item.version);
        let Some((id, parent_id)) =
            entry_identity(&value, version, entry_index, legacy_previous_id.as_deref())
        else {
            return Err(format!("{label} entry has an invalid tree ID"));
        };
        if parents
            .insert(id.clone(), (parent_id, entry_index))
            .is_some()
        {
            return Err(format!("{label} session contains duplicate entry ID"));
        }
        latest_id = Some(id.clone());
        legacy_previous_id = Some(id);
    }
    let header = header.ok_or_else(|| format!("{label} session has no valid header"))?;
    let mut active_entry_indexes = HashSet::new();
    let mut visited_ids = HashSet::new();
    let mut current = latest_id;
    while let Some(id) = current {
        if !visited_ids.insert(id.clone()) {
            return Err(format!("{label} session tree contains a cycle"));
        }
        let Some((parent_id, entry_index)) = parents.get(&id) else {
            return Err(format!("{label} session tree references a missing entry"));
        };
        active_entry_indexes.insert(*entry_index);
        current = parent_id.clone();
    }
    Ok(SessionTree {
        header,
        active_entry_indexes,
        summary,
    })
}

fn update_session_summary(summary: &mut SessionSummary, value: &Value) {
    if value.get("type").and_then(Value::as_str) == Some("session_info") {
        summary.explicit_name = Some(non_empty_text(value.get("name")));
        return;
    }
    let Some((role, content)) = value
        .get("message")
        .filter(|_| value.get("type").and_then(Value::as_str) == Some("message"))
        .and_then(parse_message)
    else {
        return;
    };
    if !matches!(role.as_str(), "user" | "assistant") {
        return;
    }
    let timestamp = value
        .get("message")
        .and_then(|message| message.get("timestamp"))
        .and_then(parse_timestamp_to_ms)
        .or_else(|| value.get("timestamp").and_then(parse_timestamp_to_ms));
    if let Some(timestamp) = timestamp {
        summary.last_active_at = Some(
            summary
                .last_active_at
                .map_or(timestamp, |current| current.max(timestamp)),
        );
    }
    if role == "user" && summary.first_user_message.is_none() {
        summary.first_user_message = Some(content.clone());
    }
    summary.last_message = Some(content);
}

fn non_empty_text(value: Option<&Value>) -> Option<String> {
    value
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|text| !text.is_empty())
        .map(str::to_string)
}

fn read_active_messages(
    d: &Dialect,
    path: &Path,
    tree: &SessionTree,
) -> Result<Vec<SessionMessage>, String> {
    let label = d.label;
    validate_file_size(d, path)?;
    let mut transcript = PiTranscript::new();
    let mut saw_header = false;
    let mut entry_index = 0usize;
    let mut legacy_previous_id = None;
    for_each_jsonl_value(path, |span, mut value| {
        if !saw_header {
            if value.get("type").and_then(Value::as_str) == Some("session") {
                saw_header = true;
            }
            return Ok(());
        }
        entry_index += 1;
        if entry_index > MAX_TREE_ENTRIES {
            return Err(format!(
                "{label} session exceeds the {MAX_TREE_ENTRIES}-entry safety limit"
            ));
        }
        let Some((id, _)) = entry_identity(
            &value,
            tree.header.version,
            entry_index,
            legacy_previous_id.as_deref(),
        ) else {
            return Ok(());
        };
        legacy_previous_id = Some(id.clone());
        if tree.active_entry_indexes.contains(&entry_index) {
            if d.model_selector {
                split_model_selector(&mut value);
            }
            transcript.push_entry(&value, span, Some(id));
        }
        Ok(())
    })
    .map_err(|error| error.replace("session file", &format!("{label} session")))?;
    Ok(transcript.finish())
}

/// Oh My Pi 的模型串是 `provider/id`，转录块按 Pi 的 `provider` + `modelId` 读。
fn split_model_selector(value: &mut Value) {
    if value.get("type").and_then(Value::as_str) != Some("model_change") {
        return;
    }
    let Some(model) = value
        .get("model")
        .and_then(Value::as_str)
        .map(str::to_string)
    else {
        return;
    };
    match model.split_once('/') {
        Some((provider, model_id)) => {
            value["provider"] = Value::String(provider.to_string());
            value["modelId"] = Value::String(model_id.to_string());
        }
        None => value["modelId"] = Value::String(model),
    }
}

fn parse_header(d: &Dialect, value: &Value) -> Result<SessionHeader, String> {
    let label = d.label;
    if value.get("type").and_then(Value::as_str) != Some("session") {
        return Err(format!(
            "{label} session header must be the first valid JSON entry"
        ));
    }
    let id = value
        .get("id")
        .and_then(Value::as_str)
        .filter(|id| is_valid_tree_id(id))
        .ok_or_else(|| format!("{label} session header has an invalid ID"))?
        .to_string();
    let version = value.get("version").and_then(Value::as_u64).unwrap_or(1);
    if version == 0 {
        return Err(format!("Unsupported {label} session version: {version}"));
    }
    Ok(SessionHeader {
        id,
        cwd: value
            .get("cwd")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string(),
        timestamp: value.get("timestamp").and_then(parse_timestamp_to_ms),
        version,
        title: d
            .title_slot
            .then(|| non_empty_text(value.get("title")))
            .flatten(),
    })
}

fn entry_identity(
    value: &Value,
    version: u64,
    entry_index: usize,
    legacy_previous_id: Option<&str>,
) -> Option<(String, Option<String>)> {
    if version < 2 {
        return Some((
            format!("legacy-{entry_index}"),
            legacy_previous_id.map(str::to_string),
        ));
    }
    let id = value
        .get("id")
        .and_then(Value::as_str)
        .filter(|id| is_valid_tree_id(id))?
        .to_string();
    let parent_id = match value.get("parentId") {
        None | Some(Value::Null) => None,
        Some(Value::String(parent)) if is_valid_tree_id(parent) => Some(parent.clone()),
        _ => return None,
    };
    Some((id, parent_id))
}

fn parse_message(message: &Value) -> Option<(String, String)> {
    let role = message.get("role").and_then(Value::as_str)?;
    let (display_role, content) = match role {
        "user" | "assistant" => (
            role.to_string(),
            message.get("content").map(extract_text).unwrap_or_default(),
        ),
        "toolResult" => (
            "tool".to_string(),
            message.get("content").map(extract_text).unwrap_or_default(),
        ),
        "bashExecution" => (
            "tool".to_string(),
            format!(
                "$ {}\n{}",
                message
                    .get("command")
                    .and_then(Value::as_str)
                    .unwrap_or_default(),
                message
                    .get("output")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
            ),
        ),
        "branchSummary" | "compactionSummary" => (
            "system".to_string(),
            message
                .get("summary")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string(),
        ),
        _ => return None,
    };
    (!content.trim().is_empty()).then_some((display_role, content))
}

fn validate_file_size(d: &Dialect, path: &Path) -> Result<(), String> {
    let label = d.label;
    let metadata = fs::symlink_metadata(path)
        .map_err(|error| format!("Failed to inspect {label} session: {error}"))?;
    if !metadata.is_file() || metadata.file_type().is_symlink() || metadata.len() > MAX_SESSION_BYTES {
        Err(format!(
            "{label} session exceeds the {MAX_SESSION_BYTES}-byte safety limit"
        ))
    } else {
        Ok(())
    }
}

pub(crate) fn is_valid_tree_id(id: &str) -> bool {
    let bytes = id.as_bytes();
    !bytes.is_empty()
        && bytes.len() <= MAX_TREE_ID_BYTES
        && bytes.first().is_some_and(u8::is_ascii_alphanumeric)
        && bytes.last().is_some_and(u8::is_ascii_alphanumeric)
        && bytes
            .iter()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
}

