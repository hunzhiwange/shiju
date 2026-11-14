use super::*;
use crate::memory::MemoryCategory;

/// 测试 `name()` 方法返回正确的标识符
///
/// 验证 SQLite 记忆实例的名称标识为 "sqlite"
#[tokio::test]
async fn sqlite_name() {
    let (_tmp, mem) = temp_sqlite();
    assert_eq!(mem.name(), "sqlite");
}

/// 测试 `health_check()` 方法返回健康状态
///
/// 验证新创建的实例能够通过健康检查
#[tokio::test]
async fn sqlite_health() {
    let (_tmp, mem) = temp_sqlite();
    assert!(mem.health_check().await);
}

/// 测试存储和检索记忆条目的基本功能
///
/// 验证点：
/// - 存储条目后能够成功检索
/// - 检索的条目包含正确的键、内容和分类
#[tokio::test]
async fn sqlite_store_and_get() {
    let (_tmp, mem) = temp_sqlite();
    mem.store("user_lang", "Prefers Rust", MemoryCategory::Core, None).await.unwrap();

    let entry = mem.get("user_lang").await.unwrap();
    assert!(entry.is_some());
    let entry = entry.unwrap();
    assert_eq!(entry.key, "user_lang");
    assert_eq!(entry.content, "Prefers Rust");
    assert_eq!(entry.category, MemoryCategory::Core);
}

/// 测试存储操作的 UPSERT 行为
///
/// 验证点：
/// - 相同键的重复存储会更新内容
/// - 最终只保留一条记录
#[tokio::test]
async fn sqlite_store_upsert() {
    let (_tmp, mem) = temp_sqlite();
    mem.store("pref", "likes Rust", MemoryCategory::Core, None).await.unwrap();
    mem.store("pref", "loves Rust", MemoryCategory::Core, None).await.unwrap();

    let entry = mem.get("pref").await.unwrap().unwrap();
    assert_eq!(entry.content, "loves Rust");
    assert_eq!(mem.count().await.unwrap(), 1);
}

/// 测试删除已存在的记忆条目
///
/// 验证点：
/// - 删除操作返回 true 表示成功
/// - 删除后条目数量减为 0
#[tokio::test]
async fn sqlite_forget() {
    let (_tmp, mem) = temp_sqlite();
    mem.store("temp", "temporary data", MemoryCategory::Conversation, None).await.unwrap();
    assert_eq!(mem.count().await.unwrap(), 1);

    let removed = mem.forget("temp").await.unwrap();
    assert!(removed);
    assert_eq!(mem.count().await.unwrap(), 0);
}

/// 测试删除不存在的记忆条目
///
/// 验证删除不存在的键时返回 false
#[tokio::test]
async fn sqlite_forget_nonexistent() {
    let (_tmp, mem) = temp_sqlite();
    let removed = mem.forget("nope").await.unwrap();
    assert!(!removed);
}

/// 测试列出所有记忆条目
///
/// 验证 `list()` 方法能返回所有已存储的条目
#[tokio::test]
async fn sqlite_list_all() {
    let (_tmp, mem) = temp_sqlite();
    mem.store("a", "one", MemoryCategory::Core, None).await.unwrap();
    mem.store("b", "two", MemoryCategory::Daily, None).await.unwrap();
    mem.store("c", "three", MemoryCategory::Conversation, None).await.unwrap();

    let all = mem.list(None, None).await.unwrap();
    assert_eq!(all.len(), 3);
}

/// 测试按分类过滤列出记忆条目
///
/// 验证点：
/// - 能够按指定分类过滤条目
/// - 不同分类的条目数量正确
#[tokio::test]
async fn sqlite_list_by_category() {
    let (_tmp, mem) = temp_sqlite();
    mem.store("a", "core1", MemoryCategory::Core, None).await.unwrap();
    mem.store("b", "core2", MemoryCategory::Core, None).await.unwrap();
    mem.store("c", "daily1", MemoryCategory::Daily, None).await.unwrap();

    let core = mem.list(Some(&MemoryCategory::Core), None).await.unwrap();
    assert_eq!(core.len(), 2);

    let daily = mem.list(Some(&MemoryCategory::Daily), None).await.unwrap();
    assert_eq!(daily.len(), 1);
}

/// 测试空数据库的计数
///
/// 验证新创建的实例计数为 0
#[tokio::test]
async fn sqlite_count_empty() {
    let (_tmp, mem) = temp_sqlite();
    assert_eq!(mem.count().await.unwrap(), 0);
}

/// 测试检索不存在的记忆条目
///
/// 验证查询不存在的键时返回 None
#[tokio::test]
async fn sqlite_get_nonexistent() {
    let (_tmp, mem) = temp_sqlite();
    assert!(mem.get("nope").await.unwrap().is_none());
}

