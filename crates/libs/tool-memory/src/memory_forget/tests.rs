//! 记忆删除工具测试。
//!
//! 这些测试覆盖成功删除、空缺键、只读自治模式和速率限制，确保删除记忆必须经过
//! 显式安全策略检查。

use super::*;
use memory_types::memory::MemoryCategory;
use memory_sqlite::memory::sqlite::SqliteMemory;
use agent_security::{AutonomyLevel, SecurityPolicy};
use serde_json::json;
use tempfile::TempDir;

fn test_security() -> Arc<SecurityPolicy> {
    Arc::new(SecurityPolicy::default())
}

fn test_mem() -> (TempDir, Arc<dyn Memory>) {
    let tmp = TempDir::new().unwrap();
    // 每个测试使用独立 SQLite 目录，避免记忆键在用例之间串扰。
    let mem = SqliteMemory::new(tmp.path()).unwrap();
    (tmp, Arc::new(mem))
}

#[test]
fn name_and_schema() {
    let (_tmp, mem) = test_mem();
    let tool = MemoryForgetTool::new(mem, test_security());
    assert_eq!(tool.name(), "memory_forget");
    assert!(tool.parameters_schema()["properties"]["key"].is_object());
}

#[tokio::test]
async fn forget_existing() {
    let (_tmp, mem) = test_mem();
    mem.store("temp", "temporary", MemoryCategory::Conversation, None).await.unwrap();

    let tool = MemoryForgetTool::new(mem.clone(), test_security());
    let result = tool.execute(json!({"key": "temp"})).await.unwrap();
    assert!(result.success);
    assert!(result.output.contains("Forgot"));

    assert!(mem.get("temp").await.unwrap().is_none());
}

#[tokio::test]
async fn forget_nonexistent() {
    let (_tmp, mem) = test_mem();
    let tool = MemoryForgetTool::new(mem, test_security());
    let result = tool.execute(json!({"key": "nope"})).await.unwrap();
    assert!(result.success);
    assert!(result.output.contains("No memory found"));
}

#[tokio::test]
async fn forget_missing_key() {
    let (_tmp, mem) = test_mem();
    let tool = MemoryForgetTool::new(mem, test_security());
    let result = tool.execute(json!({})).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn forget_blocked_in_readonly_mode() {
    let (_tmp, mem) = test_mem();
    mem.store("temp", "temporary", MemoryCategory::Conversation, None).await.unwrap();
    let readonly =
        Arc::new(SecurityPolicy { autonomy: AutonomyLevel::ReadOnly, ..SecurityPolicy::default() });
    let tool = MemoryForgetTool::new(mem.clone(), readonly);
    let result = tool.execute(json!({"key": "temp"})).await.unwrap();
    assert!(!result.success);
    assert!(result.error.as_deref().unwrap_or("").contains("read-only mode"));
    assert!(mem.get("temp").await.unwrap().is_some());
}

#[tokio::test]
async fn forget_blocked_when_rate_limited() {
    let (_tmp, mem) = test_mem();
    mem.store("temp", "temporary", MemoryCategory::Conversation, None).await.unwrap();
    let limited = Arc::new(SecurityPolicy { max_actions_per_hour: 0, ..SecurityPolicy::default() });
    let tool = MemoryForgetTool::new(mem.clone(), limited);
    let result = tool.execute(json!({"key": "temp"})).await.unwrap();
    assert!(!result.success);
    assert!(result.error.as_deref().unwrap_or("").contains("Rate limit exceeded"));
    assert!(mem.get("temp").await.unwrap().is_some());
}
