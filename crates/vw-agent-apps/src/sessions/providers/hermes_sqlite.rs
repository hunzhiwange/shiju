//! Read-only Hermes state.db history, resolved on the selected execution host.
use std::collections::{hash_map::Entry, HashMap, HashSet};
use std::path::Path;
use rusqlite::{Connection, OpenFlags};
use serde_json::Value;
use vw_api_types::external_session::{ContentRef, SessionBlock, ToolStatus, SessionMessage, SessionMeta, ExternalSessionIdentity, ExternalSessionId, SessionProvider, SessionSource, SessionCapabilities};
use crate::sessions::blocks::{openai_tool_calls, thinking_block, tool_result_block};
use crate::sessions::utils::{extract_text, truncate_summary};
const SQLITE_SCAN_LIMIT: usize = 500;
const TITLE_MAX_CHARS: usize = 80;
const CONTENT_JSON_PREFIX: &str = "\u{0}json:";

/// The cap is explicit so directory callers can disclose incomplete discovery.
pub struct SessionScan {
    pub sessions: Vec<SessionMeta>,
    pub sqlite_truncated: bool,
}

fn open_database(root: &Path) -> Result<Option<Connection>, String> {
    if !root.is_absolute() { return Err("Hermes instance root must be absolute".into()); }
    let path = root.join("state.db");
    let meta = match std::fs::symlink_metadata(&path) {
        Ok(meta) => meta,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(_) => return Err("Cannot inspect Hermes database".into()),
    };
    if !meta.is_file() || meta.file_type().is_symlink() { return Err("Invalid Hermes database".into()); }
    Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX)
        .map(Some).map_err(|_| "Cannot open Hermes database".into())
}

/// SQLite wins over JSONL for the same native ID. No WSL subprocess is used.
pub fn scan_sessions(root: &Path, identity: &ExternalSessionIdentity) -> Result<SessionScan, String> {
    if identity.provider != SessionProvider::Hermes { return Err("Expected Hermes identity".into()); }
    let mut sessions = Vec::new();
    let mut sqlite_truncated = false;
    if let Some(conn) = open_database(root)? {
        let count: u64 = conn.query_row("SELECT count(*) FROM sessions", [], |row| row.get(0))
            .map_err(|_| "Cannot count Hermes sessions")?;
        sqlite_truncated = count > SQLITE_SCAN_LIMIT as u64;
        let columns = get_table_columns(&conn, "sessions");
        let first = first_user_messages(&conn);
        let last = last_messages(&conn);
        let mut stmt = conn.prepare("SELECT * FROM sessions ORDER BY rowid DESC LIMIT 500")
            .map_err(|_| "Cannot prepare Hermes sessions")?;
        let rows = stmt.query_map([], |row| Ok(row_to_json(row, &columns)))
            .map_err(|_| "Cannot read Hermes sessions")?;
        for row in rows {
            let row = row.map_err(|_| "Cannot decode Hermes session")?;
            let Some(id) = row["id"].as_str() else { return Err("Invalid Hermes session ID".into()); };
            let get = |name: &str| row[name].as_str().filter(|s| !s.is_empty()).map(str::to_owned);
            let created_at = row.get("started_at").or_else(|| row.get("created_at")).and_then(timestamp_value);
            let last_active_at = row.get("last_activity_at").filter(|v| !v.is_null()).or_else(|| row.get("ended_at")).or_else(|| row.get("updated_at")).and_then(timestamp_value).or(created_at);
            let title = get("title").or_else(|| first.get(id).cloned()).map(|s| truncate_summary(&s, TITLE_MAX_CHARS));
            sessions.push(SessionMeta {
                identity: ExternalSessionIdentity { session_id: ExternalSessionId(id.to_owned()), source: SessionSource::Sqlite { database: "state.db".into(), record_id: id.to_owned() }, ..identity.clone() },
                capabilities: SessionCapabilities { read: true, delete: true, resume: false, export_markdown: true },
                title, summary: last.get(id).or_else(|| first.get(id)).map(|s| truncate_summary(s, 160)),
                project_dir: get("cwd").or_else(|| get("directory")), created_at, last_active_at,
                source_path: Some(format!("sqlite:{}#{id}", root.join("state.db").display())),
            });
        }
    }
    let ids: HashSet<_> = sessions.iter().map(|m| m.identity.session_id.clone()).collect();
    for session in super::hermes::scan_sessions(root, identity).map_err(|_| "Cannot read Hermes transcripts")? {
        if !ids.contains(&session.identity.session_id) { sessions.push(session); }
    }
    Ok(SessionScan { sessions, sqlite_truncated })
}

