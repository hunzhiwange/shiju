use crate::history::*;
use model_types::ChatMessage;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

#[test]
fn normalize_cached_turns_merges_unexpected_roles_and_skips_system() {
    let turns = vec![
        ChatMessage::system("ignored"),
        ChatMessage::user("one"),
        ChatMessage::user("two"),
        ChatMessage::assistant("answer"),
        ChatMessage::assistant("more"),
        ChatMessage::user("three"),
    ];

    let normalized = normalize_cached_channel_turns(turns);

    assert_eq!(normalized.len(), 3);
    assert_eq!(normalized[0].role, "user");
    assert_eq!(normalized[0].content, "one\n\ntwo");
    assert_eq!(normalized[1].role, "assistant");
    assert_eq!(normalized[1].content, "answer\n\nmore");
    assert_eq!(normalized[2].content, "three");
}

#[test]
fn normalize_cached_turns_drops_orphan_leading_roles_and_merges_without_separator_for_empty_last() {
    let turns = vec![
        ChatMessage::assistant("orphan"),
        ChatMessage::tool("ignored"),
        ChatMessage::user(""),
        ChatMessage::user("late"),
        ChatMessage::user(""),
        ChatMessage::assistant(""),
        ChatMessage::assistant("tail"),
    ];

    let normalized = normalize_cached_channel_turns(turns);

    assert_eq!(normalized.len(), 2);
    assert_eq!(normalized[0].role, "user");
    assert_eq!(normalized[0].content, "late");
    assert_eq!(normalized[1].role, "assistant");
    assert_eq!(normalized[1].content, "tail");
}

#[test]
fn append_sender_turn_preserves_complete_history() {
    let ctx = Arc::new(Mutex::new(HashMap::new()));

    for idx in 0..53 {
        append_sender_turn(&ctx, "sender", ChatMessage::user(format!("turn-{idx}")));
    }

    let histories = ctx.lock().unwrap();
    let turns = histories.get("sender").expect("history should exist");
    assert_eq!(turns.len(), 53);
    assert_eq!(turns.first().expect("first turn").content, "turn-0");
}

#[test]
fn clear_sender_history_removes_existing_key_and_ignores_missing_key() {
    let ctx = Arc::new(Mutex::new(HashMap::new()));
    append_sender_turn(&ctx, "sender", ChatMessage::user("hello"));

    clear_sender_history(&ctx, "missing");
    assert!(ctx.lock().unwrap().contains_key("sender"));

    clear_sender_history(&ctx, "sender");
    assert!(!ctx.lock().unwrap().contains_key("sender"));
}

#[test]
// 共享历史锁中毒时仍需降级恢复，不能让渠道永久停服。
fn history_operations_recover_from_poisoned_history_lock() {
    let ctx = Arc::new(Mutex::new(HashMap::new()));
    let histories = Arc::clone(&ctx);
    let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(move || {
        let _guard = histories.lock().unwrap();
        panic!("poison history lock");
    }));

    clear_sender_history(&ctx, "missing");
    append_sender_turn(&ctx, "sender", ChatMessage::user("hello"));
    assert!(rollback_orphan_user_turn(&ctx, "sender", "hello"));
    assert!(
        !ctx
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .contains_key("sender")
    );
}

#[test]
fn rollback_orphan_user_turn_only_removes_matching_last_user_turn() {
    let ctx = Arc::new(Mutex::new(HashMap::new()));

    assert!(!rollback_orphan_user_turn(&ctx, "sender", "hello"));

    append_sender_turn(&ctx, "sender", ChatMessage::user("hello"));
    append_sender_turn(&ctx, "sender", ChatMessage::assistant("reply"));
    assert!(!rollback_orphan_user_turn(&ctx, "sender", "hello"));

    append_sender_turn(&ctx, "sender", ChatMessage::user("latest"));
    assert!(!rollback_orphan_user_turn(&ctx, "sender", "other"));
    assert!(rollback_orphan_user_turn(&ctx, "sender", "latest"));

    let histories = ctx.lock().unwrap();
    assert_eq!(histories["sender"].last().expect("last turn").role, "assistant");
}

#[test]
fn rollback_orphan_user_turn_removes_empty_history_entry() {
    let ctx = Arc::new(Mutex::new(HashMap::new()));
    append_sender_turn(&ctx, "sender", ChatMessage::user("only"));

    assert!(rollback_orphan_user_turn(&ctx, "sender", "only"));
    assert!(!ctx.lock().unwrap().contains_key("sender"));
}

/// 规范化缓存的通道轮次
///
/// 该函数处理和规范化对话轮次序列，确保用户和助手消息正确交替。
/// 对于不符合交替模式的消息（如中断导致的连续用户消息），会进行合并处理。
///
/// # 参数
///
/// - `turns`: 原始的聊天消息向量，可能包含不符合交替模式的消息
///
/// # 返回值
///
/// 返回规范化后的聊天消息向量，其中：
/// - 用户和助手消息正确交替
/// - 不符合模式的消息已合并到相邻的同类消息中
///
/// # 处理逻辑
///
/// 1. 期望状态机：跟踪当前期望的消息角色（用户或助手）
/// 2. 正常情况：用户 -> 助手 -> 用户 -> 助手 的交替模式
/// 3. 异常处理：
///    - 连续的用户消息（助手被中断未持久化）：合并到前一个用户消息
///    - 连续的助手消息：合并到前一个助手消息
///
/// # 示例
///
/// ```ignore
/// let turns = vec![
///     ChatMessage { role: "user".into(), content: "Hello".into() },
///     ChatMessage { role: "user".into(), content: "World".into() }, // 异常：连续用户消息
/// ];
/// let normalized = normalize_cached_channel_turns(turns);
/// // 结果：一个包含合并内容 "Hello\n\nWorld" 的用户消息
/// ```
#[cfg(test)]
fn normalize_cached_channel_turns(turns: Vec<ChatMessage>) -> Vec<ChatMessage> {
    let mut normalized = Vec::with_capacity(turns.len());
    // 状态标志：true 表示期望用户消息，false 表示期望助手消息
    let mut expecting_user = true;

    for turn in turns {
        match (expecting_user, turn.role.as_str()) {
            // 正常情况：收到期望的用户消息
            (true, "user") => {
                normalized.push(turn);
                expecting_user = false; // 下次期望助手消息
            }
            // 正常情况：收到期望的助手消息
            (false, "assistant") => {
                normalized.push(turn);
                expecting_user = true; // 下次期望用户消息
            }
            // 中断的通道轮次可能产生连续的用户消息（助手尚未持久化）
            // 采用合并策略而非丢弃，以保留完整的用户输入上下文
            (false, "user") | (true, "assistant") => {
                if let Some(last_turn) = normalized.last_mut() {
                    if !turn.content.is_empty() {
                        if !last_turn.content.is_empty() {
                            // 用双换行符分隔合并的内容，保持可读性
                            last_turn.content.push_str("\n\n");
                        }
                        last_turn.content.push_str(&turn.content);
                    }
                }
            }
            // 忽略其他角色（如 system）的消息
            _ => {}
        }
    }

    normalized
}
