//! 历史管理功能测试模块
//!
//! 本模块包含对通道历史记录管理相关功能的单元测试，主要覆盖以下功能：
//! - 上下文窗口溢出错误检测
//! - 记忆上下文跳过规则
//! - 缓存的通道对话轮次归一化处理
//!
//! 这些测试确保历史记录在不同场景下的正确处理，包括：
//! - 连续同角色消息的合并
//! - 失败/超时标记的保留
//! - 空输入的边界情况处理

use super::*;

/// 测试上下文窗口溢出错误检测功能
///
/// 验证 `is_context_window_overflow_error` 函数能够正确识别：
/// - 真正的上下文窗口溢出错误（应返回 true）
/// - 其他类型的 API 错误（应返回 false）
///
/// # 测试场景
/// 1. 包含 "exceeds the context window" 关键字的错误 → 应被识别为溢出错误
/// 2. 普通的 502 Bad Gateway 错误 → 不应被识别为溢出错误
#[test]
fn context_window_overflow_error_detector_matches_known_messages() {
    // 构造一个上下文窗口溢出错误
    let overflow_err = anyhow::anyhow!(
        "OpenAI Codex stream error: Your input exceeds the context window of this model."
    );
    // 验证能够正确识别为溢出错误
    assert!(is_context_window_overflow_error(&overflow_err));

    // 构造一个非溢出类型的错误（502 网关错误）
    let other_err = anyhow::anyhow!("OpenAI Codex API error (502 Bad Gateway): error code: 502");
    // 验证不会被误识别为溢出错误
    assert!(!is_context_window_overflow_error(&other_err));
}

/// 测试记忆上下文跳过规则
///
/// 验证 `should_skip_memory_context_entry` 函数能够正确判断哪些记忆条目应该被跳过：
/// - 历史记录 blob（以 `_history` 结尾的键）应被跳过
/// - 旧的助手响应遗留数据应被跳过
/// - 正常的对话记录不应被跳过
///
/// # 测试场景
/// 1. `telegram_123_history` → 应跳过（历史 blob）
/// 2. `assistant_resp_legacy` → 应跳过（旧格式遗留数据）
/// 3. `telegram_123_45` → 不应跳过（正常对话记录）
#[test]
fn memory_context_skip_rules_exclude_history_blobs() {
    // 历史记录 blob 应该被跳过
    assert!(should_skip_memory_context_entry("telegram_123_history", r#"[{"role":"user"}]"#));
    // 旧格式的助手响应遗留数据应该被跳过
    assert!(should_skip_memory_context_entry("assistant_resp_legacy", "fabricated memory"));
    // 正常的对话记录不应该被跳过
    assert!(!should_skip_memory_context_entry("telegram_123_45", "hi"));
}

/// 测试连续用户消息的合并功能
///
/// 验证 `normalize_cached_channel_turns` 函数能够正确合并连续的用户消息：
/// - 多条连续的用户消息应合并为一条
/// - 合并后的内容应包含所有原始消息内容
/// - 合并后的角色仍为 "user"
///
/// # 测试场景
/// 输入：两条连续的用户消息 ["forwarded content", "summarize this"]
/// 预期输出：一条用户消息，内容包含两段文本
#[test]
fn normalize_cached_channel_turns_merges_consecutive_user_turns() {
    // 构造两条连续的用户消息
    let turns = vec![ChatMessage::user("forwarded content"), ChatMessage::user("summarize this")];

    // 执行归一化处理
    let normalized = normalize_cached_channel_turns(turns);

    // 验证结果：应该合并为 1 条消息
    assert_eq!(normalized.len(), 1);
    assert_eq!(normalized[0].role, "user");
    // 验证合并后的内容包含两段原始文本
    assert!(normalized[0].content.contains("forwarded content"));
    assert!(normalized[0].content.contains("summarize this"));
}

/// 测试连续助手消息的合并功能
///
/// 验证 `normalize_cached_channel_turns` 函数能够正确合并连续的助手消息：
/// - 多条连续的助手消息应合并为一条
/// - 合并后的内容应包含所有原始消息内容
/// - 保持用户消息和助手消息的交替顺序
///
/// # 测试场景
/// 输入：[user, assistant, assistant, user] 四条消息
/// 预期输出：[user, assistant, user] 三条消息，其中两条助手消息被合并
#[test]
fn normalize_cached_channel_turns_merges_consecutive_assistant_turns() {
    // 构造包含连续助手消息的对话序列
    let turns = vec![
        ChatMessage::user("first user"),
        ChatMessage::assistant("assistant part 1"),
        ChatMessage::assistant("assistant part 2"),
        ChatMessage::user("next user"),
    ];

    // 执行归一化处理
    let normalized = normalize_cached_channel_turns(turns);

    // 验证结果：应该有 3 条消息（两条助手消息被合并）
    assert_eq!(normalized.len(), 3);
    assert_eq!(normalized[0].role, "user");
    assert_eq!(normalized[1].role, "assistant");
    assert_eq!(normalized[2].role, "user");
    // 验证合并后的助手消息包含两段原始内容
    assert!(normalized[1].content.contains("assistant part 1"));
    assert!(normalized[1].content.contains("assistant part 2"));
}

/// 测试失败标记在孤立用户消息后的保留
///
/// 验证在归一化过程中，任务失败标记能够被正确保留：
/// - 包含 "[Task failed — not continuing this request]" 的助手消息应被保留
/// - 失败标记后的用户消息也应被保留
/// - 不应因为归一化而丢失失败状态信息
///
/// # 测试场景
/// 输入：[user, assistant(failed), user] 三条消息
/// 预期输出：保持原样，失败标记被保留
#[test]
fn normalize_preserves_failure_marker_after_orphan_user_turn() {
    // 构造包含失败标记的对话序列
    let turns = vec![
        ChatMessage::user("download something from GitHub"),
        ChatMessage::assistant("[Task failed — not continuing this request]"),
        ChatMessage::user("what is WAL?"),
    ];

    // 执行归一化处理
    let normalized = normalize_cached_channel_turns(turns);

    // 验证结果：所有消息都应被保留
    assert_eq!(normalized.len(), 3);
    assert_eq!(normalized[0].role, "user");
    assert_eq!(normalized[1].role, "assistant");
    // 验证失败标记被保留
    assert!(normalized[1].content.contains("Task failed"));
    assert_eq!(normalized[2].role, "user");
    assert_eq!(normalized[2].content, "what is WAL?");
}

/// 测试超时标记在孤立用户消息后的保留
///
/// 验证在归一化过程中，任务超时标记能够被正确保留：
/// - 包含 "[Task timed out — not continuing this request]" 的助手消息应被保留
/// - 超时标记后的用户消息也应被保留
/// - 不应因为归一化而丢失超时状态信息
///
/// # 测试场景
/// 输入：[user, assistant(timeout), user] 三条消息
/// 预期输出：保持原样，超时标记被保留
#[test]
fn normalize_preserves_timeout_marker_after_orphan_user_turn() {
    // 构造包含超时标记的对话序列
    let turns = vec![
        ChatMessage::user("run a long task"),
        ChatMessage::assistant("[Task timed out — not continuing this request]"),
        ChatMessage::user("next question"),
    ];

    // 执行归一化处理
    let normalized = normalize_cached_channel_turns(turns);

    // 验证结果：所有消息都应被保留
    assert_eq!(normalized.len(), 3);
    assert_eq!(normalized[1].role, "assistant");
    // 验证超时标记被保留
    assert!(normalized[1].content.contains("Task timed out"));
    assert_eq!(normalized[2].content, "next question");
}

/// 测试简单的连续用户消息合并
///
/// 验证最基本的两条连续用户消息合并场景：
/// - 两条用户消息应合并为一条
/// - 合并格式应为 "第一条消息\n\n第二条消息"
/// - 合并后的角色仍为 "user"
///
/// # 测试场景
/// 输入：["hello", "world"] 两条用户消息
/// 预期输出：["hello\n\nworld"] 一条用户消息
#[test]
fn normalize_merges_consecutive_user_turns_simple() {
    // 构造两条简单的连续用户消息
    let turns = vec![ChatMessage::user("hello"), ChatMessage::user("world")];

    // 执行归一化处理
    let result = normalize_cached_channel_turns(turns);

    // 验证结果：应该合并为 1 条消息，使用双换行符分隔
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].role, "user");
    assert_eq!(result[0].content, "hello\n\nworld");
}