pub fn load_messages(root: &Path, session_id: &str) -> Result<Vec<SessionMessage>, String> {
    if session_id.is_empty() { return Err("Missing Hermes session ID".into()); }
    let conn = open_database(root)?.ok_or("Hermes database not found")?;
    load_messages_from_conn(&conn, session_id)
}
fn timestamp_value(value: &Value) -> Option<i64> {
    if let Some(n) = value.as_f64() {
        if n > 1_000_000_000_000.0 { return Some(n as i64); }
        return timestamp_secs_to_ms(n);
    }
    chrono::DateTime::parse_from_rfc3339(value.as_str()?).ok().map(|v| v.timestamp_millis())
}
fn first_user_messages(conn: &Connection) -> HashMap<String, String> {
    edge_messages(conn, "MIN", "role = 'user'")
}

/// Last displayable user / assistant message of each listed session: what the
/// list shows after "Last:", same as the other apps' session lists.
fn last_messages(conn: &Connection) -> HashMap<String, String> {
    edge_messages(
        conn,
        "MAX",
        "role IN ('user', 'assistant') AND content IS NOT NULL AND content != ''",
    )
}

fn edge_messages(conn: &Connection, pick: &str, role_filter: &str) -> HashMap<String, String> {
    let columns = get_table_columns(conn, "messages");
    if columns.is_empty() {
        return HashMap::new();
    }
    let query = format!(
        "SELECT m.session_id, m.content FROM messages m \
         JOIN (SELECT session_id, {pick}(id) AS edge_id FROM messages \
               WHERE {role_filter}{filter} \
                 AND session_id IN (SELECT id FROM sessions ORDER BY rowid DESC LIMIT {SQLITE_SCAN_LIMIT}) \
               GROUP BY session_id) f ON m.id = f.edge_id",
        filter = display_filter(&columns),
    );
    let mut stmt = match conn.prepare(&query) {
        Ok(s) => s,
        Err(_) => return HashMap::new(),
    };
    let rows = match stmt.query_map([], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, Option<String>>(1)?))
    }) {
        Ok(r) => r,
        Err(_) => return HashMap::new(),
    };
    rows.flatten()
        .filter_map(|(session_id, content)| {
            let text = decode_content(content.as_deref()?);
            (!text.trim().is_empty()).then_some((session_id, text))
        })
        .collect()
}

/// Rows Hermes itself shows in a transcript: live rows plus rows archived by
/// context compression (`compacted = 1`), but not rows hidden by
/// undo/rewind/regenerate (`active = 0, compacted = 0`). Older stores predate
/// these columns and show everything.
fn display_filter(columns: &[String]) -> &'static str {
    let has = |name: &str| columns.iter().any(|c| c == name);
    match (has("active"), has("compacted")) {
        (true, true) => " AND (active = 1 OR compacted = 1)",
        (true, false) => " AND active = 1",
        _ => "",
    }
}

/// Undo `SessionDB._encode_content`: structured content becomes its text parts.
fn decode_content(raw: &str) -> String {
    match raw.strip_prefix(CONTENT_JSON_PREFIX) {
        Some(json) => match serde_json::from_str::<Value>(json) {
            Ok(value) => extract_text(&value),
            Err(_) => raw.to_string(),
        },
        None => raw.to_string(),
    }
}

fn get_table_columns(conn: &Connection, table: &str) -> Vec<String> {
    let query = format!("PRAGMA table_info({table})");
    let mut stmt = match conn.prepare(&query) {
        Ok(s) => s,
        Err(_) => return Vec::new(),
    };
    let rows = match stmt.query_map([], |row| {
        let name: String = row.get(1)?;
        Ok(name)
    }) {
        Ok(r) => r,
        Err(_) => return Vec::new(),
    };
    rows.flatten().collect()
}

