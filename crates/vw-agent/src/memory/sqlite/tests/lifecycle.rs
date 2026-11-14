use super::*;
use crate::memory::MemoryCategory;
use crate::memory::embeddings;

// ─────────────────────────────────────────────────────────────────────────────
// 打开超时测试
// ─────────────────────────────────────────────────────────────────────────────

/// 测试快速路径下的打开超时设置
///
/// 验证在快速初始化场景下，带超时参数的构造函数能成功完成
#[test]
fn open_with_timeout_succeeds_when_fast() {
    let tmp = TempDir::new().unwrap();
    let embedder = Arc::new(embeddings::NoopEmbedding);
    let mem = SqliteMemory::with_embedder(tmp.path(), embedder, 0.7, 0.3, 1000, Some(5));
    assert!(mem.is_ok(), "open with 5s timeout should succeed on fast path");
    assert_eq!(mem.unwrap().name(), "sqlite");
}

/// 测试带超时设置时的存储和召回功能
///
/// 验证超时参数不影响正常的存储和检索操作
#[tokio::test]
async fn open_with_timeout_store_recall_unchanged() {
    let tmp = TempDir::new().unwrap();
    let mem = SqliteMemory::with_embedder(
        tmp.path(),
        Arc::new(embeddings::NoopEmbedding),
        0.7,
        0.3,
        1000,
        Some(2),
    )
    .unwrap();
    mem.store("timeout_key", "value with timeout", MemoryCategory::Core, None).await.unwrap();
    let entry = mem.get("timeout_key").await.unwrap().unwrap();
    assert_eq!(entry.content, "value with timeout");
}

// ─────────────────────────────────────────────────────────────────────────────
// 带嵌入器构造函数测试
// ─────────────────────────────────────────────────────────────────────────────

/// 测试使用空操作嵌入器构造实例
///
/// 验证 `with_embedder` 构造函数能正确初始化
#[test]
fn with_embedder_noop() {
    let tmp = TempDir::new().unwrap();
    let embedder = Arc::new(embeddings::NoopEmbedding);
    let mem = SqliteMemory::with_embedder(tmp.path(), embedder, 0.7, 0.3, 1000, None);
    assert!(mem.is_ok());
    assert_eq!(mem.unwrap().name(), "sqlite");
}

// ─────────────────────────────────────────────────────────────────────────────
// 重建索引测试
// ─────────────────────────────────────────────────────────────────────────────

/// 测试重建 FTS 索引功能
///
/// 验证点：
/// - 重建索引操作成功完成
/// - 重建后 FTS5 搜索功能正常
/// - 使用空操作嵌入器时无需重新嵌入
#[tokio::test]
async fn reindex_rebuilds_fts() {
    let (_tmp, mem) = temp_sqlite();
    mem.store("r1", "reindex test alpha", MemoryCategory::Core, None).await.unwrap();
    mem.store("r2", "reindex test beta", MemoryCategory::Core, None).await.unwrap();

    // 重建索引应成功（空操作嵌入器 → 0 次重新嵌入）
    let count = mem.reindex().await.unwrap();
    assert_eq!(count, 0);

    // 重建后 FTS5 仍应正常工作
    let results = mem.recall("reindex", 10, None).await.unwrap();
    assert_eq!(results.len(), 2);
}

// ─────────────────────────────────────────────────────────────────────────────
// 边界条件：重建索引
// ─────────────────────────────────────────────────────────────────────────────

/// 测试空数据库的重建索引
///
/// 验证对空数据库执行重建索引不会出错
#[tokio::test]
async fn reindex_empty_db() {
    let (_tmp, mem) = temp_sqlite();
    let count = mem.reindex().await.unwrap();
    assert_eq!(count, 0);
}

/// 测试多次重建索引的安全性
///
/// 验证点：
/// - 多次重建索引不会出错
/// - 数据保持完整
#[tokio::test]
async fn reindex_twice_is_safe() {
    let (_tmp, mem) = temp_sqlite();
    mem.store("r1", "reindex data", MemoryCategory::Core, None).await.unwrap();
    mem.reindex().await.unwrap();
    let count = mem.reindex().await.unwrap();
    assert_eq!(count, 0); // 空操作嵌入器 → 无需重新嵌入
    // 数据应仍然完整
    let results = mem.recall("reindex", 10, None).await.unwrap();
    assert_eq!(results.len(), 1);
}

