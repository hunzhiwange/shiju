//! MariaDB 内存后端单元测试与集成测试模块
//!
//! 本模块提供 MariaDB 内存存储后端的测试覆盖，包括：
//! - 标识符验证测试（表名、模式名等）
//! - 分类解析测试（核心、日常、对话等类型）
//! - 模式名规范化测试
//! - 构造函数健壮性测试（确保不发生 panic）
//! - 完整的存储/读取/召回集成测试
//!
//! # 测试分类
//!
//! - **单元测试**：验证单个函数/方法的行为（如 `valid_identifiers_pass_validation`）
//! - **集成测试**：需要真实 MariaDB 实例的端到端测试（如 `integration_roundtrip_when_test_db_is_configured`）
//!
//! # 环境变量
//!
//! 集成测试需要设置 `VIBEWINDOW_TEST_MARIADB_URL` 环境变量，
//! 格式示例：`mysql://user:password@host:port/database`

use super::*;

/// 测试有效标识符能够通过验证
///
/// 验证以下有效的表名/模式名格式：
/// - 普通字母数字组合（如 "memories"）
/// - 带下划线的标识符（如 "_memories_01"）
#[test]
fn valid_identifiers_pass_validation() {
    assert!(validate_identifier("memories", "table").is_ok());
    assert!(validate_identifier("_memories_01", "table").is_ok());
}

/// 测试无效标识符被正确拒绝
///
/// 验证以下无效格式会被拒绝：
/// - 空字符串（""）
/// - 以数字开头的标识符（"1bad"）
/// - 包含连字符的标识符（"bad-name"）
#[test]
fn invalid_identifiers_are_rejected() {
    assert!(validate_identifier("", "schema").is_err());
    assert!(validate_identifier("1bad", "schema").is_err());
    assert!(validate_identifier("bad-name", "table").is_err());
}

#[test]
fn identifier_validation_accepts_ascii_boundaries() {
    for valid in ["_", "A", "z9_", "_0"] {
        assert!(validate_identifier(valid, "table").is_ok(), "{valid} should be valid");
    }
}

#[test]
fn identifier_validation_errors_include_field_value_and_failed_rule() {
    let invalid_prefix = validate_identifier("数据库", "storage schema").unwrap_err().to_string();
    assert_eq!(
        invalid_prefix,
        "storage schema must start with an ASCII letter or underscore; got '数据库'"
    );

    let invalid_body = validate_identifier("table名", "storage table").unwrap_err().to_string();
    assert_eq!(
        invalid_body,
        "storage table can only contain ASCII letters, numbers, and underscores; got 'table名'"
    );
}

/// 测试分类字符串解析能够正确映射已知和自定义值
///
/// 验证以下映射关系：
/// - "core" -> MemoryCategory::Core（核心记忆）
/// - "daily" -> MemoryCategory::Daily（日常记忆）
/// - "conversation" -> MemoryCategory::Conversation（对话记忆）
/// - 其他值 -> MemoryCategory::Custom（自定义分类）
#[test]
fn parse_category_maps_known_and_custom_values() {
    assert_eq!(MariadbMemory::parse_category("core"), MemoryCategory::Core);
    assert_eq!(MariadbMemory::parse_category("daily"), MemoryCategory::Daily);
    assert_eq!(MariadbMemory::parse_category("conversation"), MemoryCategory::Conversation);
    assert_eq!(
        MariadbMemory::parse_category("custom_notes"),
        MemoryCategory::Custom("custom_notes".into())
    );
}

#[test]
fn custom_category_serialization_preserves_text_and_reserved_name_semantics() {
    let custom = MemoryCategory::Custom("团队_notes".into());
    let serialized = MariadbMemory::category_to_str(&custom);
    assert_eq!(serialized, "团队_notes");
    assert_eq!(
        MariadbMemory::parse_category(&serialized),
        MemoryCategory::Custom("团队_notes".into())
    );

    let reserved = MariadbMemory::category_to_str(&MemoryCategory::Custom("core".into()));
    assert_eq!(reserved, "core");
    assert_eq!(MariadbMemory::parse_category(&reserved), MemoryCategory::Core);
}

/// 测试模式名规范化处理 PostgreSQL 默认模式
///
/// 规范化规则：
/// - 空字符串 -> None（使用默认）
/// - "public" 或 "PUBLIC" -> None（PostgreSQL 默认模式，相当于未指定）
/// - 其他值 -> Some(值)（使用自定义模式名）
///
/// 注意：虽然这是 MariaDB 后端，但保留对 PostgreSQL 默认模式名的兼容处理
#[test]
fn normalize_schema_handles_default_postgres_schema() {
    assert!(normalize_schema("").is_none());
    assert!(normalize_schema("public").is_none());
    assert!(normalize_schema("PUBLIC").is_none());
    assert_eq!(normalize_schema("vibewindow"), Some("vibewindow".into()));
}

