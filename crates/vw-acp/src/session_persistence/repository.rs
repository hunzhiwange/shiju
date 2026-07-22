//! 会话记录仓库的查找、读写与关闭操作。
//!
//! 本模块定义面向会话记录目录的仓库式操作接口，负责从工作区或显式路径中定位
//! 会话记录，并在读写时处理路径归一化、关闭标记与历史截断等细节。
//!
//! 它是持久化子系统与上层业务之间最常用的交互层。

use std::env;
use std::io;
use std::path::{Component, Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::Value;
use tokio::fs;

use crate::{
    PersistedKeyPolicyError, SessionIndexEntry, SessionNotFoundError, SessionRecord,
    SessionResolutionError, assert_persisted_key_policy, default_home_dir,
    load_or_rebuild_session_index, measure_perf, rebuild_session_index,
    serialize_session_record_for_disk, session_base_dir, terminate_process, to_session_index_entry,
    write_session_index,
};

#[cfg(test)]
#[path = "repository_edge_tests.rs"]
mod repository_edge_tests;
#[cfg(test)]
#[path = "repository_tests.rs"]
mod repository_tests;

pub const DEFAULT_HISTORY_LIMIT: usize = 20;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FindSessionOptions {
    pub agent_command: String,
    pub cwd: String,
    pub name: Option<String>,
    pub include_closed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FindSessionByDirectoryWalkOptions {
    pub agent_command: String,
    pub cwd: String,
    pub name: Option<String>,
    pub boundary: Option<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum SessionRepositoryError {
    #[error("HOME directory is unavailable")]
    HomeDirUnavailable,
    #[error(transparent)]
    Io(#[from] io::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error(transparent)]
    PersistedKeyPolicy(#[from] PersistedKeyPolicyError),
    #[error(transparent)]
    SessionNotFound(#[from] SessionNotFoundError),
    #[error(transparent)]
    SessionResolution(#[from] SessionResolutionError),
}

pub type SessionRepositoryResult<T> = Result<T, SessionRepositoryError>;

fn percent_encode_component(value: &str) -> String {
    let mut encoded = String::with_capacity(value.len());
    for byte in value.bytes() {
        if byte.is_ascii_alphanumeric()
            || matches!(byte, b'-' | b'_' | b'.' | b'!' | b'~' | b'*' | b'\'' | b'(' | b')')
        {
            encoded.push(byte as char);
        } else {
            encoded.push('%');
            encoded.push_str(&format!("{byte:02X}"));
        }
    }
    encoded
}

fn temp_suffix() -> u128 {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_millis()
}

#[allow(clippy::result_large_err)]
fn default_repository_dir() -> SessionRepositoryResult<PathBuf> {
    let home_dir = default_home_dir().ok_or(SessionRepositoryError::HomeDirUnavailable)?;
    Ok(session_base_dir(home_dir))
}

fn session_file_path(session_dir: &Path, vwacp_record_id: &str) -> PathBuf {
    session_dir.join(format!("{}.json", percent_encode_component(vwacp_record_id)))
}

async fn ensure_session_dir() -> SessionRepositoryResult<PathBuf> {
    let session_dir = default_repository_dir()?;
    fs::create_dir_all(&session_dir).await?;
    Ok(session_dir)
}

async fn load_record_from_index_entry(
    session_dir: &Path,
    entry: &SessionIndexEntry,
) -> Option<SessionRecord> {
    let payload = fs::read_to_string(session_dir.join(&entry.file)).await.ok()?;
    let value = serde_json::from_str::<Value>(&payload).ok()?;
    crate::parse_session_record(&value)
}

async fn load_session_index_entries(
    session_dir: &Path,
) -> SessionRepositoryResult<Vec<SessionIndexEntry>> {
    let index = measure_perf("session.index_load", || async {
        load_or_rebuild_session_index(session_dir).await
    })
    .await?;
    Ok(index.entries)
}

fn matches_session_entry(
    session: &SessionIndexEntry,
    normalized_cwd: &Path,
    normalized_name: Option<&str>,
    include_closed: bool,
) -> bool {
    if Path::new(&session.cwd) != normalized_cwd {
        return false;
    }
    if !include_closed && session.closed {
        return false;
    }
    match normalized_name {
        Some(name) => session.name.as_deref() == Some(name),
        None => session.name.is_none(),
    }
}

fn lexical_absolute(path: &Path) -> PathBuf {
    let base = if path.is_absolute() {
        PathBuf::new()
    } else {
        env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
    };

    let mut normalized = if path.is_absolute() { PathBuf::new() } else { base };

    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                normalized.pop();
            }
            Component::RootDir | Component::Prefix(_) => {
                normalized.push(component.as_os_str());
            }
            Component::Normal(part) => normalized.push(part),
        }
    }

    normalized
}

fn has_git_directory(dir: &Path) -> bool {
    dir.join(".git").is_dir()
}

fn is_within_boundary(boundary: &Path, target: &Path) -> bool {
    let Ok(relative) = target.strip_prefix(boundary) else {
        return false;
    };
    relative.as_os_str().is_empty() || !relative.starts_with("..")
}

pub fn absolute_path(value: &str) -> PathBuf {
    lexical_absolute(Path::new(value))
}

pub fn find_git_repository_root(start_dir: &str) -> Option<PathBuf> {
    let mut current = absolute_path(start_dir);
    let root = current.ancestors().last()?.to_path_buf();

    loop {
        if has_git_directory(&current) {
            return Some(current);
        }
        if current == root {
            return None;
        }
        let parent = current.parent()?.to_path_buf();
        if parent == current {
            return None;
        }
        current = parent;
    }
}

pub fn normalize_name(value: Option<&str>) -> Option<String> {
    value.and_then(|value| {
        let trimmed = value.trim();
        if trimmed.is_empty() { None } else { Some(trimmed.to_string()) }
    })
}

pub fn iso_now() -> String {
    time::OffsetDateTime::now_utc()
        .format(&time::format_description::well_known::Rfc3339)
        .unwrap_or_else(|_| "1970-01-01T00:00:00Z".to_string())
}

pub async fn write_session_record(record: &SessionRecord) -> SessionRepositoryResult<()> {
    measure_perf("session.write_record", || async {
        let session_dir = ensure_session_dir().await?;
        let persisted = serialize_session_record_for_disk(record);
        assert_persisted_key_policy(&persisted)?;

        let file_path = session_file_path(&session_dir, &record.vwacp_record_id);
        let temp_file =
            format!("{}.{}.{}.tmp", file_path.display(), std::process::id(), temp_suffix());
        let payload = serde_json::to_vec_pretty(&persisted)?;
        fs::write(&temp_file, [&payload[..], b"\n"].concat()).await?;
        fs::rename(temp_file, &file_path).await?;

        let mut index = load_or_rebuild_session_index(&session_dir).await?;
        let file_name =
            file_path.file_name().map(|value| value.to_string_lossy().into_owned()).ok_or_else(
                || io::Error::new(io::ErrorKind::InvalidInput, "invalid session file path"),
            )?;
        index.entries.retain(|entry| entry.file != file_name);
        index.entries.push(to_session_index_entry(record, &file_name));
        index.files.retain(|entry| entry != &file_name);
        index.files.push(file_name);
        write_session_index(&session_dir, &index).await?;
        Ok(())
    })
    .await
}

pub async fn resolve_session_record(session_id: &str) -> SessionRepositoryResult<SessionRecord> {
    let session_dir = ensure_session_dir().await?;

    let direct_path = session_file_path(&session_dir, session_id);
    if let Ok(direct_payload) =
        measure_perf("session.resolve_direct", || async { fs::read_to_string(&direct_path).await })
            .await
        && let Ok(value) = serde_json::from_str::<Value>(&direct_payload)
        && let Some(record) = crate::parse_session_record(&value)
    {
        return Ok(record);
    }

    let entries = load_session_index_entries(&session_dir).await?;
    let exact_entries = entries
        .iter()
        .filter(|entry| entry.vwacp_record_id == session_id || entry.acp_session_id == session_id)
        .cloned()
        .collect::<Vec<_>>();
    let exact_records = futures_from_entries(&session_dir, &exact_entries).await;
    if exact_records.len() == 1 {
        return Ok(exact_records.into_iter().next().unwrap());
    }
    if exact_records.len() > 1 {
        return Err(SessionResolutionError::new(format!(
            "Multiple sessions match id: {session_id}"
        ))
        .into());
    }

    let suffix_entries = entries
        .iter()
        .filter(|entry| {
            entry.vwacp_record_id.ends_with(session_id)
                || entry.acp_session_id.ends_with(session_id)
        })
        .cloned()
        .collect::<Vec<_>>();
    let suffix_records = futures_from_entries(&session_dir, &suffix_entries).await;
    if suffix_records.len() == 1 {
        return Ok(suffix_records.into_iter().next().unwrap());
    }
    if suffix_records.len() > 1 {
        return Err(
            SessionResolutionError::new(format!("Session id is ambiguous: {session_id}")).into()
        );
    }

    Err(SessionNotFoundError::new(session_id).into())
}

async fn futures_from_entries(
    session_dir: &Path,
    entries: &[SessionIndexEntry],
) -> Vec<SessionRecord> {
    let mut records = Vec::new();
    for entry in entries {
        if let Some(record) = load_record_from_index_entry(session_dir, entry).await {
            records.push(record);
        }
    }
    records
}

pub async fn list_sessions() -> SessionRepositoryResult<Vec<SessionRecord>> {
    let session_dir = ensure_session_dir().await?;
    let entries = load_session_index_entries(&session_dir).await?;
    let mut records = Vec::new();
    for entry in &entries {
        if let Some(record) = load_record_from_index_entry(&session_dir, entry).await {
            records.push(record);
        }
    }
    records.sort_by(|left, right| right.last_used_at.cmp(&left.last_used_at));
    Ok(records)
}

pub async fn list_sessions_for_agent(
    agent_command: &str,
) -> SessionRepositoryResult<Vec<SessionRecord>> {
    let session_dir = ensure_session_dir().await?;
    let entries = load_session_index_entries(&session_dir)
        .await?
        .into_iter()
        .filter(|entry| entry.agent_command == agent_command)
        .collect::<Vec<_>>();
    let mut records = futures_from_entries(&session_dir, &entries).await;
    records.sort_by(|left, right| right.last_used_at.cmp(&left.last_used_at));
    Ok(records)
}

pub async fn find_session(
    options: &FindSessionOptions,
) -> SessionRepositoryResult<Option<SessionRecord>> {
    let session_dir = ensure_session_dir().await?;
    let normalized_cwd = absolute_path(&options.cwd);
    let normalized_name = normalize_name(options.name.as_deref());
    let entries = load_session_index_entries(&session_dir).await?;
    let Some(entry) = entries.into_iter().find(|entry| {
        entry.agent_command == options.agent_command
            && matches_session_entry(
                entry,
                &normalized_cwd,
                normalized_name.as_deref(),
                options.include_closed,
            )
    }) else {
        return Ok(None);
    };

    Ok(load_record_from_index_entry(&session_dir, &entry).await)
}

pub async fn find_session_by_directory_walk(
    options: &FindSessionByDirectoryWalkOptions,
) -> SessionRepositoryResult<Option<SessionRecord>> {
    let session_dir = ensure_session_dir().await?;
    let normalized_name = normalize_name(options.name.as_deref());
    let normalized_start = absolute_path(&options.cwd);
    let requested_boundary =
        options.boundary.as_deref().map(absolute_path).unwrap_or_else(|| normalized_start.clone());
    let walk_boundary = if is_within_boundary(&requested_boundary, &normalized_start) {
        requested_boundary
    } else {
        normalized_start.clone()
    };

    let sessions = load_session_index_entries(&session_dir)
        .await?
        .into_iter()
        .filter(|entry| entry.agent_command == options.agent_command)
        .collect::<Vec<_>>();

    let mut current = normalized_start.clone();
    let walk_root = current.ancestors().last().map(Path::to_path_buf).unwrap_or(current.clone());

    loop {
        if let Some(entry) = sessions
            .iter()
            .find(|entry| matches_session_entry(entry, &current, normalized_name.as_deref(), false))
        {
            return Ok(load_record_from_index_entry(&session_dir, entry).await);
        }

        if current == walk_boundary || current == walk_root {
            return Ok(None);
        }

        let Some(parent) = current.parent().map(Path::to_path_buf) else {
            return Ok(None);
        };
        if parent == current {
            return Ok(None);
        }
        current = parent;
        if !is_within_boundary(&walk_boundary, &current) {
            return Ok(None);
        }
    }
}

pub async fn close_session(id: &str) -> SessionRepositoryResult<SessionRecord> {
    let mut record = resolve_session_record(id).await?;
    let now = iso_now();

    if let Some(pid) = record.pid {
        let _ = terminate_process(pid).await;
    }

    record.closed = Some(true);
    record.closed_at = Some(now.clone());
    record.pid = None;
    record.last_used_at = now.clone();
    if record.last_prompt_at.is_none() {
        record.last_prompt_at = Some(now);
    }

    write_session_record(&record).await?;
    if let Ok(session_dir) = default_repository_dir() {
        let _ = rebuild_session_index(&session_dir).await;
    }
    Ok(record)
}
