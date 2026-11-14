//! PostgreSQL 内存存储模块的单元测试
//!
//! 本模块包含针对 PostgreSQL 内存后端的各项测试用例，主要验证：
//! - 标识符验证逻辑（schema 名称、表名等）
//! - 分类字符串解析功能
//! - 构造函数的健壮性与错误处理
//!
//! 所有测试均使用标准 Rust 测试框架，异步测试使用 tokio 运行时。

use super::*;
use crate::memory::MemoryCategory;

/// 测试有效的标识符能够通过验证
///
/// 验证规则：
/// - 允许字母开头或下划线开头
/// - 允许包含字母、数字、下划线
/// - 不能为空
/// - 不能以数字开头
/// - 不能包含连字符等特殊字符
///
/// # 示例
/// - `public` - 有效的 schema 名称
/// - `_memories_01` - 有效的表名（下划线开头 + 数字后缀）
#[test]
fn valid_identifiers_pass_validation() {
    // 测试标准的 schema 名称
    assert!(validate_identifier("public", "schema").is_ok());
    // 测试包含下划线和数字的表名
    assert!(validate_identifier("_memories_01", "table").is_ok());
}

/// 测试无效的标识符会被正确拒绝
///
/// 覆盖的无效情况：
/// - 空字符串
/// - 数字开头
/// - 包含非法字符（如连字符）
#[test]
fn invalid_identifiers_are_rejected() {
    // 空字符串应该被拒绝
    assert!(validate_identifier("", "schema").is_err());
    // 数字开头应该被拒绝
    assert!(validate_identifier("1bad", "schema").is_err());
    // 包含连字符应该被拒绝
    assert!(validate_identifier("bad-name", "table").is_err());
}

#[test]
fn category_to_str_maps_all_category_variants() {
    assert_eq!(PostgresMemory::category_to_str(&MemoryCategory::Core), "core");
    assert_eq!(PostgresMemory::category_to_str(&MemoryCategory::Daily), "daily");
    assert_eq!(PostgresMemory::category_to_str(&MemoryCategory::Conversation), "conversation");
    assert_eq!(
        PostgresMemory::category_to_str(&MemoryCategory::Custom("archive".into())),
        "archive"
    );
}

/// 测试分类解析功能能够正确映射已知值和自定义值
///
/// `parse_category` 方法应支持：
/// - 内置分类：`core`、`daily`、`conversation`
/// - 自定义分类：任意字符串映射到 `MemoryCategory::Custom`
#[test]
fn parse_category_maps_known_and_custom_values() {
    // 验证内置分类 "core"
    assert_eq!(PostgresMemory::parse_category("core"), MemoryCategory::Core);
    // 验证内置分类 "daily"
    assert_eq!(PostgresMemory::parse_category("daily"), MemoryCategory::Daily);
    // 验证内置分类 "conversation"
    assert_eq!(PostgresMemory::parse_category("conversation"), MemoryCategory::Conversation);
    // 验证自定义分类 "custom_notes" 映射到 Custom 变体
    assert_eq!(
        PostgresMemory::parse_category("custom_notes"),
        MemoryCategory::Custom("custom_notes".into())
    );
}

#[test]
fn new_rejects_invalid_schema_before_url_parsing_or_connection() {
    let err = PostgresMemory::new("not a postgres url", "bad-schema", "memories", Some(999), true)
        .err()
        .expect("invalid schema should fail before URL parsing");
    let message = err.to_string();

    assert!(message.contains("storage schema"));
    assert!(!message.contains("PostgreSQL connection URL"));
}

#[test]
fn new_rejects_invalid_table_before_url_parsing_or_connection() {
    let err = PostgresMemory::new("not a postgres url", "public", "bad-table", None, false)
        .err()
        .expect("invalid table should fail before URL parsing");
    let message = err.to_string();

    assert!(message.contains("storage table"));
    assert!(!message.contains("PostgreSQL connection URL"));
}

#[test]
fn new_reports_invalid_postgres_url_after_identifier_validation() {
    let err = PostgresMemory::new("not a postgres url", "public", "memories", Some(30), false)
        .err()
        .expect("invalid URL should be reported after valid identifiers");
    let message = format!("{err:#}");

    assert!(message.contains("invalid PostgreSQL connection URL"));
}

#[test]
fn quote_identifier_preserves_valid_identifier_text_inside_quotes() {
    assert_eq!(quote_identifier("public"), "\"public\"");
    assert_eq!(quote_identifier("_memories_01"), "\"_memories_01\"");
}

/// 测试在 tokio 运行时中 `PostgresMemory::new` 不会 panic
///
/// 此测试验证构造函数的健壮性：
/// - 即使连接到一个不可达的端点，也不应 panic
/// - 应该返回明确的错误而不是崩溃
///
/// # 测试策略
/// 1. 使用 `std::panic::catch_unwind` 捕获任何可能的 panic
/// 2. 连接到 `127.0.0.1:1`（极大概率不可达的端口）
/// 3. 验证返回 `Err` 而非 panic
///
/// # 注意
/// 使用 `flavor = "current_thread"` 保持单线程测试环境，
/// 避免多线程调度开销。
#[tokio::test(flavor = "current_thread")]
async fn new_does_not_panic_inside_tokio_runtime() {
    // 使用 panic::catch_unwind 包装构造函数调用
    let outcome = std::panic::catch_unwind(|| {
        PostgresMemory::new(
            "postgres://vibewindow:password@127.0.0.1:1/vibewindow", // 端口 1 几乎肯定不可达
            "public",
            "memories",
            Some(1), // 连接池大小
            false,   // 不使用 SSL
        )
    });

    // 验证：构造函数不应 panic
    assert!(outcome.is_ok(), "PostgresMemory::new should not panic");
    // 验证：对于不可达端点应返回连接错误
    assert!(
        outcome.unwrap().is_err(),
        "PostgresMemory::new should return a connect error for an unreachable endpoint"
    );
}