#[test]
fn normalize_schema_trims_mixed_case_default_and_custom_values() {
    assert_eq!(normalize_schema("\tPuBlIc\n"), None);
    assert_eq!(normalize_schema("\tAudit_Log\n"), Some("Audit_Log".into()));
}

#[test]
fn name_reports_backend_without_opening_a_connection() {
    let pool_options =
        mysql::PoolOpts::default().with_constraints(mysql::PoolConstraints::new_const::<0, 1>());
    let options = OptsBuilder::default()
        .ip_or_hostname(Some("127.0.0.1"))
        .tcp_port(1)
        .prefer_socket(false)
        .pool_opts(pool_options);
    let memory = MariadbMemory {
        pool: Pool::new(options).expect("a pool with zero minimum connections should be lazy"),
        qualified_table: "`memories`".into(),
    };

    assert_eq!(memory.name(), "mariadb");
}

/// 测试在 Tokio 运行时中构造函数不会发生 panic
///
/// 此测试验证即使连接 URL 无效，`MariadbMemory::new` 也应该返回错误
/// 而不是在 Tokio 运行时中发生 panic。
///
/// # 测试策略
///
/// 1. 使用 `std::panic::catch_unwind` 捕获任何可能的 panic
/// 2. 确保构造函数调用本身不会 panic
/// 3. 确保无效 URL 返回解析错误且不发起网络连接
#[tokio::test(flavor = "current_thread")]
async fn new_does_not_panic_inside_tokio_runtime() {
    // 使用 catch_unwind 包装构造函数调用，捕获可能的 panic
    let outcome = std::panic::catch_unwind(|| {
        MariadbMemory::new("not a mysql url", "public", "memories", Some(1), false)
    });

    // 验证构造函数没有 panic
    assert!(outcome.is_ok(), "MariadbMemory::new should not panic");
    let error = match outcome.unwrap() {
        Ok(_) => panic!("invalid URL should fail"),
        Err(error) => error.to_string(),
    };
    assert!(error.contains("invalid MariaDB connection URL"));
}

/// MariaDB 完整存储往返集成测试
///
/// 此测试执行完整的存储后端生命周期验证：
/// 1. 初始化 MariaDB 内存后端
/// 2. 存储一条测试记忆
/// 3. 通过键获取存储的记忆
/// 4. 通过关键词召回相关记忆
///
/// # 前置条件
///
/// 必须设置 `VIBEWINDOW_TEST_MARIADB_URL` 环境变量。
/// 如果未设置，测试将被跳过。
///
/// # 测试隔离
///
/// 每次运行使用唯一的模式名（带 UUID）以确保测试隔离，
/// 避免并行测试之间的数据干扰。
///
/// # 示例
///
/// ```bash
/// export VIBEWINDOW_TEST_MARIADB_URL="mysql://root:password@localhost:3306/test"
/// cargo test --features mariadb
/// ```
#[tokio::test(flavor = "current_thread")]
async fn integration_roundtrip_when_test_db_is_configured() {
    // 检查是否配置了测试数据库 URL，未配置则跳过测试
    let Some(db_url) =
        std::env::var("VIBEWINDOW_TEST_MARIADB_URL").ok().filter(|value| !value.trim().is_empty())
    else {
        eprintln!("Skipping MariaDB integration test: set VIBEWINDOW_TEST_MARIADB_URL to enable");
        return;
    };

    // 生成唯一的测试模式名，确保测试隔离
    let schema = format!("vibewindow_test_{}", Uuid::new_v4().simple());

    // 初始化 MariaDB 内存后端
    let memory = MariadbMemory::new(&db_url, &schema, "memories", Some(5), false)
        .expect("should initialize MariaDB memory backend");

    // 阶段 1：存储测试数据
    memory
        .store("integration_key", "integration content", MemoryCategory::Conversation, None)
        .await
        .expect("store should succeed");

    // 阶段 2：通过键获取数据，验证存储成功
    let fetched = memory
        .get("integration_key")
        .await
        .expect("get should succeed")
        .expect("entry should exist");
    assert_eq!(fetched.content, "integration content");

    // 阶段 3：通过关键词召回数据，验证语义检索功能
    let recalled = memory.recall("integration", 5, None).await.expect("recall should succeed");
    assert!(
        recalled.iter().any(|entry| entry.key == "integration_key"),
        "recall should return the stored key"
    );
}
