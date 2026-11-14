//! 验证 SQLite 旧入口的类型身份及真实数据库持久化。

#[cfg(not(target_arch = "wasm32"))]
#[tokio::test]
async fn memory_sqlite_production_boundary() {
    use crate::memory::{Memory, MemoryCategory, SqliteMemory};

    let workspace = tempfile::TempDir::new().unwrap();
    let storage = crate::memory::paths::project_data_dir(workspace.path()).unwrap();
    assert!(!storage.exists(), "boundary fixture must own a fresh data directory");
    struct StorageCleanup(std::path::PathBuf);
    impl Drop for StorageCleanup {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
    let _cleanup = StorageCleanup(storage.clone());
    let memory: memory_sqlite::memory::sqlite::SqliteMemory =
        SqliteMemory::new(workspace.path()).unwrap();
    let _: &dyn memory_types::memory::Memory = &memory;
    memory.store("boundary", "first", MemoryCategory::Core, None).await.unwrap();
    memory.store("boundary", "persisted keyword", MemoryCategory::Core, None).await.unwrap();
    assert_eq!(memory.count().await.unwrap(), 1);
    assert!(storage.join("memory/brain.db").is_file());
    drop(memory);

    let reopened = SqliteMemory::new(workspace.path()).unwrap();
    assert_eq!(reopened.get("boundary").await.unwrap().unwrap().content, "persisted keyword");
    assert_eq!(reopened.recall("keyword", 1, None).await.unwrap()[0].key, "boundary");
    assert!(reopened.forget("boundary").await.unwrap());
    assert_eq!(reopened.count().await.unwrap(), 0);
}
