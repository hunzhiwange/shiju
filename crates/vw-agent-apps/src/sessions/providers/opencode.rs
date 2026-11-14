//! OpenCode legacy file storage. Roots are supplied by the authorized execution host.
use std::{fs, io, path::{Component, Path, PathBuf}};
use serde_json::Value;
use vw_api_types::external_session::{ExternalSessionId, ExternalSessionIdentity, SessionCapabilities, SessionMessage, SessionMeta, SessionProvider, SessionSource};
use super::opencode_blocks::{message_from_parts, PartLocator};
use crate::sessions::utils::truncate_summary;

/// OpenCode history lives in XDG data, independently of its configuration directory.
pub fn data_dir(execution_home: &Path, xdg_data_home: Option<&Path>) -> io::Result<PathBuf> {
    let base = match xdg_data_home.filter(|p| !p.as_os_str().is_empty()) {
        Some(path) => path.to_path_buf(),
        None => execution_home.join(".local/share"),
    };
    if !base.is_absolute() {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "OpenCode data root must be absolute"));
    }
    Ok(base.join("opencode/storage"))
}

fn valid_id(id: &str) -> bool {
    !id.is_empty() && id != "." && id != ".." && !id.contains(['/', '\\', '\0'])
}

/// Internal root boundary; never resolve an arbitrary client-supplied source path.
fn contained(root: &Path, path: &Path) -> io::Result<PathBuf> {
    let root = root.canonicalize()?;
    let path = path.canonicalize()?;
    if !path.starts_with(&root) {
        return Err(io::Error::new(io::ErrorKind::PermissionDenied, "OpenCode source escaped storage"));
    }
    Ok(path)
}

fn collect_json(root: &Path, dir: &Path, files: &mut Vec<PathBuf>) -> io::Result<()> {
    let meta = match fs::symlink_metadata(dir) {
        Ok(meta) => meta,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(()),
        Err(e) => return Err(e),
    };
    if meta.file_type().is_symlink() || !meta.is_dir() { return Ok(()); }
    let dir = contained(root, dir)?;
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let kind = entry.file_type()?;
        let path = entry.path();
        if kind.is_dir() { collect_json(root, &path, files)?; }
        else if kind.is_file() && path.extension().and_then(|v| v.to_str()) == Some("json") {
            files.push(path);
        }
    }
    Ok(())
}

fn read_json(root: &Path, path: &Path) -> io::Result<Value> {
    let path = contained(root, path)?;
    serde_json::from_slice(&fs::read(path)?).map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "invalid OpenCode record"))
}

pub(super) fn parse_timestamp_to_ms(value: &Value) -> Option<i64> {
    if let Some(n) = value.as_i64() { return if n > 1_000_000_000_000 { Some(n) } else { n.checked_mul(1000) }; }
    let text = value.as_str()?;
    text.parse().ok().or_else(|| chrono::DateTime::parse_from_rfc3339(text).ok().map(|v| v.timestamp_millis()))
}

pub fn scan_sessions(storage: &Path, identity: &ExternalSessionIdentity) -> io::Result<Vec<SessionMeta>> {
    if !storage.is_absolute() || identity.provider != SessionProvider::Opencode {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "expected an OpenCode data storage root"));
    }
    let mut files = Vec::new();
    collect_json(storage, &storage.join("session"), &mut files)?;
    files.sort();
    let mut sessions = Vec::new();
    for path in files {
        let value = match read_json(storage, &path) { Ok(value) => value, Err(_) => continue };
        let Some(id) = value.get("id").and_then(Value::as_str).filter(|id| valid_id(id)) else { continue };
        let directory = value.get("directory").and_then(Value::as_str).map(str::to_owned);
        let title = value.get("title").and_then(Value::as_str).filter(|s| !s.trim().is_empty()).map(str::to_owned);
        let summary = if title.is_some() { title.clone() } else {
            load_messages(storage, id).ok().and_then(|messages| messages.into_iter().find(|m| m.role == "user" && !m.content.trim().is_empty())).map(|m| truncate_summary(&m.content, 160))
        };
        let title = title.or_else(|| directory.as_deref().and_then(|p| Path::new(p).file_name()).map(|s| s.to_string_lossy().into_owned()));
        let mut bound = identity.clone();
        bound.session_id = ExternalSessionId(id.to_owned());
        bound.source = SessionSource::File { relative_path: format!("message/{id}") };
        let created_at = value.pointer("/time/created").and_then(parse_timestamp_to_ms);
        sessions.push(SessionMeta {
            identity: bound,
            capabilities: SessionCapabilities { read: true, delete: true, resume: true, export_markdown: true },
            title, summary, project_dir: directory, created_at,
            last_active_at: value.pointer("/time/updated").and_then(parse_timestamp_to_ms).or(created_at),
            source_path: Some(storage.join("message").join(id).to_string_lossy().into_owned()),
        });
    }
    sessions.sort_by(|a,b| b.last_active_at.cmp(&a.last_active_at).then_with(|| a.identity.session_id.0.cmp(&b.identity.session_id.0)));
    Ok(sessions)
}

pub fn load_messages(storage: &Path, session_id: &str) -> io::Result<Vec<SessionMessage>> {
    if !storage.is_absolute() || !valid_id(session_id) {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "invalid OpenCode session identity"));
    }
    let dir = storage.join("message").join(session_id);
    contained(storage, &dir)?;
    let mut files = Vec::new();
    collect_json(storage, &dir, &mut files)?;
    files.sort();
    let mut messages = Vec::new();
    for path in files {
        let value = match read_json(storage, &path) { Ok(value) => value, Err(_) => continue };
        let Some(id) = value.get("id").and_then(Value::as_str).filter(|id| valid_id(id)) else { continue };
        if value.get("sessionID").and_then(Value::as_str).is_some_and(|v| v != session_id) { continue; }
        let role = value.get("role").and_then(Value::as_str).unwrap_or("unknown");
        let ts = value.pointer("/time/created").and_then(parse_timestamp_to_ms);
        let mut part_files = Vec::new();
        collect_json(storage, &storage.join("part").join(id), &mut part_files)?;
        part_files.sort();
        let root = storage.canonicalize()?;
        let mut parts = Vec::new();
        for path in part_files {
            let part = match read_json(storage, &path) { Ok(part) => part, Err(_) => continue };
            if part.get("messageID").and_then(Value::as_str).is_some_and(|v| v != id) { continue; }
            let relative = path.strip_prefix(&root).map_err(io::Error::other)?;
            let rel_path = relative.components().filter_map(|c| match c { Component::Normal(s) => Some(s.to_string_lossy()), _ => None }).collect::<Vec<_>>().join("/");
            parts.push((PartLocator::File { rel_path }, part));
        }
        let message = message_from_parts(role, Some(id.to_owned()), ts, &value, &parts);
        if !message.is_empty() { messages.push(message); }
    }
    messages.sort_by(|a,b| a.ts.cmp(&b.ts).then_with(|| a.id.cmp(&b.id)));
    Ok(messages)
}