/// Convert a SQLite row to a JSON Value using known column names.
fn row_to_json(row: &rusqlite::Row, columns: &[String]) -> Value {
    let mut map = serde_json::Map::new();
    for (i, col) in columns.iter().enumerate() {
        // Try string first, then integer, then float, then null
        if let Ok(val) = row.get::<_, String>(i) {
            map.insert(col.clone(), Value::String(val));
        } else if let Ok(val) = row.get::<_, i64>(i) {
            map.insert(col.clone(), Value::Number(val.into()));
        } else if let Ok(val) = row.get::<_, f64>(i) {
            if let Some(n) = serde_json::Number::from_f64(val) {
                map.insert(col.clone(), Value::Number(n));
            }
        } else {
            map.insert(col.clone(), Value::Null);
        }
    }
    Value::Object(map)
}

type DedupeKey = (
    String,
    Option<String>,
    Option<u64>,
    Option<String>,
    Option<String>,
    Option<String>,
);

struct MessageRow {
    id: i64,
    role: String,
    content: Option<String>,
    timestamp: Option<f64>,
    tool_call_id: Option<String>,
    tool_calls: Option<String>,
    tool_name: Option<String>,
    active: i64,
    display_metadata: Option<String>,
    /// 助手的推理文本（较新的 Hermes 才有该列，待核实）
    reasoning: Option<String>,
}

impl MessageRow {
    /// Hermes' display identity (`SessionDB._display_dedupe_key`): compression
    /// re-inserts protected head/tail rows with their original timestamp, so
    /// the archived original and the live copy are the same message.
    fn dedupe_key(&self) -> DedupeKey {
        (
            self.role.clone(),
            self.content.clone(),
            self.timestamp.map(f64::to_bits),
            self.tool_call_id.clone(),
            self.tool_calls.clone(),
            self.tool_name.clone(),
        )
    }

    /// Rows the model reads but nobody typed as one message (micro-compaction
    /// merges) carry `display_metadata.model_only`; their sources stay visible.
    fn is_model_only(&self) -> bool {
        let Some(raw) = self.display_metadata.as_deref() else {
            return false;
        };
        let mut meta: Value = Value::String(raw.to_string());
        // Pre-guard rows are double-encoded.
        for _ in 0..2 {
            if let Value::String(s) = &meta {
                meta = match serde_json::from_str(s) {
                    Ok(v) => v,
                    Err(_) => return false,
                };
            }
        }
        match meta.get("model_only") {
            Some(Value::Bool(b)) => *b,
            Some(Value::Number(n)) => n.as_f64() != Some(0.0),
            Some(Value::String(s)) => !s.is_empty(),
            _ => false,
        }
    }
}

fn load_messages_from_conn(
    conn: &Connection,
    session_id: &str,
) -> Result<Vec<SessionMessage>, String> {
    let columns = get_table_columns(conn, "messages");
    let col = |name: &str| {
        if columns.iter().any(|c| c == name) {
            name.to_string()
        } else {
            format!("NULL AS {name}")
        }
    };
    // Insertion order, as Hermes reads it: timestamps can regress (clock skew,
    // compaction re-inserting rows with their original time).
    let query = format!(
        "SELECT id, role, content, {timestamp}, {tool_call_id}, {tool_calls}, {tool_name}, \
                {active}, {display_metadata}, {reasoning} \
         FROM messages WHERE session_id = ?1{filter} ORDER BY id ASC",
        timestamp = col("timestamp"),
        tool_call_id = col("tool_call_id"),
        tool_calls = col("tool_calls"),
        tool_name = col("tool_name"),
        active = col("active"),
        display_metadata = col("display_metadata"),
        reasoning = col("reasoning"),
        filter = display_filter(&columns),
    );

    let mut stmt = conn
        .prepare(&query)
        .map_err(|_| "Cannot read Hermes messages".to_string())?;

    let rows = stmt
        .query_map([session_id], |row| {
            Ok(MessageRow {
                id: row.get(0)?,
                role: row.get(1)?,
                content: row.get(2).ok().flatten(),
                timestamp: row.get(3).ok().flatten(),
                tool_call_id: row.get(4).ok().flatten(),
                tool_calls: row.get(5).ok().flatten(),
                tool_name: row.get(6).ok().flatten(),
                active: row.get::<_, Option<i64>>(7).ok().flatten().unwrap_or(1),
                display_metadata: row.get(8).ok().flatten(),
                reasoning: row.get(9).ok().flatten(),
            })
        })
        .map_err(|_| "Cannot read Hermes messages".to_string())?;

    let rows = rows.collect::<Result<Vec<_>, _>>().map_err(|_| "Cannot decode Hermes message")?;
    messages_from_rows(rows)
}

