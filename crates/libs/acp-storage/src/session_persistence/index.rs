//! 会话索引文件的构建、重建与查询逻辑。

use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use tokio::fs;

use crate::{SessionRecord, parse_session_record};

#[cfg(test)]
#[path = "index_tests.rs"]
mod index_tests;

pub const SESSION_INDEX_SCHEMA: &str = "vwacp.session-index.v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionIndexEntry {
    pub file: String,
    pub vwacp_record_id: String,
    pub acp_session_id: String,
    pub agent_command: String,
    pub cwd: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    pub closed: bool,
    pub last_used_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionIndex {
    pub files: Vec<String>,
    pub entries: Vec<SessionIndexEntry>,
}

fn as_record(value: &Value) -> Option<&Map<String, Value>> {
    match value {
        Value::Object(record) => Some(record),
        _ => None,
    }
}

fn parse_index_entry(raw: &Value) -> Option<SessionIndexEntry> {
    let record = as_record(raw)?;
    let file = record.get("file")?.as_str()?.to_string();
    let vwacp_record_id = record.get("vwacpRecordId")?.as_str()?.to_string();
    let acp_session_id = record.get("acpSessionId")?.as_str()?.to_string();
    let agent_command = record.get("agentCommand")?.as_str()?.to_string();
    let cwd = record.get("cwd")?.as_str()?.to_string();
    let last_used_at = record.get("lastUsedAt")?.as_str()?.to_string();
    let closed = record.get("closed")?.as_bool()?;
    let name = match record.get("name") {
        Some(Value::String(value)) => Some(value.clone()),
        Some(Value::Null) | None => None,
        _ => return None,
    };

    Some(SessionIndexEntry {
        file,
        vwacp_record_id,
        acp_session_id,
        agent_command,
        cwd,
        name,
        closed,
        last_used_at,
    })
}

fn temp_suffix() -> u128 {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_millis()
}

async fn session_files(session_dir: &Path) -> std::io::Result<Vec<String>> {
    let mut entries = fs::read_dir(session_dir).await?;
    let mut files = Vec::new();
    while let Some(entry) = entries.next_entry().await? {
        let file_type = entry.file_type().await?;
        let file_name = entry.file_name();
        let file_name = file_name.to_string_lossy();
        if file_type.is_file() && file_name.ends_with(".json") && file_name != "index.json" {
            files.push(file_name.into_owned());
        }
    }
    files.sort();
    Ok(files)
}

pub fn session_index_path(session_dir: impl AsRef<Path>) -> PathBuf {
    session_dir.as_ref().join("index.json")
}

pub fn to_session_index_entry(record: &SessionRecord, file_name: &str) -> SessionIndexEntry {
    SessionIndexEntry {
        file: file_name.to_string(),
        vwacp_record_id: record.vwacp_record_id.clone(),
        acp_session_id: record.acp_session_id.clone(),
        agent_command: record.agent_command.clone(),
        cwd: record.cwd.clone(),
        name: record.name.clone(),
        closed: record.closed.unwrap_or(false),
        last_used_at: record.last_used_at.clone(),
    }
}

pub async fn read_session_index(session_dir: impl AsRef<Path>) -> Option<SessionIndex> {
    let file_path = session_index_path(session_dir);
    let payload = fs::read_to_string(file_path).await.ok()?;
    let parsed: Value = serde_json::from_str(&payload).ok()?;
    let record = as_record(&parsed)?;
    if record.get("schema").and_then(Value::as_str) != Some(SESSION_INDEX_SCHEMA) {
        return None;
    }

    let files = record
        .get("files")?
        .as_array()?
        .iter()
        .map(Value::as_str)
        .map(|entry| entry.map(ToOwned::to_owned))
        .collect::<Option<Vec<_>>>()?;
    let entries = record
        .get("entries")?
        .as_array()?
        .iter()
        .map(parse_index_entry)
        .collect::<Option<Vec<_>>>()?;

    Some(SessionIndex { files, entries })
}

pub async fn write_session_index(
    session_dir: impl AsRef<Path>,
    index: &SessionIndex,
) -> std::io::Result<()> {
    let file_path = session_index_path(session_dir.as_ref());
    let temp_file = format!("{}.{}.{}.tmp", file_path.display(), std::process::id(), temp_suffix());

    let mut files = index.files.clone();
    files.sort();
    let mut entries = index.entries.clone();
    entries.sort_by(|left, right| right.last_used_at.cmp(&left.last_used_at));

    let payload = serde_json::to_vec_pretty(&serde_json::json!({
        "schema": SESSION_INDEX_SCHEMA,
        "files": files,
        "entries": entries,
    }))?;

    fs::write(&temp_file, [&payload[..], b"\n"].concat()).await?;
    fs::rename(temp_file, file_path).await
}

pub async fn rebuild_session_index(session_dir: impl AsRef<Path>) -> std::io::Result<SessionIndex> {
    let session_dir = session_dir.as_ref();
    let files = session_files(session_dir).await?;
    let mut entries = Vec::new();

    for file in &files {
        let Ok(payload) = fs::read_to_string(session_dir.join(file)).await else {
            continue;
        };
        let Ok(value) = serde_json::from_str::<Value>(&payload) else {
            continue;
        };
        let Some(record) = parse_session_record(&value) else {
            continue;
        };
        entries.push(to_session_index_entry(&record, file));
    }

    let index = SessionIndex { files, entries };
    write_session_index(session_dir, &index).await?;
    Ok(index)
}

pub async fn load_or_rebuild_session_index(
    session_dir: impl AsRef<Path>,
) -> std::io::Result<SessionIndex> {
    let session_dir = session_dir.as_ref();
    let files = session_files(session_dir).await?;
    if let Some(existing) = read_session_index(session_dir).await
        && existing.files.len() == files.len()
        && existing.files.iter().zip(&files).all(|(left, right)| left == right)
    {
        return Ok(existing);
    }

    rebuild_session_index(session_dir).await
}
