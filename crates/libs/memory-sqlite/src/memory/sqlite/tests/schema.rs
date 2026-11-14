use super::*;
use crate::memory::MemoryCategory;

// ─────────────────────────────────────────────────────────────────────────────
// Schema 结构测试
// ─────────────────────────────────────────────────────────────────────────────

/// 测试 FTS5 全文搜索表是否存在
///
/// 验证数据库初始化时创建了 `memories_fts` 虚拟表
#[tokio::test]
async fn schema_has_fts5_table() {
    let (_tmp, mem) = temp_sqlite();
    let conn = mem.conn.lock();
    // 查询 sqlite_master 确认 FTS5 表存在
    let count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='memories_fts'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(count, 1);
}

/// 测试嵌入向量缓存表是否存在
///
/// 验证数据库初始化时创建了 `embedding_cache` 表
#[tokio::test]
async fn schema_has_embedding_cache() {
    let (_tmp, mem) = temp_sqlite();
    let conn = mem.conn.lock();
    let count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='embedding_cache'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(count, 1);
}

/// 测试 memories 表包含 embedding 列
///
/// 验证主表结构支持存储嵌入向量
#[tokio::test]
async fn schema_memories_has_embedding_column() {
    let (_tmp, mem) = temp_sqlite();
    let conn = mem.conn.lock();
    // 通过查询空结果集来验证列存在性
    let result = conn.execute_batch("SELECT embedding FROM memories LIMIT 0");
    assert!(result.is_ok());
}

// ─────────────────────────────────────────────────────────────────────────────
// FTS5 同步触发器测试
// ─────────────────────────────────────────────────────────────────────────────

/// 测试插入时 FTS5 索引同步
///
/// 验证新存储的记忆条目能立即通过 FTS5 搜索到
#[tokio::test]
async fn fts5_syncs_on_insert() {
    let (_tmp, mem) = temp_sqlite();
    mem.store("test_key", "unique_searchterm_xyz", MemoryCategory::Core, None).await.unwrap();

    let conn = mem.conn.lock();
    // 直接查询 FTS5 索引，验证内容已同步
    let count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM memories_fts WHERE memories_fts MATCH '\"unique_searchterm_xyz\"'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(count, 1);
}

/// 测试删除时 FTS5 索引同步
///
/// 验证删除记忆条目后，FTS5 索引中的对应记录也被清除
#[tokio::test]
async fn fts5_syncs_on_delete() {
    let (_tmp, mem) = temp_sqlite();
    mem.store("del_key", "deletable_content_abc", MemoryCategory::Core, None).await.unwrap();
    mem.forget("del_key").await.unwrap();

    let conn = mem.conn.lock();
    // 验证删除的内容无法通过 FTS5 搜索到
    let count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM memories_fts WHERE memories_fts MATCH '\"deletable_content_abc\"'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(count, 0);
}

/// 测试更新时 FTS5 索引同步
///
/// 验证点：
/// - 更新后旧内容无法通过 FTS5 搜索到
/// - 更新后新内容能通过 FTS5 搜索到
#[tokio::test]
async fn fts5_syncs_on_update() {
    let (_tmp, mem) = temp_sqlite();
    mem.store("upd_key", "original_content_111", MemoryCategory::Core, None).await.unwrap();
    mem.store("upd_key", "updated_content_222", MemoryCategory::Core, None).await.unwrap();

    let conn = mem.conn.lock();
    // 旧内容不应能被搜索到
    let old: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM memories_fts WHERE memories_fts MATCH '\"original_content_111\"'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(old, 0);

    // 新内容应能被搜索到
    let new: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM memories_fts WHERE memories_fts MATCH '\"updated_content_222\"'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(new, 1);
}

// ─────────────────────────────────────────────────────────────────────────────
// 边界条件：Schema 幂等性
// ─────────────────────────────────────────────────────────────────────────────

/// 测试 Schema 初始化的幂等性
///
/// 验证点：
/// - 重新打开现有数据库时，schema 初始化是幂等的
/// - 已存在的数据保持完整
/// - 可以继续正常存储新数据
#[tokio::test]
async fn schema_idempotent_reopen() {
    let tmp = TempDir::new().unwrap();
    {
        let mem = SqliteMemory::new(tmp.path()).unwrap();
        mem.store("k1", "v1", MemoryCategory::Core, None).await.unwrap();
    }
    // 再次打开 — init_schema 在现有数据库上再次运行
    let mem2 = SqliteMemory::new(tmp.path()).unwrap();
    let entry = mem2.get("k1").await.unwrap();
    assert!(entry.is_some());
    assert_eq!(entry.unwrap().content, "v1");
    // 存储更多数据 — 应正常工作
    mem2.store("k2", "v2", MemoryCategory::Daily, None).await.unwrap();
    assert_eq!(mem2.count().await.unwrap(), 2);
}

/// 测试多次打开同一数据库
///
/// 验证连续多次打开同一数据库不会出错
#[tokio::test]
async fn schema_triple_open() {
    let tmp = TempDir::new().unwrap();
    let _m1 = SqliteMemory::new(tmp.path()).unwrap();
    let _m2 = SqliteMemory::new(tmp.path()).unwrap();
    let m3 = SqliteMemory::new(tmp.path()).unwrap();
    assert!(m3.health_check().await);
}