fn messages_from_rows(
    rows: impl IntoIterator<Item = MessageRow>,
) -> Result<Vec<SessionMessage>, String> {
    // Collapse duplicates the way Hermes' display projection does: keep the
    // first position, show the most live copy.
    let mut order: Vec<MessageRow> = Vec::new();
    let mut index: HashMap<DedupeKey, usize> = HashMap::new();
    for row in rows {
        if row.is_model_only() {
            continue;
        }
        match index.entry(row.dedupe_key()) {
            Entry::Occupied(slot) => {
                let kept = &mut order[*slot.get()];
                if (row.active, row.id) > (kept.active, kept.id) {
                    *kept = row;
                }
            }
            Entry::Vacant(slot) => {
                slot.insert(order.len());
                order.push(row);
            }
        }
    }

    let mut messages = Vec::new();
    for row in order {
        let ts = row.timestamp.and_then(timestamp_secs_to_ms);
        let raw_content = row.content.as_deref().unwrap_or_default();
        let text = decode_content(raw_content);
        let blocks = match row.role.as_str() {
            // 工具输出：按 tool_call_id 配对；Hermes 不记录成败 → Unknown
            "tool" => {
                // 结构化编码（`\0json:` 前缀）的内容取整列没有意义，只给预览
                let plain = !raw_content.starts_with(CONTENT_JSON_PREFIX);
                vec![tool_result_block(
                    row.tool_call_id.clone().unwrap_or_default(),
                    ToolStatus::Unknown,
                    &text,
                    || plain.then(|| messages_cell(row.id, "content", String::new())),
                )]
            }
            // 助手：推理 → 正文 → tool_calls（OpenAI 形状）
            "assistant" => {
                let mut blocks = Vec::new();
                if let Some(reasoning) = row.reasoning.as_deref().filter(|r| !r.trim().is_empty()) {
                    blocks.push(thinking_block(reasoning, None, None, || {
                        Some(messages_cell(row.id, "reasoning", String::new()))
                    }));
                }
                if !text.trim().is_empty() {
                    blocks.push(SessionBlock::text(text));
                }
                let calls = row
                    .tool_calls
                    .as_deref()
                    .and_then(|raw| serde_json::from_str::<Value>(raw).ok());
                blocks.extend(openai_tool_calls(calls.as_ref(), |pointer| {
                    Some(messages_cell(row.id, "tool_calls", pointer))
                }));
                blocks
            }
            _ if text.trim().is_empty() => Vec::new(),
            _ => vec![SessionBlock::text(text)],
        };
        let mut message = SessionMessage::from_blocks(row.role.clone(), ts, blocks);
        if message.is_empty() {
            continue;
        }
        message.id = Some(row.id.to_string());
        messages.push(message);
    }

    assign_turn_ids(&mut messages);
    Ok(messages)
}

/// `messages` 表某行某列的引用（`content::sqlite_allowed` 白名单内的列）
fn messages_cell(id: i64, column: &str, pointer: String) -> ContentRef {
    ContentRef::Sqlite {
        table: "messages".into(),
        id: id.to_string(),
        column: column.into(),
        pointer,
    }
}

/// Hermes stores timestamps as Unix epoch seconds (REAL).
fn timestamp_secs_to_ms(secs: f64) -> Option<i64> {
    (secs.is_finite() && secs > 0.0).then(|| (secs * 1000.0).round() as i64)
}


fn assign_turn_ids(messages: &mut [SessionMessage]) {
    let mut turn = 0u32;
    for message in messages {
        if message.role == "user" && !message.injected && message.blocks.iter().any(|b| matches!(b, SessionBlock::Text { .. })) { turn = turn.saturating_add(1); }
        message.turn_id = Some(format!("t{turn}"));
    }
}
