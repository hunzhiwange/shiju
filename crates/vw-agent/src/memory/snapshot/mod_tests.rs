// 验证记忆快照的解析、恢复、去重与局部失败容忍。

// 恢复时跳过既有键和单条坏记录，导出或模式错误则保留明确失败。
use super::*;
use rusqlite::Connection;
use tempfile::TempDir;

fn create_memories_table(conn: &Connection) {
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
}

#[test]
fn parse_snapshot_skips_empty_entries_and_trims_content() {
    let input = "### 🔑 ``\n\nignored\n\n### 🔑 `empty`\n\n---\n\n### 🔑 `notes`\n\n  keep indentation\n\n*Created: x | Updated: y*\n\n---\n";

    let entries = parse_snapshot(input);

    assert_eq!(entries, vec![("notes".to_string(), "  keep indentation".to_string())]);
}

#[test]
fn hydrate_returns_zero_without_snapshot_file() {
    let tmp = TempDir::new().unwrap();

    let count = hydrate_from_snapshot(tmp.path()).unwrap();

    assert_eq!(count, 0);
}

#[test]
fn hydrate_returns_zero_for_snapshot_without_entries() {
    let tmp = TempDir::new().unwrap();
    let storage = paths::project_data_dir(tmp.path()).unwrap();
    fs::create_dir_all(&storage).unwrap();
    fs::write(snapshot_path(&storage), "# snapshot\n\n---\n").unwrap();

    let count = hydrate_from_snapshot(tmp.path()).unwrap();

    assert_eq!(count, 0);
    assert!(!storage.join("memory").join("brain.db").exists());
}

#[test]
fn hydrate_skips_keys_that_already_exist() {
    let tmp = TempDir::new().unwrap();
    let workspace = tmp.path();
    let storage = paths::project_data_dir(workspace).unwrap();
    let db_dir = storage.join("memory");
    fs::create_dir_all(&db_dir).unwrap();
    fs::write(
        snapshot_path(&storage),
        "### 🔑 `known`\n\nnew content\n\n### 🔑 `fresh`\n\nfresh content\n",
    )
    .unwrap();

    let db_path = db_dir.join("brain.db");
    let conn = Connection::open(&db_path).unwrap();
    create_memories_table(&conn);
    conn.execute(
        "INSERT INTO memories (id, key, content, category, created_at, updated_at)
         VALUES ('id1', 'known', 'old content', 'core', '2026-01-01', '2026-01-01')",
        [],
    )
    .unwrap();
    drop(conn);

    let count = hydrate_from_snapshot(workspace).unwrap();

    assert_eq!(count, 1);
    let conn = Connection::open(&db_path).unwrap();
    let known: String = conn
        .query_row("SELECT content FROM memories WHERE key = 'known'", [], |row| row.get(0))
        .unwrap();
    let total: i64 = conn.query_row("SELECT COUNT(*) FROM memories", [], |row| row.get(0)).unwrap();
    assert_eq!(known, "old content");
    assert_eq!(total, 2);
}

#[test]
fn hydrate_continues_when_an_entry_cannot_be_inserted() {
    let tmp = TempDir::new().unwrap();
    let workspace = tmp.path();
    let storage = paths::project_data_dir(workspace).unwrap();
    let db_dir = storage.join("memory");
    fs::create_dir_all(&db_dir).unwrap();
    fs::write(snapshot_path(&storage), "### 🔑 `key`\n\ncontent\n").unwrap();

    let conn = Connection::open(db_dir.join("brain.db")).unwrap();
    conn.execute_batch(
        "CREATE TABLE memories (
            id TEXT PRIMARY KEY,
            key TEXT NOT NULL UNIQUE,
            content TEXT NOT NULL,
            category TEXT NOT NULL DEFAULT 'core',
            embedding BLOB,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            required_metadata TEXT NOT NULL
        );",
    )
    .unwrap();
    drop(conn);

    let count = hydrate_from_snapshot(workspace).unwrap();

    assert_eq!(count, 0);
    let conn = Connection::open(db_dir.join("brain.db")).unwrap();
    let stored: i64 =
        conn.query_row("SELECT COUNT(*) FROM memories", [], |row| row.get(0)).unwrap();
    assert_eq!(stored, 0);
}

#[test]
fn export_returns_zero_when_core_memories_are_absent() {
    let tmp = TempDir::new().unwrap();
    let workspace = tmp.path();
    let storage = paths::project_data_dir(workspace).unwrap();
    let db_dir = storage.join("memory");
    fs::create_dir_all(&db_dir).unwrap();
    let conn = Connection::open(db_dir.join("brain.db")).unwrap();
    create_memories_table(&conn);
    conn.execute(
        "INSERT INTO memories (id, key, content, category, created_at, updated_at)
         VALUES ('id1', 'conversation', 'chat', 'conversation', '2026-01-01', '2026-01-01')",
        [],
    )
    .unwrap();
    drop(conn);

    let count = export_snapshot(workspace).unwrap();

    assert_eq!(count, 0);
    assert!(!snapshot_path(&storage).exists());
}

#[test]
fn export_reports_schema_errors() {
    let tmp = TempDir::new().unwrap();
    let storage = paths::project_data_dir(tmp.path()).unwrap();
    let db_dir = storage.join("memory");
    fs::create_dir_all(&db_dir).unwrap();
    Connection::open(db_dir.join("brain.db")).unwrap();

    let err = export_snapshot(tmp.path()).unwrap_err().to_string();

    assert!(err.contains("no such table: memories"));
}

#[test]
fn should_hydrate_treats_small_database_as_empty() {
    let tmp = TempDir::new().unwrap();
    let workspace = tmp.path();
    let storage = paths::project_data_dir(workspace).unwrap();
    let db_dir = storage.join("memory");
    fs::create_dir_all(&db_dir).unwrap();
    fs::write(snapshot_path(&storage), "### 🔑 `key`\n\ncontent\n").unwrap();
    fs::write(db_dir.join("brain.db"), b"small").unwrap();

    assert!(should_hydrate(workspace));
}
