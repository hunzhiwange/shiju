//! OpenCode SQLite history. The authorized host supplies the data root, never a wire source path.
use std::{collections::{HashMap, HashSet}, fs, path::{Path, PathBuf}, time::Duration};
use rusqlite::{Connection, OpenFlags};
use serde_json::Value;
use vw_api_types::external_session::{ExternalSessionId, ExternalSessionIdentity, SessionCapabilities, SessionMessage, SessionMeta, SessionProvider, SessionSource};
use super::opencode_blocks::{message_from_parts, PartLocator};

pub(crate) fn open(data_root: &Path) -> Result<(Connection, PathBuf), String> {
    if !data_root.is_absolute() { return Err("OpenCode data root must be absolute".into()); }
    let root = data_root.canonicalize().map_err(|_| "OpenCode data root unavailable")?;
    let path = root.join("opencode.db");
    let meta = fs::symlink_metadata(&path).map_err(|_| "OpenCode database unavailable")?;
    if !meta.is_file() || meta.file_type().is_symlink() {
        return Err("OpenCode database must be a regular file".into());
    }
    let path = path.canonicalize().map_err(|_| "OpenCode database unavailable")?;
    if !path.starts_with(&root) { return Err("OpenCode database escaped data root".into()); }
    let conn = Connection::open_with_flags(&path, OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX)
        .map_err(|_| "Cannot open OpenCode database read-only")?;
    conn.busy_timeout(Duration::from_secs(2)).map_err(|_| "Cannot configure database timeout")?;
    Ok((conn, path))
}

fn is_v2(conn: &Connection) -> bool {
    sqlite_table_exists(conn, "session_v2") && sqlite_table_exists(conn, "session_message")
}

/// Database rows take precedence over file rows within this bound application instance.
pub fn scan_sessions(data_root: &Path, identity: &ExternalSessionIdentity) -> Result<Vec<SessionMeta>, String> {
    if identity.provider != SessionProvider::Opencode { return Err("Expected OpenCode identity".into()); }
    let mut files = super::opencode::scan_sessions(&data_root.join("storage"), identity)
        .map_err(|_| "Cannot scan OpenCode file history")?;
    if !data_root.join("opencode.db").try_exists().map_err(|_| "Cannot inspect OpenCode database")? {
        return Ok(files);
    }
    let (conn, path) = open(data_root)?;
    let mut sessions = Vec::new();
    let mut queries = Vec::new();
    if is_v2(&conn) {
        queries.push(("SELECT s.id, COALESCE(s.title, ''), s.directory, s.time_created, MAX(s.time_updated, COALESCE(MAX(m.time_updated), s.time_updated)) FROM session_v2 s LEFT JOIN session_message m ON m.session_id = s.id GROUP BY s.id ORDER BY 5 DESC", None));
        if sqlite_table_exists(&conn, "session") {
            if let Some(marker) = get_v2_migration_marker_ts(&conn) {
                queries.push(("SELECT id, COALESCE(title, ''), directory, time_created, time_updated FROM session WHERE NOT EXISTS (SELECT 1 FROM session_v2 WHERE session_v2.id = session.id) AND time_updated > ?1 ORDER BY time_updated DESC", Some(marker)));
            }
        }
    } else {
        queries.push(("SELECT id, COALESCE(title, ''), directory, time_created, time_updated FROM session ORDER BY time_updated DESC", None));
    }
    for (sql, marker) in queries {
        let mut stmt = conn.prepare(sql).map_err(|_| "Unsupported OpenCode session schema")?;
        let params: Vec<i64> = marker.into_iter().collect();
        let rows = stmt.query_map(rusqlite::params_from_iter(params), |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?, row.get::<_, String>(2)?, row.get::<_, i64>(3)?, row.get::<_, i64>(4)?))
        }).map_err(|_| "Cannot query OpenCode sessions")?;
        for row in rows {
            let (id, title, directory, created, updated) = row.map_err(|_| "Invalid OpenCode session row")?;
            let title = if title.trim().is_empty() { Path::new(&directory).file_name().map(|v| v.to_string_lossy().into_owned()) } else { Some(title) };
            let mut bound = identity.clone();
            bound.session_id = ExternalSessionId(id.clone());
            bound.source = SessionSource::Sqlite { database: "opencode.db".into(), record_id: id };
            sessions.push(SessionMeta {
                identity: bound, capabilities: SessionCapabilities { read: true, delete: true, resume: true, export_markdown: true },
                summary: title.clone(), title, project_dir: if directory.is_empty() { None } else { Some(directory) },
                created_at: Some(created), last_active_at: Some(updated), source_path: Some(path.to_string_lossy().into_owned()),
            });
        }
    }
    let ids: HashSet<_> = sessions.iter().map(|s| s.identity.session_id.clone()).collect();
    files.retain(|s| !ids.contains(&s.identity.session_id));
    sessions.extend(files);
    sessions.sort_by(|a,b| b.last_active_at.cmp(&a.last_active_at).then_with(|| a.identity.session_id.0.cmp(&b.identity.session_id.0)));
    Ok(sessions)
}