/// 测试数据库持久化能力
///
/// 验证点：
/// - 关闭实例后重新打开，数据仍然存在
/// - 持久化的条目内容完整
#[tokio::test]
async fn sqlite_db_persists() {
    let tmp = TempDir::new().unwrap();

    // 第一次打开：存储数据
    {
        let mem = SqliteMemory::new(tmp.path()).unwrap();
        mem.store("persist", "I survive restarts", MemoryCategory::Core, None).await.unwrap();
    }

    // 重新打开：验证数据持久化
    let mem2 = SqliteMemory::new(tmp.path()).unwrap();
    let entry = mem2.get("persist").await.unwrap();
    assert!(entry.is_some());
    assert_eq!(entry.unwrap().content, "I survive restarts");
}

/// 测试记忆分类的完整往返（序列化与反序列化）
///
/// 验证点：
/// - 预定义分类（Core、Daily、Conversation）能正确保存和读取
/// - 自定义分类能正确保存和读取
/// - 分类值在存储后能完整还原
#[tokio::test]
async fn sqlite_category_roundtrip() {
    let (_tmp, mem) = temp_sqlite();
    let categories = [
        MemoryCategory::Core,
        MemoryCategory::Daily,
        MemoryCategory::Conversation,
        MemoryCategory::Custom("project".into()),
    ];

    // 存储不同分类的条目
    for (i, cat) in categories.iter().enumerate() {
        let category: MemoryCategory = (*cat).clone();
        mem.store(&format!("k{i}"), &format!("v{i}"), category, None).await.unwrap();
    }

    // 验证每个分类能正确还原
    for (i, cat) in categories.iter().enumerate() {
        let entry = mem.get(&format!("k{i}")).await.unwrap().unwrap();
        assert_eq!(&entry.category, cat);
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// 边界条件：存储操作
// ─────────────────────────────────────────────────────────────────────────────

/// 测试存储空内容
///
/// 验证能正确存储和检索空字符串内容
#[tokio::test]
async fn store_empty_content() {
    let (_tmp, mem) = temp_sqlite();
    mem.store("empty", "", MemoryCategory::Core, None).await.unwrap();
    let entry = mem.get("empty").await.unwrap().unwrap();
    assert_eq!(entry.content, "");
}

/// 测试存储空键
///
/// 验证能正确存储和检索键为空字符串的条目
#[tokio::test]
async fn store_empty_key() {
    let (_tmp, mem) = temp_sqlite();
    mem.store("", "content for empty key", MemoryCategory::Core, None).await.unwrap();
    let entry = mem.get("").await.unwrap().unwrap();
    assert_eq!(entry.content, "content for empty key");
}

/// 测试存储超长内容
///
/// 验证能正确存储和检索 10 万字符的长文本
#[tokio::test]
async fn store_very_long_content() {
    let (_tmp, mem) = temp_sqlite();
    let long_content = "x".repeat(100_000);
    mem.store("long", &long_content, MemoryCategory::Core, None).await.unwrap();
    let entry = mem.get("long").await.unwrap().unwrap();
    assert_eq!(entry.content.len(), 100_000);
}

/// 测试存储 Unicode 和表情符号
///
/// 验证能正确存储和检索包含多语言字符和表情符号的内容
#[tokio::test]
async fn store_unicode_and_emoji() {
    let (_tmp, mem) = temp_sqlite();
    mem.store("emoji_key_🦀", "こんにちは 🚀 Ñoño", MemoryCategory::Core, None).await.unwrap();
    let entry = mem.get("emoji_key_🦀").await.unwrap().unwrap();
    assert_eq!(entry.content, "こんにちは 🚀 Ñoño");
}

/// 测试存储包含换行符和制表符的内容
///
/// 验证能正确保留内容中的各种空白字符
#[tokio::test]
async fn store_content_with_newlines_and_tabs() {
    let (_tmp, mem) = temp_sqlite();
    let content = "line1\nline2\ttab\rcarriage\n\nnewparagraph";
    mem.store("whitespace", content, MemoryCategory::Core, None).await.unwrap();
    let entry = mem.get("whitespace").await.unwrap().unwrap();
    assert_eq!(entry.content, content);
}

// ─────────────────────────────────────────────────────────────────────────────
// 边界条件：列表操作
// ─────────────────────────────────────────────────────────────────────────────

/// 测试按自定义分类列出条目
///
/// 验证能正确过滤并列出自定义分类的条目
#[tokio::test]
async fn list_custom_category() {
    let (_tmp, mem) = temp_sqlite();
    mem.store("c1", "custom1", MemoryCategory::Custom("project".into()), None).await.unwrap();
    mem.store("c2", "custom2", MemoryCategory::Custom("project".into()), None).await.unwrap();
    mem.store("c3", "other", MemoryCategory::Core, None).await.unwrap();

    let project = mem.list(Some(&MemoryCategory::Custom("project".into())), None).await.unwrap();
    assert_eq!(project.len(), 2);
}

/// 测试空数据库的列表操作
///
/// 验证空数据库返回空列表
#[tokio::test]
async fn list_empty_db() {
    let (_tmp, mem) = temp_sqlite();
    let all = mem.list(None, None).await.unwrap();
    assert!(all.is_empty());
}
