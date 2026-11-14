//! Markdown 记忆后端的单元测试模块
//!
//! 本模块包含了对 `MarkdownMemory` 实现的全面测试用例，覆盖以下核心功能：
//! - 基础元数据操作（名称获取、健康检查）
//! - 存储功能（核心记忆、日常记忆）
//! - 检索功能（关键词搜索、空结果处理）
//! - 列表和计数功能
//! - 遗忘操作的幂等性验证
//!
//! 所有测试均使用临时目录作为工作空间，确保测试隔离性和可重复性。

use super::*;
use super::test_support::TempWorkspace as TempDir;

/// 创建临时工作空间和 MarkdownMemory 实例
///
/// 为每个测试用例创建独立的临时目录和对应的 MarkdownMemory 实例，
/// 确保测试之间完全隔离，不会相互干扰。
///
/// # 返回值
///
/// 返回一个元组：
/// - `TempDir`: 临时目录句柄，离开作用域时会自动清理
/// - `MarkdownMemory`: 配置好路径的 MarkdownMemory 实例
fn temp_workspace() -> (TempDir, MarkdownMemory) {
    let tmp = TempDir::new().unwrap();
    let mem = MarkdownMemory::new(tmp.path());
    (tmp, mem)
}

#[test]
fn markdown_memory_uses_user_scoped_data_dir() {
    let workspace = TempDir::new().unwrap();
    let storage = paths::project_data_dir(workspace.path()).unwrap();
    let mem = MarkdownMemory::new(workspace.path());

    assert!(mem.core_path().starts_with(&storage));
    assert!(!mem.core_path().starts_with(workspace.path()));
}

/// 测试 MarkdownMemory 的名称标识
///
/// 验证 MarkdownMemory 实例返回正确的后端名称标识符 "markdown"，
/// 该标识符用于在多后端环境中识别和路由记忆操作。
#[tokio::test]
async fn markdown_name() {
    let (_tmp, mem) = temp_workspace();
    assert_eq!(mem.name(), "markdown");
}

/// 测试健康检查功能
///
/// 验证 MarkdownMemory 的健康检查始终返回 true，
/// 表示基于文件系统的 Markdown 后端始终可用且健康。
#[tokio::test]
async fn markdown_health_check() {
    let (_tmp, mem) = temp_workspace();
    assert!(mem.health_check().await);
}

/// 测试核心记忆的存储功能
///
/// 验证将记忆项存储到 Core 类别时：
/// 1. 文件操作成功完成
/// 2. 内容正确写入到核心记忆文件
/// 3. 可以通过读取文件验证存储的内容
#[tokio::test]
async fn markdown_store_core() {
    let (_tmp, mem) = temp_workspace();
    mem.store("pref", "User likes Rust", MemoryCategory::Core, None).await.unwrap();
    let content = tokio::fs::read_to_string(mem.core_path()).await.unwrap();
    assert!(content.contains("User likes Rust"));
}

/// 测试日常记忆的存储功能
///
/// 验证将记忆项存储到 Daily 类别时：
/// 1. 文件操作成功完成
/// 2. 内容正确写入到日常记忆文件
/// 3. 可以通过读取文件验证存储的内容
#[tokio::test]
async fn markdown_store_daily() {
    let (_tmp, mem) = temp_workspace();
    mem.store("note", "Finished tests", MemoryCategory::Daily, None).await.unwrap();
    let path = mem.daily_path();
    let content = tokio::fs::read_to_string(path).await.unwrap();
    assert!(content.contains("Finished tests"));
}

