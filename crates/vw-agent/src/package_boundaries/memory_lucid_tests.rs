//! 验证 Lucid 旧入口的类型身份与本地 SQLite 行为。

#[tokio::test]
async fn memory_lucid_production_boundary() {
    use crate::memory::{LucidMemory, Memory, MemoryCategory, SqliteMemory};

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
    let local = SqliteMemory::new(workspace.path()).unwrap();
    local.store("boundary", "persisted local note", MemoryCategory::Core, Some("s1"))
        .await
        .unwrap();
    let memory: memory_lucid::memory::lucid::LucidMemory =
        LucidMemory::new(workspace.path(), local);
    let _: &dyn memory_types::memory::Memory = &memory;

    // 本地读取和删除不启动 Lucid 子进程。
    assert!(memory.health_check().await);
    assert_eq!(memory.count().await.unwrap(), 1);
    assert_eq!(memory.get("boundary").await.unwrap().unwrap().content, "persisted local note");
    assert_eq!(memory.list(Some(&MemoryCategory::Core), Some("s1")).await.unwrap().len(), 1);
    assert!(storage.join("memory/brain.db").is_file());
    assert!(memory.forget("boundary").await.unwrap());
    drop(memory);

    let reopened = SqliteMemory::new(workspace.path()).unwrap();
    assert_eq!(reopened.count().await.unwrap(), 0);
    assert!(reopened.get("boundary").await.unwrap().is_none());
}
