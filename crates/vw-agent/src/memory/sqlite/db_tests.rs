// 验证 SQLite 连接超时、模式初始化及旧表迁移行为。

// 旧表迁移需补齐会话列、索引和触发器，并允许已迁移模式再次初始化。
use super::*;
use rusqlite::Connection;
use tempfile::TempDir;

#[test]
fn sqlite_open_timeout_cap_is_bounded() {
    assert_eq!(SQLITE_OPEN_TIMEOUT_CAP_SECS, 300);
}

#[test]
fn open_connection_opens_database_with_and_without_timeout() {
    let tmp = TempDir::new().unwrap();
    let db_path = tmp.path().join("brain.db");

    let conn = SqliteMemory::open_connection(&db_path, None).unwrap();
    conn.execute("CREATE TABLE t (id INTEGER)", []).unwrap();
    drop(conn);

    let conn = SqliteMemory::open_connection(&db_path, Some(1)).unwrap();
    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM sqlite_master WHERE name = 't'", [], |row| row.get(0))
        .unwrap();
    assert_eq!(count, 1);
}

#[test]
fn open_connection_reports_invalid_parent_path() {
    let tmp = TempDir::new().unwrap();
    let file_parent = tmp.path().join("not-a-dir");
    std::fs::write(&file_parent, b"file").unwrap();
    let db_path = file_parent.join("brain.db");

    let err = SqliteMemory::open_connection(&db_path, None).unwrap_err().to_string();

    assert!(err.contains("SQLite 无法打开数据库"));
}

#[test]
fn open_connection_with_timeout_reports_open_error() {
    let tmp = TempDir::new().unwrap();
    let file_parent = tmp.path().join("not-a-dir");
    std::fs::write(&file_parent, b"file").unwrap();
    let db_path = file_parent.join("brain.db");

    let err = SqliteMemory::open_connection(&db_path, Some(1)).unwrap_err().to_string();

    assert!(err.contains("SQLite 无法打开数据库"));
}

#[test]
fn init_schema_creates_tables_indexes_triggers_and_session_column() {
    let conn = Connection::open_in_memory().unwrap();

    SqliteMemory::init_schema(&conn).unwrap();

    let table_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master
             WHERE name IN ('memories', 'memories_fts', 'embedding_cache')",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(table_count, 3);

    let trigger_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master
             WHERE type = 'trigger' AND name IN ('memories_ai', 'memories_ad', 'memories_au')",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(trigger_count, 3);

    let columns = conn
        .prepare("PRAGMA table_info(memories)")
        .unwrap()
        .query_map([], |row| row.get::<_, String>(1))
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    assert!(columns.iter().any(|column| column == "session_id"));
}

#[test]
fn init_schema_migrates_legacy_memories_table() {
    let conn = Connection::open_in_memory().unwrap();
    conn.execute_batch(
        "CREATE TABLE memories (
            id TEXT PRIMARY KEY,
            key TEXT NOT NULL UNIQUE,
            content TEXT NOT NULL,
            category TEXT NOT NULL DEFAULT 'core',
            embedding BLOB,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        );",
    )
    .unwrap();

    SqliteMemory::init_schema(&conn).unwrap();

    conn.prepare("SELECT session_id FROM memories").unwrap();
}

#[test]
fn init_schema_accepts_existing_session_column_without_remigration() {
    let conn = Connection::open_in_memory().unwrap();
    conn.execute_batch(
        "CREATE TABLE memories (
            id TEXT PRIMARY KEY,
            key TEXT NOT NULL UNIQUE,
            content TEXT NOT NULL,
            category TEXT NOT NULL DEFAULT 'core',
            embedding BLOB,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            session_id TEXT
        );",
    )
    .unwrap();

    SqliteMemory::init_schema(&conn).unwrap();
    SqliteMemory::init_schema(&conn).unwrap();

    let session_columns: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM pragma_table_info('memories') WHERE name = 'session_id'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(session_columns, 1);
}
