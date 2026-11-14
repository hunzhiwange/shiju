//! Lazy SQLite text reads bound to an authorized application's session record.
use rusqlite::{Connection, OpenFlags, params, types::ValueRef};
use serde_json::Value;
use vw_api_types::external_session::{ContentRef, SessionProvider};

use super::{
    content_file::{BlockContent, MAX_TEXT_BYTES, extract_text, paginate, safe_join},
    source::ValidatedSource,
};

const STALE: &str = "会话已更新，请重新读取";
const UNSUPPORTED: &str = "不支持的内容引用";
const TOO_LARGE: &str = "内容超过32MiB";
const NOT_TEXT: &str = "引用的内容不是文本";

/// Identifiers cannot be bound in SQLite. Only these literal identifiers may be
/// interpolated; record IDs and the session boundary are always SQL parameters.
fn sqlite_allowed(provider: SessionProvider, table: &str, column: &str) -> bool {
    match provider {
        SessionProvider::Opencode => {
            matches!(table, "part" | "message" | "session_message")
                && matches!(column, "data" | "content")
        }
        SessionProvider::Hermes => {
            table == "messages" && matches!(column, "content" | "reasoning" | "tool_calls")
        }
        _ => false,
    }
}

pub fn resolve_content_ref(source: &ValidatedSource, content: &ContentRef) -> Result<String, String> {
    let ContentRef::Sqlite { table, id, column, pointer } = content else {
        return Err(UNSUPPORTED.into());
    };
    if !sqlite_allowed(source.identity().provider, table, column) || id.is_empty() {
        return Err(UNSUPPORTED.into());
    }
    let session_id = source.record_id().ok_or(UNSUPPORTED)?;
    // Recheck containment and symlinks immediately before opening. The database
    // comes solely from the server-owned registry, never a content reference.
    let relative = source.path().strip_prefix(source.root()).map_err(|_| STALE)?;
    let path = safe_join(source.root(), relative.to_str().ok_or(STALE)?)?;
    if path != source.path() { return Err(STALE.into()); }
    let conn = Connection::open_with_flags(
        path, OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    ).map_err(|_| "无法打开会话数据库")?;
    // A CASE guard avoids copying an oversized cell into Rust before checking
    // its length. Size and value are observed in the same SQLite statement.
    let sql = format!(
        "SELECT length(CAST({column} AS BLOB)), \
         CASE WHEN length(CAST({column} AS BLOB)) <= ?3 THEN {column} ELSE NULL END \
         FROM {table} WHERE id = ?1 AND session_id = ?2"
    );
    let mut stmt = conn.prepare(&sql).map_err(|_| STALE)?;
    let mut rows = stmt.query(params![id, session_id, MAX_TEXT_BYTES as i64]).map_err(|_| STALE)?;
    let row = rows.next().map_err(|_| STALE)?.ok_or(STALE)?;
    let size: Option<i64> = row.get(0).map_err(|_| STALE)?;
    if size.is_some_and(|size| size < 0 || size as u64 > MAX_TEXT_BYTES) {
        return Err(TOO_LARGE.into());
    }
    let raw = match row.get_ref(1).map_err(|_| STALE)? {
        ValueRef::Text(bytes) | ValueRef::Blob(bytes) => {
            if bytes.len() as u64 > MAX_TEXT_BYTES { return Err(TOO_LARGE.into()); }
            std::str::from_utf8(bytes).map_err(|_| NOT_TEXT)?.to_owned()
        }
        _ => return Err(NOT_TEXT.into()),
    };
    // A malformed database with duplicate IDs must not ambiguously select a row.
    if rows.next().map_err(|_| STALE)?.is_some() { return Err(STALE.into()); }
    let text = if pointer.is_empty() {
        raw
    } else {
        let value: Value = serde_json::from_str(&raw).map_err(|_| STALE)?;
        extract_text(&value, pointer)?
    };
    if text.len() as u64 > MAX_TEXT_BYTES { return Err(TOO_LARGE.into()); }
    Ok(text)
}

pub fn read_block_content(
    source: &ValidatedSource,
    content: &ContentRef,
    offset: Option<u32>,
    limit: Option<u32>,
) -> Result<BlockContent, String> {
    paginate(&resolve_content_ref(source, content)?, offset, limit)
}
