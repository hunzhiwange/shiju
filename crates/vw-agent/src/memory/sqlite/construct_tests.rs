// 验证 SQLite 记忆后端的构造配置、数据目录与错误路径。
use super::*;

#[test]
fn sqlite_memory_type_name_remains_stable() {
    assert!(std::any::type_name::<SqliteMemory>().contains("SqliteMemory"));
}

#[test]
fn sqlite_memory_uses_user_scoped_data_dir() {
    let workspace = tempfile::TempDir::new().unwrap();
    let storage = crate::app::agent::memory::paths::project_data_dir(workspace.path()).unwrap();

    let _memory = SqliteMemory::new(workspace.path()).unwrap();

    assert!(!workspace.path().join("memory").join("brain.db").exists());
    assert!(storage.join("memory").join("brain.db").exists());
}

#[test]
fn with_embedder_preserves_constructor_configuration() {
    let workspace = tempfile::TempDir::new().unwrap();
    let embedder = std::sync::Arc::new(crate::app::agent::memory::embeddings::NoopEmbedding);

    let memory =
        SqliteMemory::with_embedder(workspace.path(), embedder, 0.25, 0.75, 12, Some(1)).unwrap();

    assert_eq!(memory.vector_weight, 0.25);
    assert_eq!(memory.keyword_weight, 0.75);
    assert_eq!(memory.cache_max, 12);
    assert!(memory.db_path.ends_with("memory/brain.db"));
}

#[test]
fn sqlite_memory_reports_blocked_memory_directory() {
    let workspace = tempfile::TempDir::new().unwrap();
    let storage = crate::app::agent::memory::paths::project_data_dir(workspace.path()).unwrap();
    std::fs::create_dir_all(&storage).unwrap();
    std::fs::write(storage.join("memory"), b"not a directory").unwrap();

    let result = SqliteMemory::new(workspace.path());

    assert!(result.is_err());
}
