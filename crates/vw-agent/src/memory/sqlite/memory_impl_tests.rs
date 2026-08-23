// 验证 SQLite 记忆实现对损坏行和缺失模式的错误边界。
use super::*;
use rusqlite::params;

#[test]
fn sqlite_memory_impl_module_is_linked() {
    assert!(std::any::type_name::<SqliteMemory>().contains("SqliteMemory"));
}

#[tokio::test]
async fn malformed_row_is_hidden_by_get_and_rejected_by_list() {
    let workspace = tempfile::TempDir::new().unwrap();
    let memory = SqliteMemory::new(workspace.path()).unwrap();
    {
        let conn = memory.conn.lock();
        conn.execute(
            "INSERT INTO memories
             (id, key, content, category, created_at, updated_at, session_id)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                vec![0xff_u8],
                "malformed",
                "content",
                "core",
                "2026-01-01T00:00:00Z",
                "2026-01-01T00:00:00Z",
                Option::<String>::None,
            ],
        )
        .unwrap();
    }

    assert!(memory.get("malformed").await.unwrap().is_none());
    assert!(memory.list(None, None).await.is_err());
}

#[tokio::test]
async fn operations_report_missing_memories_table() {
    let workspace = tempfile::TempDir::new().unwrap();
    let memory = SqliteMemory::new(workspace.path()).unwrap();
    memory.conn.lock().execute("DROP TABLE memories", []).unwrap();

    assert!(memory.store("key", "content", MemoryCategory::Core, None).await.is_err());
    assert!(memory.recall("content", 10, None).await.is_err());
    assert!(memory.get("key").await.is_err());
    assert!(memory.list(None, None).await.is_err());
    assert!(memory.forget("key").await.is_err());
    assert!(memory.count().await.is_err());
}
