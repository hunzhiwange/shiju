use super::*;
use crate::memory::MemoryCategory;

// ─────────────────────────────────────────────────────────────────────────────
// 会话隔离测试
// ─────────────────────────────────────────────────────────────────────────────

/// 测试带会话 ID 的存储和召回
///
/// 验证点：
/// - 能存储带会话 ID 的条目
/// - 能按会话 ID 过滤召回结果
/// - 返回的结果包含正确的会话 ID
#[tokio::test]
async fn store_and_recall_with_session_id() {
    let (_tmp, mem) = temp_sqlite();
    mem.store("k1", "session A fact", MemoryCategory::Core, Some("sess-a")).await.unwrap();
    mem.store("k2", "session B fact", MemoryCategory::Core, Some("sess-b")).await.unwrap();
    mem.store("k3", "no session fact", MemoryCategory::Core, None).await.unwrap();

    // 使用 session-a 过滤召回，应只返回 session-a 的条目
    let results = mem.recall("fact", 10, Some("sess-a")).await.unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].key, "k1");
    assert_eq!(results[0].session_id.as_deref(), Some("sess-a"));
}

/// 测试无会话过滤时召回所有条目
///
/// 验证不指定会话 ID 时，召回返回所有匹配条目
#[tokio::test]
async fn recall_no_session_filter_returns_all() {
    let (_tmp, mem) = temp_sqlite();
    mem.store("k1", "alpha fact", MemoryCategory::Core, Some("sess-a")).await.unwrap();
    mem.store("k2", "beta fact", MemoryCategory::Core, Some("sess-b")).await.unwrap();
    mem.store("k3", "gamma fact", MemoryCategory::Core, None).await.unwrap();

    // 无会话过滤时召回所有匹配条目
    let results = mem.recall("fact", 10, None).await.unwrap();
    assert_eq!(results.len(), 3);
}

/// 测试跨会话召回隔离
///
/// 验证点：
/// - 一个会话无法看到另一个会话的数据
/// - 会话只能看到自己的数据
#[tokio::test]
async fn cross_session_recall_isolation() {
    let (_tmp, mem) = temp_sqlite();
    mem.store("secret", "session A secret data", MemoryCategory::Core, Some("sess-a"))
        .await
        .unwrap();

    // Session B 无法看到 Session A 的数据
    let results = mem.recall("secret", 10, Some("sess-b")).await.unwrap();
    assert!(results.is_empty());

    // Session A 可以看到自己的数据
    let results = mem.recall("secret", 10, Some("sess-a")).await.unwrap();
    assert_eq!(results.len(), 1);
}

/// 测试带会话过滤的列表操作
///
/// 验证点：
/// - 列表操作支持会话过滤
/// - 支持同时按分类和会话过滤
#[tokio::test]
async fn list_with_session_filter() {
    let (_tmp, mem) = temp_sqlite();
    mem.store("k1", "a1", MemoryCategory::Core, Some("sess-a")).await.unwrap();
    mem.store("k2", "a2", MemoryCategory::Conversation, Some("sess-a")).await.unwrap();
    mem.store("k3", "b1", MemoryCategory::Core, Some("sess-b")).await.unwrap();
    mem.store("k4", "none1", MemoryCategory::Core, None).await.unwrap();

    // 仅按 session-a 过滤
    let results = mem.list(None, Some("sess-a")).await.unwrap();
    assert_eq!(results.len(), 2);
    assert!(results.iter().all(|e| e.session_id.as_deref() == Some("sess-a")));

    // 按 session-a + 分类过滤
    let results = mem.list(Some(&MemoryCategory::Core), Some("sess-a")).await.unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].key, "k1");
}

/// 测试 Schema 迁移在重新打开时的幂等性
///
/// 验证点：
/// - 重新打开数据库时迁移脚本能幂等执行
/// - 会话相关的数据能正确保存和读取
#[tokio::test]
async fn schema_migration_idempotent_on_reopen() {
    let tmp = TempDir::new().unwrap();

    // 首次打开：创建 schema + 迁移
    {
        let mem = SqliteMemory::new(tmp.path()).unwrap();
        mem.store("k1", "before reopen", MemoryCategory::Core, Some("sess-x")).await.unwrap();
    }

    // 二次打开：迁移再次运行但幂等
    {
        let mem = SqliteMemory::new(tmp.path()).unwrap();
        let results = mem.recall("reopen", 10, Some("sess-x")).await.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].key, "k1");
        assert_eq!(results[0].session_id.as_deref(), Some("sess-x"));
    }
}
