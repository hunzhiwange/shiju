//! 验证旧快照入口的本地恢复、导出及持久化布局。

#[test]
fn memory_snapshot_production_boundary() {
    use crate::memory::snapshot;

    let workspace = tempfile::TempDir::new().unwrap();
    let storage = crate::memory::paths::project_data_dir(workspace.path()).unwrap();
    assert!(!storage.exists(), "boundary fixture must own a fresh project data directory");
    std::fs::create_dir_all(&storage).unwrap();
    struct StorageCleanup(std::path::PathBuf);
    impl Drop for StorageCleanup {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
    let _cleanup = StorageCleanup(storage.clone());
    let snapshot_file = storage.join(snapshot::SNAPSHOT_FILENAME);
    std::fs::write(&snapshot_file, "### 🔑 `boundary`\n\n持久化快照内容\n").unwrap();

    assert!(snapshot::should_hydrate(workspace.path()));
    assert_eq!(snapshot::hydrate_from_snapshot(workspace.path()).unwrap(), 1);
    assert!(!snapshot::should_hydrate(workspace.path()));
    let db_path = storage.join("memory/brain.db");
    let conn = rusqlite::Connection::open(&db_path).unwrap();
    let content: String = conn
        .query_row("SELECT content FROM memories WHERE key = 'boundary'", [], |row| row.get(0))
        .unwrap();
    assert_eq!(content, "持久化快照内容");
    drop(conn);

    std::fs::remove_file(&snapshot_file).unwrap();
    assert_eq!(snapshot::export_snapshot(workspace.path()).unwrap(), 1);
    let exported = std::fs::read_to_string(&snapshot_file).unwrap();
    assert!(exported.contains("### 🔑 `boundary`"));
    assert!(exported.contains("持久化快照内容"));
    assert!(!workspace.path().join(snapshot::SNAPSHOT_FILENAME).exists());
    assert!(!workspace.path().join("memory/brain.db").exists());

    std::fs::remove_file(&db_path).unwrap();
    assert!(snapshot::should_hydrate(workspace.path()));
    assert_eq!(snapshot::hydrate_from_snapshot(workspace.path()).unwrap(), 1);
    assert_eq!(memory_snapshot::memory::snapshot::hydrate_from_snapshot(workspace.path()).unwrap(), 0);
    let conn = rusqlite::Connection::open(&db_path).unwrap();
    let restored: String = conn
        .query_row("SELECT content FROM memories WHERE key = 'boundary'", [], |row| row.get(0))
        .unwrap();
    assert_eq!(restored, content);
}
