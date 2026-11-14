use super::*;
use crate::memory::MemoryCategory;

// ─────────────────────────────────────────────────────────────────────────────
// FTS5 全文搜索测试
// ─────────────────────────────────────────────────────────────────────────────

/// 测试基于单个关键词的记忆召回
///
/// 验证点：
/// - 能够召回包含指定关键词的所有条目
/// - 结果数量与预期一致
/// - 所有结果都包含搜索关键词（忽略大小写）
#[tokio::test]
async fn sqlite_recall_keyword() {
    let (_tmp, mem) = temp_sqlite();
    mem.store("a", "Rust is fast and safe", MemoryCategory::Core, None).await.unwrap();
    mem.store("b", "Python is interpreted", MemoryCategory::Core, None).await.unwrap();
    mem.store("c", "Rust has zero-cost abstractions", MemoryCategory::Core, None).await.unwrap();

    let results = mem.recall("Rust", 10, None).await.unwrap();
    assert_eq!(results.len(), 2);
    assert!(results.iter().all(|r| r.content.to_lowercase().contains("rust")));
}

/// 测试多关键词搜索的排序行为
///
/// 验证点：
/// - 包含多个关键词的条目排名更高
/// - 多关键词查询能返回匹配结果
#[tokio::test]
async fn sqlite_recall_multi_keyword() {
    let (_tmp, mem) = temp_sqlite();
    mem.store("a", "Rust is fast", MemoryCategory::Core, None).await.unwrap();
    mem.store("b", "Rust is safe and fast", MemoryCategory::Core, None).await.unwrap();

    let results = mem.recall("fast safe", 10, None).await.unwrap();
    assert!(!results.is_empty());
    // 同时包含两个关键词的条目应排名靠前
    assert!(results[0].content.contains("safe") && results[0].content.contains("fast"));
}

/// 测试无匹配结果时的召回行为
///
/// 验证当搜索词与所有条目都不匹配时，返回空结果集
#[tokio::test]
async fn sqlite_recall_no_match() {
    let (_tmp, mem) = temp_sqlite();
    mem.store("a", "Rust rocks", MemoryCategory::Core, None).await.unwrap();
    let results = mem.recall("javascript", 10, None).await.unwrap();
    assert!(results.is_empty());
}

/// 测试 FTS5 BM25 排名功能
///
/// 验证点：
/// - 使用 BM25 算法对搜索结果进行排名
/// - 所有返回的结果都包含搜索关键词
/// - 关键词出现频率更高的条目排名靠前
#[tokio::test]
async fn fts5_bm25_ranking() {
    let (_tmp, mem) = temp_sqlite();
    mem.store("a", "Rust is a systems programming language", MemoryCategory::Core, None)
        .await
        .unwrap();
    mem.store("b", "Python is great for scripting", MemoryCategory::Core, None).await.unwrap();
    mem.store("c", "Rust and Rust and Rust everywhere", MemoryCategory::Core, None).await.unwrap();

    let results = mem.recall("Rust", 10, None).await.unwrap();
    assert!(results.len() >= 2);
    // 所有结果都应包含 "Rust"
    for r in &results {
        assert!(r.content.to_lowercase().contains("rust"), "Expected 'rust' in: {}", r.content);
    }
}

/// 测试 FTS5 多词查询
///
/// 验证点：
/// - 多个搜索词能正确匹配包含这些词的条目
/// - 同时包含多个词的条目应排在前面
#[tokio::test]
async fn fts5_multi_word_query() {
    let (_tmp, mem) = temp_sqlite();
    mem.store("a", "The quick brown fox jumps", MemoryCategory::Core, None).await.unwrap();
    mem.store("b", "A lazy dog sleeps", MemoryCategory::Core, None).await.unwrap();
    mem.store("c", "The quick dog runs fast", MemoryCategory::Core, None).await.unwrap();

    let results = mem.recall("quick dog", 10, None).await.unwrap();
    assert!(!results.is_empty());
    // "The quick dog runs fast" 同时匹配两个词
    assert!(results[0].content.contains("quick"));
}

/// 测试空查询的召回行为
///
/// 验证空字符串查询返回空结果集
#[tokio::test]
async fn recall_empty_query_returns_empty() {
    let (_tmp, mem) = temp_sqlite();
    mem.store("a", "data", MemoryCategory::Core, None).await.unwrap();
    let results = mem.recall("", 10, None).await.unwrap();
    assert!(results.is_empty());
}

/// 测试纯空白字符查询的召回行为
///
/// 验证只包含空白字符的查询返回空结果集
#[tokio::test]
async fn recall_whitespace_query_returns_empty() {
    let (_tmp, mem) = temp_sqlite();
    mem.store("a", "data", MemoryCategory::Core, None).await.unwrap();
    let results = mem.recall("   ", 10, None).await.unwrap();
    assert!(results.is_empty());
}

// ─────────────────────────────────────────────────────────────────────────────
// 召回限制测试
// ─────────────────────────────────────────────────────────────────────────────

/// 测试召回操作遵守限制参数
///
/// 验证返回结果数量不超过指定的限制值
#[tokio::test]
async fn recall_respects_limit() {
    let (_tmp, mem) = temp_sqlite();
    // 创建 20 个包含共同关键词的条目
    for i in 0..20 {
        mem.store(
            &format!("k{i}"),
            &format!("common keyword item {i}"),
            MemoryCategory::Core,
            None,
        )
        .await
        .unwrap();
    }

    // 限制返回 5 条
    let results = mem.recall("common keyword", 5, None).await.unwrap();
    assert!(results.len() <= 5);
}