// ─────────────────────────────────────────────────────────────────────────────
// §4.2 重建索引 / 损坏恢复测试
// ─────────────────────────────────────────────────────────────────────────────

/// 测试重建索引保留数据
///
/// 验证点：
/// - 重建索引后所有条目仍然存在
/// - 条目内容保持不变
#[tokio::test]
async fn sqlite_reindex_preserves_data() {
    let (_tmp, mem) = temp_sqlite();
    mem.store("a", "Rust is fast", MemoryCategory::Core, None).await.unwrap();
    mem.store("b", "Python is interpreted", MemoryCategory::Core, None).await.unwrap();

    mem.reindex().await.unwrap();

    // 重建索引后数据应完整保留
    let count = mem.count().await.unwrap();
    assert_eq!(count, 2, "reindex must preserve all entries");

    let entry = mem.get("a").await.unwrap();
    assert!(entry.is_some());
    assert_eq!(entry.unwrap().content, "Rust is fast");
}

/// 测试重建索引的幂等性
///
/// 验证多次重建索引是安全的，不会破坏数据
#[tokio::test]
async fn sqlite_reindex_idempotent() {
    let (_tmp, mem) = temp_sqlite();
    mem.store("x", "test data", MemoryCategory::Core, None).await.unwrap();

    // 多次重建索引应是安全的
    mem.reindex().await.unwrap();
    mem.reindex().await.unwrap();
    mem.reindex().await.unwrap();

    assert_eq!(mem.count().await.unwrap(), 1);
}

// ─────────────────────────────────────────────────────────────────────────────
// 边界条件：内容哈希
// ─────────────────────────────────────────────────────────────────────────────

/// 测试内容哈希的确定性
///
/// 验证相同输入总是产生相同的哈希值
#[test]
fn content_hash_deterministic() {
    let h1 = SqliteMemory::content_hash("hello world");
    let h2 = SqliteMemory::content_hash("hello world");
    assert_eq!(h1, h2);
}

/// 测试不同输入产生不同的哈希值
///
/// 验证内容哈希的区分能力
#[test]
fn content_hash_different_inputs() {
    let h1 = SqliteMemory::content_hash("hello");
    let h2 = SqliteMemory::content_hash("world");
    assert_ne!(h1, h2);
}

/// 测试空字符串的内容哈希
///
/// 验证空字符串能产生有效的哈希值
#[test]
fn content_hash_empty_string() {
    let h = SqliteMemory::content_hash("");
    assert!(!h.is_empty());
    assert_eq!(h.len(), 16); // 16 个十六进制字符
}

/// 测试 Unicode 字符的内容哈希
///
/// 验证 Unicode 字符（如表情符号）能正确哈希
#[test]
fn content_hash_unicode() {
    let h1 = SqliteMemory::content_hash("🦀");
    let h2 = SqliteMemory::content_hash("🦀");
    assert_eq!(h1, h2);
    let h3 = SqliteMemory::content_hash("🚀");
    assert_ne!(h1, h3);
}

/// 测试超长输入的内容哈希
///
/// 验证百万字符的输入能产生有效的 16 字符哈希
#[test]
fn content_hash_long_input() {
    let long = "a".repeat(1_000_000);
    let h = SqliteMemory::content_hash(&long);
    assert_eq!(h.len(), 16);
}

// ─────────────────────────────────────────────────────────────────────────────
// 边界条件：分类辅助函数
// ─────────────────────────────────────────────────────────────────────────────

/// 测试包含空格的自定义分类往返
///
/// 验证自定义分类名称中的空格能正确序列化和反序列化
#[test]
fn category_roundtrip_custom_with_spaces() {
    let cat = MemoryCategory::Custom("my custom category".into());
    let s = SqliteMemory::category_to_str(&cat);
    assert_eq!(s, "my custom category");
    let back = SqliteMemory::str_to_category(&s);
    assert_eq!(back, cat);
}

/// 测试空字符串自定义分类的往返
///
/// 验证空字符串的自定义分类能正确处理
#[test]
fn category_roundtrip_empty_custom() {
    let cat = MemoryCategory::Custom(String::new());
    let s = SqliteMemory::category_to_str(&cat);
    assert_eq!(s, "");
    let back = SqliteMemory::str_to_category(&s);
    assert_eq!(back, MemoryCategory::Custom(String::new()));
}
