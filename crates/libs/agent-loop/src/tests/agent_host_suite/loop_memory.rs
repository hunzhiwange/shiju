//! 循环内存测试模块
//!
//! 本模块包含针对 Agent 自动内存保存功能的集成测试。
//! 主要验证在不同配置下，Agent 与 Memory 系统的交互行为：
//!
//! - 自动保存模式下，仅保存用户消息到内存
//! - 禁用自动保存时，不存储任何消息到内存
//!
//! ## 测试覆盖范围
//!
//! 1. **auto_save_stores_only_user_messages_in_memory**:
//!    验证启用自动保存时，只保存用户输入，不保存助手响应
//!
//! 2. **auto_save_disabled_does_not_store**:
//!    验证禁用自动保存时，完全不会向内存存储任何消息

use super::helpers::{
    ScriptedProvider, build_agent_with_memory, make_sqlite_memory, text_response,
};

/// 测试：自动保存仅存储用户消息到内存
///
/// 验证当启用 auto_save 配置时，Agent 的内存保存行为：
/// - 只有用户输入的消息会被保存到内存中
/// - 助手生成的响应不会被自动保存
///
/// ## 测试步骤
///
/// 1. 创建一个 SQLite 内存后端
/// 2. 构建一个启用了 auto_save 的 Agent
/// 3. 执行一次对话轮次，用户发送 "Remember this fact"
/// 4. 验证内存中只保存了 1 条记录
/// 5. 验证用户消息被正确保存（键为 "user_msg"）
/// 6. 验证助手响应未被保存（键 "assistant_resp" 不存在）
///
/// ## 断言
///
/// - 内存计数应为 1（仅用户消息）
/// - "user_msg" 键应存在且内容匹配用户输入
/// - "assistant_resp" 键不应存在
#[tokio::test]
async fn auto_save_stores_only_user_messages_in_memory() {
    // 创建 SQLite 内存实例及临时目录
    let (mem, _tmp) = make_sqlite_memory();

    // 创建脚本化 Provider，预设返回固定文本响应
    let provider = Box::new(ScriptedProvider::new(vec![text_response("I remember everything")]));

    // 构建启用了自动保存功能的 Agent
    let mut agent = build_agent_with_memory(
        provider,
        vec![],
        mem.clone(),
        true, // 启用 auto_save
    );

    // 执行对话轮次：用户发送消息
    let _ = agent.turn("Remember this fact").await.unwrap();

    // 自动保存模式只会持久化用户输入，绝不保存助手生成的摘要
    let count = mem.count().await.unwrap();
    assert_eq!(count, 1, "预期仅有 1 条用户内存记录，实际得到 {count}");

    // 验证用户消息已正确保存
    let stored = mem.get("user_msg").await.unwrap();
    assert!(stored.is_some(), "预期 user_msg 键存在");
    assert_eq!(stored.unwrap().content, "Remember this fact", "保存的内存内容应与原始用户消息匹配");

    // 验证助手响应未被自动保存
    let assistant = mem.get("assistant_resp").await.unwrap();
    assert!(assistant.is_none(), "assistant_resp 不应被自动保存");
}

/// 测试：禁用自动保存时不存储任何消息
///
/// 验证当禁用 auto_save 配置时，Agent 完全不会向内存保存任何消息。
///
/// ## 测试步骤
///
/// 1. 创建一个 SQLite 内存后端
/// 2. 构建一个禁用了 auto_save 的 Agent
/// 3. 执行一次对话轮次
/// 4. 验证内存中没有任何记录
///
/// ## 断言
///
/// - 内存计数应为 0（不保存任何消息）
#[tokio::test]
async fn auto_save_disabled_does_not_store() {
    // 创建 SQLite 内存实例及临时目录
    let (mem, _tmp) = make_sqlite_memory();

    // 创建脚本化 Provider，预设返回简单文本
    let provider = Box::new(ScriptedProvider::new(vec![text_response("hello")]));

    // 构建禁用了自动保存功能的 Agent
    let mut agent = build_agent_with_memory(
        provider,
        vec![],
        mem.clone(),
        false, // 禁用 auto_save
    );

    // 执行对话轮次：用户发送消息
    let _ = agent.turn("test message").await.unwrap();

    // 验证禁用自动保存后，内存中没有任何记录
    let count = mem.count().await.unwrap();
    assert_eq!(count, 0, "禁用 auto_save 时应预期 0 条内存记录");
}