pub fn load_messages(data_root: &Path, session_id: &str) -> Result<Vec<SessionMessage>, String> {
    if session_id.is_empty() || session_id.contains('\0') { return Err("Invalid OpenCode session ID".into()); }
    let (conn, _) = open(data_root)?;
    let v2 = is_v2(&conn);
    let exists: bool = conn.query_row(if v2 { "SELECT EXISTS(SELECT 1 FROM session_v2 WHERE id=?1)" } else { "SELECT EXISTS(SELECT 1 FROM session WHERE id=?1)" }, [session_id], |r| r.get(0)).map_err(|_| "Unsupported OpenCode schema")?;
    if v2 && exists { return load_messages_sqlite_v2(&conn, session_id); }
    if !sqlite_table_exists(&conn, "session") { return Err("OpenCode session not found".into()); }
    let exists: bool = conn.query_row("SELECT EXISTS(SELECT 1 FROM session WHERE id=?1)", [session_id], |r| r.get(0)).map_err(|_| "Cannot locate OpenCode session")?;
    if !exists { return Err("OpenCode session not found".into()); }
    load_messages_sqlite_v1(&conn, session_id)
}

fn sqlite_table_exists(conn: &Connection, table: &str) -> bool {
    conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = ?1)",
        [table],
        |row| row.get(0),
    )
    .unwrap_or(false)
}

/// Read the V1→V2 migration marker timestamp (`migration.v1-v2`) from the `kv` table.
fn get_v2_migration_marker_ts(conn: &Connection) -> Option<i64> {
    if !sqlite_table_exists(conn, "kv") {
        return None;
    }
    conn.query_row(
        "SELECT max(coalesce(time_created, 0), coalesce(time_updated, 0)) \
         FROM kv WHERE key = 'migration.v1-v2'",
        [],
        |row| row.get(0),
    )
    .ok()
    .filter(|&ts| ts > 0)
}

