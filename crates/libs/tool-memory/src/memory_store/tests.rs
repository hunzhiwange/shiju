//! memory_store 工具的单元测试模块
//!
//! 本模块包含对 `MemoryStoreTool` 的全面测试，覆盖以下场景：
//! - 工具名称和参数 schema 验证
//! - 核心存储功能（基本键值对存储）
//! - 带类别标签的记忆存储
//! - 自定义类别的记忆存储
//! - 缺失必需参数的错误处理
//! - 只读模式下的写入拦截
//! - 速率限制下的写入拦截
//!
//! # 测试依赖
//!
//! - `SqliteMemory`：基于 SQLite 的内存存储后端
//! - `SecurityPolicy`：安全策略，用于控制工具执行权限
//! - `TempDir`：临时目录，用于隔离测试环境

use super::*;
use memory_types::memory::MemoryCategory;
use memory_sqlite::memory::sqlite::SqliteMemory;
use agent_security::{AutonomyLevel, SecurityPolicy};
use serde_json::json;
use tempfile::TempDir;

/// 创建默认的安全策略用于测试
///
/// 返回一个使用默认配置的 `SecurityPolicy` 的 Arc 智能指针，
/// 允许所有操作且无额外限制。
///
/// # 返回值
///
/// 返回包装在 `Arc` 中的默认安全策略实例
fn test_security() -> Arc<SecurityPolicy> {
    Arc::new(SecurityPolicy::default())
}

/// 创建测试用的内存存储实例
///
/// 在临时目录中初始化一个 SQLite 后端的内存存储，
/// 确保测试之间相互隔离，不会互相影响。
///
/// # 返回值
///
/// 返回一个元组：
/// - `TempDir`：临时目录句柄，离开作用域时会自动清理
/// - `Arc<dyn Memory>`：内存存储的 trait 对象
///
/// # Panics
///
/// 如果临时目录创建失败或 SQLite 初始化失败，将触发 panic
fn test_mem() -> (TempDir, Arc<dyn Memory>) {
    let tmp = TempDir::new().unwrap();
    let mem = SqliteMemory::new(tmp.path()).unwrap();
    (tmp, Arc::new(mem))
}

/// 测试工具名称和参数 schema 的正确性
///
/// 验证 `MemoryStoreTool` 的以下属性：
/// 1. 工具名称应为 "memory_store"
/// 2. 参数 schema 应包含 "key" 属性定义
/// 3. 参数 schema 应包含 "content" 属性定义
#[test]
fn name_and_schema() {
    let (_tmp, mem) = test_mem();
    let tool = MemoryStoreTool::new(mem, test_security());

    // 验证工具名称
    assert_eq!(tool.name(), "memory_store");

    // 验证参数 schema 包含必需的字段
    let schema = tool.parameters_schema();
    assert!(schema["properties"]["key"].is_object());
    assert!(schema["properties"]["content"].is_object());
}

/// 测试核心存储功能
///
/// 验证基本的键值对存储操作：
/// 1. 使用 key 和 content 参数存储记忆
/// 2. 执行结果应标记为成功
/// 3. 输出中应包含 key 信息
/// 4. 通过内存后端直接查询，验证数据已正确存储
#[tokio::test]
async fn store_core() {
    let (_tmp, mem) = test_mem();
    let tool = MemoryStoreTool::new(mem.clone(), test_security());

    // 执行存储操作
    let result = tool.execute(json!({"key": "lang", "content": "Prefers Rust"})).await.unwrap();
    assert!(result.success);
    assert!(result.output.contains("lang"));

    // 直接从内存后端验证数据已存储
    let entry = mem.get("lang").await.unwrap();
    assert!(entry.is_some());
    assert_eq!(entry.unwrap().content, "Prefers Rust");
}

/// 测试带预定义类别标签的存储
///
/// 验证在指定 category 参数时，记忆可以正确归类存储。
/// 使用预定义的 "daily" 类别进行测试。
#[tokio::test]
async fn store_with_category() {
    let (_tmp, mem) = test_mem();
    let tool = MemoryStoreTool::new(mem.clone(), test_security());

    // 使用 daily 类别存储记忆
    let result = tool
        .execute(json!({"key": "note", "content": "Fixed bug", "category": "daily"}))
        .await
        .unwrap();

    assert!(result.success);
}

