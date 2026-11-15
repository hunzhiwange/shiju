//! SQLite 迁移功能测试模块
//!
//! 本模块提供 SQLite 数据库迁移相关的单元测试，主要验证从旧版 OpenClaw 数据库
//! 读取记忆条目的兼容性。测试覆盖以下场景：
//!
//! - 旧版数据库 schema 兼容性（value 列）
//! - 空内容条目的过滤处理
//!
//! 这些测试确保迁移工具能够正确处理各种边界情况和历史数据格式。

use super::super::*;
use agent_memory::MemoryCategory;
use rusqlite::params;
use tempfile::TempDir;

/// 测试 SQLite 读取器对旧版 value 列的支持
///
/// 此测试验证迁移工具能够正确读取使用旧版 schema 的 SQLite 数据库，
/// 其中内容列名为 "value" 而非新版的 "content"。
///
/// # 测试场景
///
/// 1. 创建使用旧版 schema 的临时 SQLite 数据库
/// 2. 插入一条使用 value 列的记忆条目
/// 3. 验证读取器能够正确解析数据并映射到新结构
///
/// # 期望结果
///
/// - 成功读取所有条目
/// - key 字段正确映射
/// - value 字段正确映射到 content
/// - type 字段正确映射到 MemoryCategory 枚举
#[test]
fn sqlite_reader_supports_legacy_value_column() {
    // 创建临时目录和数据库文件路径
    let dir = TempDir::new().unwrap();
    let db_path = dir.path().join("brain.db");

    // 建立数据库连接
    let conn = Connection::open(&db_path).unwrap();

    // 创建旧版 schema 的 memories 表
    // 注意：使用 "value" 而非 "content" 作为内容列名
    conn.execute_batch("CREATE TABLE memories (key TEXT, value TEXT, type TEXT);").unwrap();

    // 插入测试数据，模拟旧版数据库格式
    conn.execute(
        "INSERT INTO memories (key, value, type) VALUES (?1, ?2, ?3)",
        params!["legacy_key", "legacy_value", "daily"],
    )
    .unwrap();

    // 调用读取函数解析数据库条目
    let rows = read_openclaw_sqlite_entries(&db_path).unwrap();

    // 验证读取结果
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].key, "legacy_key");
    assert_eq!(rows[0].content, "legacy_value");
    assert_eq!(rows[0].category, MemoryCategory::Daily);
}

/// 测试迁移时跳过空内容条目
///
/// 此测试验证迁移工具能够正确过滤掉内容为空或仅包含空白字符的记忆条目，
/// 防止无效数据被迁移到新系统中。
///
/// # 测试场景
///
/// 1. 创建临时 SQLite 数据库
/// 2. 插入一条内容为纯空白字符的记忆条目
/// 3. 验证读取器正确跳过该条目
///
/// # 期望结果
///
/// - 返回的条目列表为空
/// - 空白内容被正确识别并过滤
#[tokio::test]
async fn migration_skips_empty_content() {
    // 创建临时目录和数据库文件路径
    let dir = TempDir::new().unwrap();
    let db_path = dir.path().join("brain.db");

    // 建立数据库连接
    let conn = Connection::open(&db_path).unwrap();

    // 创建新版 schema 的 memories 表
    conn.execute_batch("CREATE TABLE memories (key TEXT, content TEXT, category TEXT);").unwrap();

    // 插入内容为纯空白字符的测试数据
    conn.execute(
        "INSERT INTO memories (key, content, category) VALUES (?1, ?2, ?3)",
        params!["empty_key", "   ", "core"],
    )
    .unwrap();

    // 调用读取函数解析数据库条目
    let rows = read_openclaw_sqlite_entries(&db_path).unwrap();

    // 验证空内容条目被正确跳过
    assert_eq!(rows.len(), 0, "entries with empty/whitespace content must be skipped");
}