/// 测试基于关键词的检索功能
///
/// 验证检索操作能够：
/// 1. 正确匹配包含指定关键词的记忆项
/// 2. 返回所有匹配的结果（不遗漏）
/// 3. 每个返回结果都确实包含搜索关键词（不误报）
#[tokio::test]
async fn markdown_recall_keyword() {
    let (_tmp, mem) = temp_workspace();
    mem.store("a", "Rust is fast", MemoryCategory::Core, None).await.unwrap();
    mem.store("b", "Python is slow", MemoryCategory::Core, None).await.unwrap();
    mem.store("c", "Rust and safety", MemoryCategory::Core, None).await.unwrap();

    let results = mem.recall("Rust", 10, None).await.unwrap();
    assert!(results.len() >= 2);
    assert!(results.iter().all(|r| r.content.to_lowercase().contains("rust")));
}

/// 测试无匹配结果的检索场景
///
/// 验证当搜索关键词不匹配任何已存储记忆时：
/// 1. 操作正常完成（不报错）
/// 2. 返回空的结果集
#[tokio::test]
async fn markdown_recall_no_match() {
    let (_tmp, mem) = temp_workspace();
    mem.store("a", "Rust is great", MemoryCategory::Core, None).await.unwrap();
    let results = mem.recall("javascript", 10, None).await.unwrap();
    assert!(results.is_empty());
}

/// 测试记忆项计数功能
///
/// 验证计数操作能够正确统计已存储的记忆项数量，
/// 即使多次存储，计数也应准确反映实际存储的数量。
#[tokio::test]
async fn markdown_count() {
    let (_tmp, mem) = temp_workspace();
    mem.store("a", "first", MemoryCategory::Core, None).await.unwrap();
    mem.store("b", "second", MemoryCategory::Core, None).await.unwrap();
    let count = mem.count().await.unwrap();
    assert!(count >= 2);
}

/// 测试按类别列出记忆项功能
///
/// 验证列表操作能够：
/// 1. 正确按类别过滤记忆项
/// 2. 仅返回指定类别的记忆
/// 3. 不同类别的记忆项互不干扰
#[tokio::test]
async fn markdown_list_by_category() {
    let (_tmp, mem) = temp_workspace();
    mem.store("a", "core fact", MemoryCategory::Core, None).await.unwrap();
    mem.store("b", "daily note", MemoryCategory::Daily, None).await.unwrap();

    let core = mem.list(Some(&MemoryCategory::Core), None).await.unwrap();
    assert!(core.iter().all(|e| e.category == MemoryCategory::Core));

    let daily = mem.list(Some(&MemoryCategory::Daily), None).await.unwrap();
    assert!(daily.iter().all(|e| e.category == MemoryCategory::Daily));
}

/// 测试遗忘操作的幂等性
///
/// 验证 Markdown 后端的遗忘操作特性：
/// - 由于 Markdown 是仅追加的存储格式，forget 操作是空操作（no-op）
/// - 操作不会报错，但也不实际删除内容
/// - 返回 false 表示没有实际删除任何内容
///
/// 这符合 Markdown 文件作为审计日志的设计理念。
#[tokio::test]
async fn markdown_forget_is_noop() {
    let (_tmp, mem) = temp_workspace();
    mem.store("a", "permanent", MemoryCategory::Core, None).await.unwrap();
    let removed = mem.forget("a").await.unwrap();
    assert!(!removed, "Markdown memory is append-only");
}

/// 测试空存储时的检索行为
///
/// 验证在没有任何记忆存储的情况下进行检索：
/// 1. 操作正常完成（不报错）
/// 2. 返回空的结果集
/// 3. 不会产生任何副作用
#[tokio::test]
async fn markdown_empty_recall() {
    let (_tmp, mem) = temp_workspace();
    let results = mem.recall("anything", 10, None).await.unwrap();
    assert!(results.is_empty());
}

/// 测试空存储时的计数行为
///
/// 验证在没有任何记忆存储的情况下进行计数：
/// 1. 操作正常完成（不报错）
/// 2. 返回 0
#[tokio::test]
async fn markdown_empty_count() {
    let (_tmp, mem) = temp_workspace();
    assert_eq!(mem.count().await.unwrap(), 0);
}
