//! Read-only MCode runtime history. Paths and environment belong to the trusted host.
use std::path::{Component, Path, PathBuf};
use rusqlite::{Connection, OpenFlags};
use serde_json::Value;
use vw_api_types::external_session::*;

const DATABASE: &str = "v2/sqlite/runtime-state.sqlite";

/// Resolve the selected host's environment explicitly; never consult gateway HOME.
pub fn data_dir(home: &Path, minimax: Option<&str>, mavis: Option<&str>) -> Result<PathBuf, String> {
    let root = [minimax, mavis].into_iter().flatten().map(str::trim)
        .find(|value| !value.is_empty()).map(PathBuf::from)
        .unwrap_or_else(|| home.join(".minimax"));
    if !root.is_absolute() || root.components().any(|c| matches!(c, Component::ParentDir)) {
        return Err("MCode data directory must be an absolute host path".into());
    }
    Ok(root)
}

pub fn database_path(root: &Path) -> Result<PathBuf, String> {
    if !root.is_absolute() || root.components().any(|c| matches!(c, Component::ParentDir)) {
        return Err("Invalid MCode data directory".into());
    }
    let path = root.join(DATABASE);
    let mut current = PathBuf::new();
    for component in path.components() {
        current.push(component);
        match std::fs::symlink_metadata(&current) {
            Ok(meta) if meta.file_type().is_symlink() => return Err("MCode database path contains a symbolic link".into()),
            Ok(_) => {},
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => break,
            Err(_) => return Err("Cannot inspect MCode database path".into()),
        }
    }
    Ok(path)
}

pub(crate) fn open_database(root: &Path) -> Result<Option<Connection>, String> {
    let path = database_path(root)?;
    match std::fs::metadata(&path) {
        Ok(meta) if !meta.is_file() => return Err("Invalid MCode database".into()),
        Ok(_) => {},
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(_) => return Err("Cannot inspect MCode database".into()),
    }
    Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY)
        .map(Some).map_err(|_| "Cannot open MCode database".into())
}

pub fn scan_sessions(root: &Path, identity: &ExternalSessionIdentity) -> Result<Vec<SessionMeta>, String> {
    if identity.provider != SessionProvider::Mcode { return Err("Expected MCode identity".into()); }
    let Some(conn) = open_database(root)? else { return Ok(Vec::new()); };
    let mut query = conn.prepare(
        "SELECT session_id, title, workspace_dir, created_at_ms, updated_at_ms
         FROM local_runtime_sessions WHERE visibility <> 'hidden' AND archived = 0
         AND parent_session_id IS NULL AND session_kind NOT IN ('peek', 'channel', 'cron')
         ORDER BY updated_at_ms DESC",
    ).map_err(|_| "Cannot prepare MCode sessions")?;
    let rows = query.query_map([], |row| {
        let id: String = row.get(0)?;
        Ok(SessionMeta {
            identity: ExternalSessionIdentity {
                session_id: ExternalSessionId(id.clone()),
                source: SessionSource::Sqlite { database: DATABASE.into(), record_id: id.clone() },
                ..identity.clone()
            },
            capabilities: SessionCapabilities { read: true, delete: false,
                resume: !id.is_empty() && id.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-')),
                export_markdown: true },
            source_path: Some(format!("mcode:{id}")),
            title: row.get(1)?, summary: None, project_dir: row.get(2)?,
            created_at: row.get(3)?, last_active_at: row.get(4)?,
        })
    }).map_err(|_| "Cannot read MCode sessions")?;
    rows.collect::<rusqlite::Result<Vec<_>>>().map_err(|_| "Cannot decode MCode sessions".into())
}

pub fn load_messages(root: &Path, session_id: &str) -> Result<Vec<SessionMessage>, String> {
    if session_id.is_empty() { return Err("Missing MCode session ID".into()); }
    let conn = open_database(root)?.ok_or("MCode database not found")?;
    read_messages(&conn, session_id).map_err(|_| "Cannot read MCode messages".into())
}

/// MCode only exposes a display projection; deletion is not supported.
pub fn delete_session(_root: &Path, _session_id: &str) -> Result<(), String> {
    Err("MCode session deletion is unsupported".into())
}

fn read_messages(conn: &Connection, id: &str) -> rusqlite::Result<Vec<SessionMessage>> {
    let mut query = conn.prepare(
        "WITH migrated AS (
            SELECT 1 FROM local_runtime_message_row_migrations WHERE session_id = ?1
         ), display AS (
            SELECT role, data_json, created_at_ms, id AS sequence
            FROM local_runtime_message_rows
            WHERE session_id = ?1 AND EXISTS (SELECT 1 FROM migrated)
            UNION ALL
            SELECT json_extract(message.value, '$.role'), message.value, NULL, message.key
            FROM local_runtime_messages, json_each(display_messages_json) AS message
            WHERE session_id = ?1 AND NOT EXISTS (SELECT 1 FROM migrated)
         )
         SELECT role, data_json, created_at_ms FROM display
         WHERE role IN ('user', 'assistant') ORDER BY sequence",
    )?;
    let rows = query.query_map([id], |row| {
        let data: String = row.get(1)?;
        let value: Value = serde_json::from_str(&data).map_err(|error| rusqlite::Error::FromSqlConversionFailure(1, rusqlite::types::Type::Text, Box::new(error)))?;
        let ts = row.get::<_, Option<i64>>(2)?.or_else(|| {
            let time = value
                .get("timestamp")
                .filter(|v| !v.is_null())
                .or_else(|| value.get("created_at"))?;
            time.as_f64()
                .or_else(|| time.as_str()?.parse::<f64>().ok())
                .filter(|time| time.is_finite())
                .map(|time| time.floor() as i64)
        });
        // 只暴露展示行，没有执行过程：只有 Text
        let text = crate::sessions::utils::extract_text(&value["msg_content"]);
        let blocks = if text.trim().is_empty() {
            Vec::new()
        } else {
            vec![SessionBlock::text(text)]
        };
        Ok(SessionMessage::from_blocks(
            row.get::<_, String>(0)?,
            ts,
            blocks,
        ))
    })?;
    let mut messages = rows
        .filter_map(|r| match r {
            Ok(m) if m.is_empty() => None,
            other => Some(other),
        })
        .collect::<rusqlite::Result<Vec<_>>>()?;
    let mut turn = 0usize;
    for message in &mut messages {
        if message.role == "user" { turn += 1; }
        if turn > 0 { message.turn_id = Some(format!("t{turn}")); }
    }
    Ok(messages)
}