/// 测试严格交替顺序的保留
///
/// 验证在已经满足用户-助手交替顺序的情况下，归一化不会做不必要的修改：
/// - 完美交替的消息序列应保持原样
/// - 不应合并任何消息
/// - 消息顺序和内容都应完全保持不变
///
/// # 测试场景
/// 输入：[user, assistant, user] 完美交替的三条消息
/// 预期输出：保持原样，不发生任何合并
#[test]
fn normalize_preserves_strict_alternation() {
    // 构造完美交替的对话序列
    let turns =
        vec![ChatMessage::user("hello"), ChatMessage::assistant("hi"), ChatMessage::user("bye")];

    // 执行归一化处理
    let result = normalize_cached_channel_turns(turns);

    // 验证结果：应保持原样，不发生合并
    assert_eq!(result.len(), 3);
    assert_eq!(result[0].content, "hello");
    assert_eq!(result[1].content, "hi");
    assert_eq!(result[2].content, "bye");
}

/// 测试多条连续用户消息的合并
///
/// 验证超过两条的连续用户消息也能被正确合并：
/// - 三条或更多连续的同角色消息应合并为一条
/// - 合并格式应为所有消息用双换行符 "\n\n" 连接
/// - 合并顺序应保持原始消息的先后顺序
///
/// # 测试场景
/// 输入：["a", "b", "c"] 三条连续用户消息
/// 预期输出：["a\n\nb\n\nc"] 一条合并后的用户消息
#[test]
fn normalize_merges_multiple_consecutive_user_turns() {
    // 构造三条连续的用户消息
    let turns = vec![ChatMessage::user("a"), ChatMessage::user("b"), ChatMessage::user("c")];

    // 执行归一化处理
    let result = normalize_cached_channel_turns(turns);

    // 验证结果：应该合并为 1 条消息，三条内容用双换行符连接
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].role, "user");
    assert_eq!(result[0].content, "a\n\nb\n\nc");
}

/// 测试空输入的边界情况
///
/// 验证空消息列表的处理：
/// - 空列表应返回空列表
/// - 不应产生任何错误或异常
/// - 这是边界条件测试，确保函数对空输入具有健壮性
///
/// # 测试场景
/// 输入：空的消息列表 vec![]
/// 预期输出：空的归一化结果列表
#[test]
fn normalize_empty_input() {
    // 使用空列表作为输入
    let result = normalize_cached_channel_turns(vec![]);

    // 验证结果：应该返回空列表
    assert!(result.is_empty());
}