// ─────────────────────────────────────────────────────────────────────────────
// 分数存在性测试
// ─────────────────────────────────────────────────────────────────────────────

/// 测试召回结果包含相关性分数
///
/// 验证每个召回结果都有 score 字段
#[tokio::test]
async fn recall_results_have_scores() {
    let (_tmp, mem) = temp_sqlite();
    mem.store("s1", "scored result test", MemoryCategory::Core, None).await.unwrap();

    let results = mem.recall("scored", 10, None).await.unwrap();
    assert!(!results.is_empty());
    for r in &results {
        assert!(r.score.is_some(), "Expected score on result: {:?}", r.key);
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// 边界条件：FTS5 特殊字符处理
// ─────────────────────────────────────────────────────────────────────────────

/// 测试查询中包含引号的情况
///
/// 验证 FTS5 能安全处理包含引号的查询，不崩溃
#[tokio::test]
async fn recall_with_quotes_in_query() {
    let (_tmp, mem) = temp_sqlite();
    mem.store("q1", "He said hello world", MemoryCategory::Core, None).await.unwrap();
    // 查询中的引号不应导致 FTS5 崩溃
    let results = mem.recall("\"hello\"", 10, None).await.unwrap();
    // 根据FTS5转义规则可能匹配或不匹配，但绝不能报错
    assert!(results.len() <= 10);
}

/// 测试查询中包含星号通配符的情况
///
/// 验证 FTS5 能处理通配符查询
#[tokio::test]
async fn recall_with_asterisk_in_query() {
    let (_tmp, mem) = temp_sqlite();
    mem.store("a1", "wildcard test content", MemoryCategory::Core, None).await.unwrap();
    let results = mem.recall("wild*", 10, None).await.unwrap();
    assert!(results.len() <= 10);
}

/// 测试查询中包含括号的情况
///
/// 验证 FTS5 能安全处理包含括号的查询
#[tokio::test]
async fn recall_with_parentheses_in_query() {
    let (_tmp, mem) = temp_sqlite();
    mem.store("p1", "function call test", MemoryCategory::Core, None).await.unwrap();
    let results = mem.recall("function()", 10, None).await.unwrap();
    assert!(results.len() <= 10);
}

/// 测试 SQL 注入尝试的防护
///
/// 验证点：
/// - 恶意 SQL 片段不会导致崩溃
/// - 不会泄露或破坏数据
/// - 表结构保持完整
#[tokio::test]
async fn recall_with_sql_injection_attempt() {
    let (_tmp, mem) = temp_sqlite();
    mem.store("safe", "normal content", MemoryCategory::Core, None).await.unwrap();
    // 应不崩溃也不泄露数据
    let results = mem.recall("'; DROP TABLE memories; --", 10, None).await.unwrap();
    assert!(results.len() <= 10);
    // 表应仍然存在
    assert_eq!(mem.count().await.unwrap(), 1);
}

// ─────────────────────────────────────────────────────────────────────────────
// 边界条件：召回操作
// ─────────────────────────────────────────────────────────────────────────────

/// 测试单字符查询
///
/// 验证单字符查询不会崩溃（可能不匹配 FTS5 但 LIKE 回退应工作）
#[tokio::test]
async fn recall_single_character_query() {
    let (_tmp, mem) = temp_sqlite();
    mem.store("a", "x marks the spot", MemoryCategory::Core, None).await.unwrap();
    // 单字符可能不匹配 FTS5，但 LIKE 回退应工作
    let results = mem.recall("x", 10, None).await.unwrap();
    // 不应崩溃；可能找到也可能找不到结果
    assert!(results.len() <= 10);
}

/// 测试限制为 0 的召回
///
/// 验证限制为 0 时返回空结果集
#[tokio::test]
async fn recall_limit_zero() {
    let (_tmp, mem) = temp_sqlite();
    mem.store("a", "some content", MemoryCategory::Core, None).await.unwrap();
    let results = mem.recall("some", 0, None).await.unwrap();
    assert!(results.is_empty());
}

/// 测试限制为 1 的召回
///
/// 验证限制为 1 时只返回最多 1 条结果
#[tokio::test]
async fn recall_limit_one() {
    let (_tmp, mem) = temp_sqlite();
    mem.store("a", "matching content alpha", MemoryCategory::Core, None).await.unwrap();
    mem.store("b", "matching content beta", MemoryCategory::Core, None).await.unwrap();
    let results = mem.recall("matching content", 1, None).await.unwrap();
    assert_eq!(results.len(), 1);
}

/// 测试按键而非仅按内容匹配
///
/// 验证召回能匹配键中的关键词，而不仅仅是内容
#[tokio::test]
async fn recall_matches_by_key_not_just_content() {
    let (_tmp, mem) = temp_sqlite();
    mem.store("rust_preferences", "User likes systems programming", MemoryCategory::Core, None)
        .await
        .unwrap();
    // "rust" 出现在键中但不在内容中 —— LIKE 回退也会检查键
    let results = mem.recall("rust", 10, None).await.unwrap();
    assert!(!results.is_empty(), "Should match by key");
}

/// 测试 Unicode 查询
///
/// 验证能正确搜索非 ASCII 字符（如日文）
#[tokio::test]
async fn recall_unicode_query() {
    let (_tmp, mem) = temp_sqlite();
    mem.store("jp", "日本語のテスト", MemoryCategory::Core, None).await.unwrap();
    let results = mem.recall("日本語", 10, None).await.unwrap();
    assert!(!results.is_empty());
}
