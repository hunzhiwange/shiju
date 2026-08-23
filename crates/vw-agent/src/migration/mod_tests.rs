// 验证旧记忆数据迁移的读取、去重、路径判断与备份。
use super::*;
use rusqlite::params;

#[test]
fn sqlite_reader_handles_missing_sources_and_rejects_missing_content_column() {
    let workspace = tempfile::TempDir::new().unwrap();
    let missing = workspace.path().join("missing.db");
    assert!(read_openclaw_sqlite_entries(&missing).unwrap().is_empty());

    let no_table = workspace.path().join("no-table.db");
    Connection::open(&no_table).unwrap();
    assert!(read_openclaw_sqlite_entries(&no_table).unwrap().is_empty());

    let no_content = workspace.path().join("no-content.db");
    Connection::open(&no_content)
        .unwrap()
        .execute_batch("CREATE TABLE memories (key TEXT, category TEXT);")
        .unwrap();
    let error = read_openclaw_sqlite_entries(&no_content).unwrap_err().to_string();
    assert!(error.contains("no content-like column"));
}

#[test]
fn sqlite_reader_falls_back_for_non_text_key_and_category() {
    let workspace = tempfile::TempDir::new().unwrap();
    let db_path = workspace.path().join("legacy.db");
    let conn = Connection::open(&db_path).unwrap();
    conn.execute_batch("CREATE TABLE memories (key BLOB, content TEXT, category BLOB);").unwrap();
    conn.execute(
        "INSERT INTO memories (key, content, category) VALUES (?1, ?2, ?3)",
        params![vec![0xff_u8], "body", vec![0xfe_u8]],
    )
    .unwrap();
    drop(conn);

    let entries = read_openclaw_sqlite_entries(&db_path).unwrap();

    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].key, "openclaw_sqlite_0");
    assert_eq!(entries[0].content, "body");
    assert_eq!(entries[0].category, MemoryCategory::Core);
}

#[test]
fn collect_source_entries_deduplicates_identical_sqlite_and_markdown_entries() {
    let workspace = tempfile::TempDir::new().unwrap();
    let memory_dir = workspace.path().join("memory");
    fs::create_dir_all(&memory_dir).unwrap();
    let conn = Connection::open(memory_dir.join("brain.db")).unwrap();
    conn.execute_batch("CREATE TABLE memories (key TEXT, content TEXT, category TEXT);").unwrap();
    conn.execute(
        "INSERT INTO memories (key, content, category) VALUES (?1, ?2, ?3)",
        params!["same", "same content", "core"],
    )
    .unwrap();
    drop(conn);
    fs::write(workspace.path().join("MEMORY.md"), "**same**: same content\n").unwrap();

    let mut stats = MigrationStats::default();
    let entries = collect_source_entries(workspace.path(), &mut stats).unwrap();

    assert_eq!(stats.from_sqlite, 1);
    assert_eq!(stats.from_markdown, 1);
    assert_eq!(entries.len(), 1);
}

#[test]
fn paths_equal_handles_existing_and_missing_paths() {
    let workspace = tempfile::TempDir::new().unwrap();
    let existing = workspace.path().join("existing");
    fs::create_dir_all(&existing).unwrap();

    assert!(paths_equal(&existing, &existing.join(".")));
    assert!(paths_equal(&workspace.path().join("missing"), &workspace.path().join("missing")));
    assert!(!paths_equal(&workspace.path().join("missing-a"), &workspace.path().join("missing-b")));
}

#[test]
fn backup_copies_root_and_daily_markdown_files() {
    let workspace = tempfile::TempDir::new().unwrap();
    let memory_dir = workspace.path().join("memory");
    fs::create_dir_all(&memory_dir).unwrap();
    fs::write(workspace.path().join("MEMORY.md"), "core").unwrap();
    fs::write(memory_dir.join("2026-07-17.md"), "daily").unwrap();
    fs::write(memory_dir.join("ignored.txt"), "ignored").unwrap();

    let backup = backup_target_memory(workspace.path()).unwrap().unwrap();

    assert_eq!(fs::read_to_string(backup.join("MEMORY.md")).unwrap(), "core");
    assert_eq!(fs::read_to_string(backup.join("daily").join("2026-07-17.md")).unwrap(), "daily");
    assert!(!backup.join("daily").join("ignored.txt").exists());
}
