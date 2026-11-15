//! # MemoryRecallTool 单元测试模块
//!
//! 本模块提供对 `MemoryRecallTool` 的综合测试覆盖，验证记忆检索功能的正确性。
//!
//! ## 测试范围
//!
//! - **空记忆检索**：验证在无记忆数据时的行为
//! - **匹配查找**：验证查询能正确匹配存储的记忆
//! - **结果限制**：验证 `limit` 参数正确限制返回数量
//! - **参数校验**：验证缺少必需参数时正确报错
//! - **元数据验证**：验证工具名称和 JSON Schema 正确性
//!
//! ## 依赖项
//!
//! - `SqliteMemory`：使用 SQLite 作为测试用内存后端
//! - `TempDir`：为每个测试创建独立的临时目录，确保测试隔离

use super::*;
use memory_types::memory::MemoryCategory;
use memory_sqlite::memory::sqlite::SqliteMemory;
use serde_json::json;
use tempfile::TempDir;

/// 创建带有临时存储目录的内存实例
///
/// 初始化一个使用临时目录的 `SqliteMemory` 实例，用于测试隔离。
/// 临时目录会在 `_tmp` 变量离开作用域时自动清理。
///
/// # 返回值
///
/// 返回元组 `(TempDir, Arc<dyn Memory>)`：
/// - `TempDir`：临时目录句柄，必须保持存活直到测试结束
/// - `Arc<dyn Memory>`：线程安全的内存接口实例
///
/// # 示例
///
/// ```ignore
/// let (_tmp, mem) = seeded_mem();
/// // 使用 mem 进行测试...
/// // _tmp 离开作用域时自动清理临时目录
/// ```
fn seeded_mem() -> (TempDir, Arc<dyn Memory>) {
    let tmp = TempDir::new().unwrap();
    let mem = SqliteMemory::new(tmp.path()).unwrap();
    (tmp, Arc::new(mem))
}

/// 测试空记忆库的检索行为
///
/// 验证在没有任何存储记忆的情况下执行查询时：
/// - 操作成功完成（`success` 为 `true`）
/// - 输出包含 "No memories found" 提示信息
#[tokio::test]
async fn recall_empty() {
    let (_tmp, mem) = seeded_mem();
    let tool = MemoryRecallTool::new(mem);
    let result = tool.execute(json!({"query": "anything"})).await.unwrap();
    assert!(result.success);
    assert!(result.output.contains("No memories found"));
}

/// 测试正常记忆匹配检索
///
/// 验证检索功能能够：
/// 1. 正确匹配包含查询关键词的记忆条目
/// 2. 返回包含匹配内容的输出
/// 3. 在输出中显示正确的匹配数量
///
/// # 测试步骤
///
/// 1. 存储两条记忆：用户偏好 Rust、时区为 EST
/// 2. 使用查询词 "Rust" 执行检索
/// 3. 验证返回 1 条匹配结果，且包含 "Rust" 内容
#[tokio::test]
async fn recall_finds_match() {
    let (_tmp, mem) = seeded_mem();

    // 存储测试用的记忆条目
    mem.store("lang", "User prefers Rust", MemoryCategory::Core, None).await.unwrap();
    mem.store("tz", "Timezone is EST", MemoryCategory::Core, None).await.unwrap();

    let tool = MemoryRecallTool::new(mem);
    let result = tool.execute(json!({"query": "Rust"})).await.unwrap();

    // 验证检索成功且返回了正确的结果
    assert!(result.success);
    assert!(result.output.contains("Rust"));
    assert!(result.output.contains("Found 1"));
}

/// 测试结果数量限制功能
///
/// 验证 `limit` 参数能够正确限制返回的记忆条目数量。
///
/// # 测试步骤
///
/// 1. 存储 10 条包含 "Rust" 关键词的记忆
/// 2. 使用 `limit: 3` 执行查询
/// 3. 验证仅返回 3 条结果（而非全部 10 条）
///
/// # 目的
///
/// 防止大量匹配结果导致输出过长或性能问题
#[tokio::test]
async fn recall_respects_limit() {
    let (_tmp, mem) = seeded_mem();

    // 批量存储 10 条包含 "Rust" 的记忆
    for i in 0..10 {
        mem.store(&format!("k{i}"), &format!("Rust fact {i}"), MemoryCategory::Core, None)
            .await
            .unwrap();
    }

    let tool = MemoryRecallTool::new(mem);
    let result = tool.execute(json!({"query": "Rust", "limit": 3})).await.unwrap();

    // 验证限制生效，仅返回 3 条结果
    assert!(result.success);
    assert!(result.output.contains("Found 3"));
}

/// 测试缺少必需参数时的错误处理
///
/// 验证当调用时未提供必需的 `query` 参数，工具应返回错误而非静默失败。
///
/// # 预期行为
///
/// - 执行应返回 `Err`，表明参数校验失败
#[tokio::test]
async fn recall_missing_query() {
    let (_tmp, mem) = seeded_mem();
    let tool = MemoryRecallTool::new(mem);

    // 不提供 query 参数，应返回错误
    let result = tool.execute(json!({})).await;
    assert!(result.is_err());
}

/// 测试工具元数据：名称和参数 Schema
///
/// 验证工具的基础元数据配置正确：
/// - 工具名称应为 "memory_recall"
/// - 参数 Schema 中应包含 "query" 属性定义
///
/// # 目的
///
/// 确保工具能被正确注册和调用，参数 Schema 符合预期结构
#[test]
fn name_and_schema() {
    let (_tmp, mem) = seeded_mem();
    let tool = MemoryRecallTool::new(mem);

    // 验证工具名称
    assert_eq!(tool.name(), "memory_recall");

    // 验证参数 Schema 包含必需的 query 属性
    assert!(tool.parameters_schema()["properties"]["query"].is_object());
}
