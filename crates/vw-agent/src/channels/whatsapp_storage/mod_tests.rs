#[cfg(feature = "whatsapp-web")]
use super::RusqliteStore;

#[test]
#[cfg(not(feature = "whatsapp-web"))]
fn rusqlite_store_is_disabled_without_whatsapp_web_feature() {
    assert!(!cfg!(feature = "whatsapp-web"));
}

#[test]
#[cfg(feature = "whatsapp-web")]
fn rusqlite_store_creates_missing_parent_directories() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("nested").join("session.db");

    let store = RusqliteStore::new(&db_path).unwrap();

    assert_eq!(store.db_path, db_path.to_string_lossy());
    assert_eq!(store.device_id, 1);
    assert!(db_path.exists());
}

#[test]
#[cfg(feature = "whatsapp-web")]
fn rusqlite_store_accepts_memory_database_without_parent_directory() {
    let store = RusqliteStore::new(":memory:").unwrap();

    assert_eq!(store.db_path, ":memory:");
}

#[test]
#[cfg(feature = "whatsapp-web")]
fn rusqlite_store_initializes_connection_pragmas_and_schema() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("session.db");
    let store = RusqliteStore::new(&db_path).unwrap();
    let conn = store.conn.lock();

    let journal_mode: String =
        conn.query_row("PRAGMA journal_mode;", [], |row| row.get(0)).unwrap();
    let synchronous: i64 = conn.query_row("PRAGMA synchronous;", [], |row| row.get(0)).unwrap();
    let device_table_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = 'device'",
            [],
            |row| row.get(0),
        )
        .unwrap();

    assert_eq!(journal_mode, "wal");
    assert_eq!(synchronous, 1);
    assert_eq!(device_table_count, 1);
}

#[test]
#[cfg(feature = "whatsapp-web")]
fn rusqlite_store_returns_error_when_parent_path_is_a_file() {
    let dir = tempfile::tempdir().unwrap();
    let file_path = dir.path().join("blocked");
    std::fs::write(&file_path, b"not a directory").unwrap();

    let err = match RusqliteStore::new(file_path.join("session.db")) {
        Ok(_) => panic!("expected parent file path to fail"),
        Err(err) => err,
    };
    assert!(!err.to_string().is_empty());
}

#[test]
#[cfg(feature = "whatsapp-web")]
fn rusqlite_store_returns_error_when_path_is_directory() {
    let dir = tempfile::tempdir().unwrap();

    let err = match RusqliteStore::new(dir.path()) {
        Ok(_) => panic!("expected directory path to fail"),
        Err(err) => err,
    };

    assert!(err.to_string().contains("unable to open database file"));
}

#[test]
#[cfg(all(feature = "whatsapp-web", unix))]
fn rusqlite_store_returns_error_when_wal_pragmas_cannot_create_sidecars() {
    use std::os::unix::fs::PermissionsExt;

    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("readonly-parent.db");
    let conn = rusqlite::Connection::open(&db_path).unwrap();
    drop(conn);

    let original_permissions = std::fs::metadata(dir.path()).unwrap().permissions();
    std::fs::set_permissions(dir.path(), std::fs::Permissions::from_mode(0o500)).unwrap();
    let result = RusqliteStore::new(&db_path);
    std::fs::set_permissions(dir.path(), original_permissions).unwrap();

    let err = match result {
        Ok(_) => panic!("expected read-only parent directory to fail WAL setup"),
        Err(err) => err,
    };
    assert!(!err.to_string().is_empty());
}

#[test]
#[cfg(feature = "whatsapp-web")]
fn rusqlite_store_preserves_existing_schema_object_named_device() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("conflicting.db");
    let conn = rusqlite::Connection::open(&db_path).unwrap();
    conn.execute_batch("CREATE VIEW device AS SELECT 1 AS id;").unwrap();
    drop(conn);

    let store = RusqliteStore::new(&db_path).unwrap();
    let conn = store.conn.lock();
    let object_type: String = conn
        .query_row("SELECT type FROM sqlite_master WHERE name = 'device'", [], |row| row.get(0))
        .unwrap();

    assert_eq!(object_type, "view");
}
// 本文件验证 WhatsApp SQLite 存储初始化、路径错误与既有结构兼容。