/// 测试带自定义类别的存储
///
/// 验证自定义类别（非预定义类别）的存储功能：
/// 1. 使用自定义 "project" 类别存储记忆
/// 2. 验证执行成功
/// 3. 通过后端验证存储的内容和类别都正确
#[tokio::test]
async fn store_with_custom_category() {
    let (_tmp, mem) = test_mem();
    let tool = MemoryStoreTool::new(mem.clone(), test_security());

    // 使用自定义 project 类别存储记忆
    let result = tool
        .execute(
            json!({"key": "proj_note", "content": "Uses async runtime", "category": "project"}),
        )
        .await
        .unwrap();
    assert!(result.success);

    // 验证内容和类别都已正确存储
    let entry = mem.get("proj_note").await.unwrap().unwrap();
    assert_eq!(entry.content, "Uses async runtime");
    assert_eq!(entry.category, MemoryCategory::Custom("project".into()));
}

/// 测试缺失 key 参数时的错误处理
///
/// 验证当缺少必需的 "key" 参数时，工具应返回错误而非静默失败。
#[tokio::test]
async fn store_missing_key() {
    let (_tmp, mem) = test_mem();
    let tool = MemoryStoreTool::new(mem, test_security());

    // 仅提供 content，缺少必需的 key 参数
    let result = tool.execute(json!({"content": "no key"})).await;
    assert!(result.is_err());
}

/// 测试缺失 content 参数时的错误处理
///
/// 验证当缺少必需的 "content" 参数时，工具应返回错误。
#[tokio::test]
async fn store_missing_content() {
    let (_tmp, mem) = test_mem();
    let tool = MemoryStoreTool::new(mem, test_security());

    // 仅提供 key，缺少必需的 content 参数
    let result = tool.execute(json!({"key": "no_content"})).await;
    assert!(result.is_err());
}

/// 测试只读模式下的写入拦截
///
/// 验证在安全策略设置为只读模式时：
/// 1. 存储操作应被拦截，返回失败结果
/// 2. 错误信息应明确说明是只读模式限制
/// 3. 内存后端中不应有实际数据写入
#[tokio::test]
async fn store_blocked_in_readonly_mode() {
    let (_tmp, mem) = test_mem();

    // 配置只读模式的安全策略
    let readonly =
        Arc::new(SecurityPolicy { autonomy: AutonomyLevel::ReadOnly, ..SecurityPolicy::default() });
    let tool = MemoryStoreTool::new(mem.clone(), readonly);

    // 尝试在只读模式下执行存储
    let result = tool.execute(json!({"key": "lang", "content": "Prefers Rust"})).await.unwrap();

    // 验证操作被拦截
    assert!(!result.success);
    assert!(result.error.as_deref().unwrap_or("").contains("read-only mode"));

    // 验证数据未写入内存后端
    assert!(mem.get("lang").await.unwrap().is_none());
}

/// 测试速率限制下的写入拦截
///
/// 验证当安全策略设置每小时最大操作数为 0 时：
/// 1. 存储操作应因速率限制被拦截
/// 2. 错误信息应明确说明是速率限制
/// 3. 内存后端中不应有实际数据写入
#[tokio::test]
async fn store_blocked_when_rate_limited() {
    let (_tmp, mem) = test_mem();

    // 配置速率限制为 0 的安全策略（禁止任何操作）
    let limited = Arc::new(SecurityPolicy { max_actions_per_hour: 0, ..SecurityPolicy::default() });
    let tool = MemoryStoreTool::new(mem.clone(), limited);

    // 尝试在速率限制下执行存储
    let result = tool.execute(json!({"key": "lang", "content": "Prefers Rust"})).await.unwrap();

    // 验证操作被拦截
    assert!(!result.success);
    assert!(result.error.as_deref().unwrap_or("").contains("Rate limit exceeded"));

    // 验证数据未写入内存后端
    assert!(mem.get("lang").await.unwrap().is_none());
}