fn load_messages_sqlite_v1(
    conn: &Connection,
    session_id: &str,
) -> Result<Vec<SessionMessage>, String> {
    let mut msg_stmt = conn
        .prepare(
            "SELECT id, time_created, data FROM message WHERE session_id = ?1 \
             ORDER BY time_created ASC, id ASC",
        )
        .map_err(|e| format!("Failed to prepare message query: {e}"))?;

    let msg_rows = msg_stmt
        .query_map([session_id], |row| {
            let id: String = row.get(0)?;
            let ts: i64 = row.get(1)?;
            let data: String = row.get(2)?;
            Ok((id, ts, data))
        })
        .map_err(|e| format!("Failed to query messages: {e}"))?;

    let mut part_stmt = conn
        .prepare(
            "SELECT id, message_id, data FROM part WHERE session_id = ?1 \
             ORDER BY time_created ASC, id ASC",
        )
        .map_err(|e| format!("Failed to prepare part query: {e}"))?;

    let part_rows = part_stmt
        .query_map([session_id], |row| {
            let part_id: String = row.get(0)?;
            let message_id: String = row.get(1)?;
            let data: String = row.get(2)?;
            Ok((part_id, message_id, data))
        })
        .map_err(|e| format!("Failed to query parts: {e}"))?;

    let mut parts_map: HashMap<String, Vec<(PartLocator, Value)>> = HashMap::new();
    for row in part_rows {
        let (part_id, message_id, data) = row.map_err(|_| "Invalid OpenCode part row")?;
        let Ok(part) = serde_json::from_str::<Value>(&data) else {
            continue;
        };
        let locator = PartLocator::Sqlite {
            table: "part",
            id: part_id,
            base: String::new(),
        };
        parts_map
            .entry(message_id)
            .or_default()
            .push((locator, part));
    }

    let mut messages = Vec::new();
    for row in msg_rows {
        let (msg_id, ts, data) = row.map_err(|_| "Invalid OpenCode message row")?;
        let Ok(msg_value) = serde_json::from_str::<Value>(&data) else {
            continue;
        };
        let role = msg_value
            .get("role")
            .and_then(Value::as_str)
            .unwrap_or("unknown");
        let parts = parts_map.remove(&msg_id).unwrap_or_default();
        let message = message_from_parts(role, Some(msg_id), Some(ts), &msg_value, &parts);
        if !message.is_empty() {
            messages.push(message);
        }
    }

    Ok(messages)
}

fn load_messages_sqlite_v2(
    conn: &Connection,
    session_id: &str,
) -> Result<Vec<SessionMessage>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT id, type, time_created, data \
             FROM session_message \
             WHERE session_id = ?1 \
             ORDER BY seq ASC, rowid ASC",
        )
        .map_err(|e| format!("Failed to prepare V2 message query: {e}"))?;

    let rows = stmt
        .query_map([session_id], |row| {
            let id: String = row.get(0)?;
            let msg_type: String = row.get(1)?;
            let ts: i64 = row.get(2)?;
            let data: String = row.get(3)?;
            Ok((id, msg_type, ts, data))
        })
        .map_err(|e| format!("Failed to query V2 messages: {e}"))?;

    let mut messages = Vec::new();
    for row in rows {
        let (row_id, msg_type, ts, data) = row.map_err(|_| "Invalid OpenCode V2 message row")?;
        let Ok(msg_value) = serde_json::from_str::<Value>(&data) else {
            continue;
        };
        if !matches!(msg_type.as_str(), "user" | "assistant" | "system") {
            continue;
        }

        // user 优先取 `text`；其余取 `content[]`（结构同 v1 parts），最后退回 `text`
        let text_part = |text: &str| {
            (
                PartLocator::Sqlite {
                    table: "session_message",
                    id: row_id.clone(),
                    base: String::new(),
                },
                serde_json::json!({ "type": "text", "text": text }),
            )
        };
        let parts: Vec<(PartLocator, Value)> = match (
            msg_type.as_str(),
            msg_value.get("text").and_then(Value::as_str),
            msg_value.get("content"),
        ) {
            ("user" | "system", Some(text), _) => vec![text_part(text)],
            (_, _, Some(Value::Array(items))) if msg_type != "system" => items
                .iter()
                .enumerate()
                .map(|(i, item)| {
                    (
                        PartLocator::Sqlite {
                            table: "session_message",
                            id: row_id.clone(),
                            base: format!("/content/{i}"),
                        },
                        item.clone(),
                    )
                })
                .collect(),
            (_, _, Some(Value::String(text))) if msg_type == "user" => vec![text_part(text)],
            (_, Some(text), _) => vec![text_part(text)],
            _ => Vec::new(),
        };

        let message = message_from_parts(
            &msg_type,
            Some(row_id.clone()),
            Some(ts),
            &msg_value,
            &parts,
        );
        if !message.is_empty() {
            messages.push(message);
        }
    }

    Ok(messages)
}

