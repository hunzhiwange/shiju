//! 通道历史存储操作；调用方提供原共享存储。
use model_types::ChatMessage;
use std::collections::HashMap;
use std::sync::Mutex;

/// 清除发送者的历史记录
///
/// 从通道运行时上下文中完全移除指定发送者的所有对话历史。
/// 通常用于用户主动清除历史或重置会话状态。
///
/// # 参数
///
/// - `histories`: 通道历史共享存储
/// - `sender_key`: 发送者会话键，通常通过 `scoped_history_key` 生成
///
/// # 线程安全
///
/// 该函数会锁定 `conversation_histories` 互斥锁进行操作。
/// 如果锁被污染（poisoned），会恢复并继续操作，确保不会因 panic 而中断。
///
/// # 示例
///
/// ```ignore
/// let sender_key = scoped_history_key(&histories, &msg);
/// clear_sender_history(&histories, &sender_key);
/// ```
pub fn clear_sender_history(histories: &Mutex<HashMap<String, Vec<ChatMessage>>>, sender_key: &str) {
    histories.lock().unwrap_or_else(|e| e.into_inner()).remove(sender_key);
}

/// Append raw history without changing earlier message boundaries or contents.
/// Model context is bounded by the session processor's checkpoints.
pub fn append_sender_turn(histories: &Mutex<HashMap<String, Vec<ChatMessage>>>, sender_key: &str, turn: ChatMessage) {
    let mut histories = histories.lock().unwrap_or_else(|e| e.into_inner());
    histories.entry(sender_key.to_string()).or_default().push(turn);
}

/// 回滚孤立的用户轮次
///
/// 当用户消息未能得到助手回复时（如请求被取消或失败），
/// 该函数用于从历史记录中移除该孤立的用户消息，保持历史的一致性。
///
/// # 参数
///
/// - `histories`: 通道历史共享存储
/// - `sender_key`: 发送者会话键
/// - `expected_content`: 预期的用户消息内容，用于验证要移除的消息
///
/// # 返回值
///
/// 返回 `bool` 表示是否成功执行了回滚操作：
/// - `true`: 最后一条是匹配的用户消息，已成功移除
/// - `false`: 历史不存在，或最后一条消息不匹配
///
/// # 一致性保证
///
/// 通过 `expected_content` 参数确保只移除预期的消息，防止误删除。
/// 如果移除后历史为空，会清理整个发送者的历史记录条目。
///
/// # 使用场景
///
/// - 用户取消正在处理的请求
/// - 助手生成失败需要恢复状态
/// - 测试或调试时需要撤销最后的用户输入
///
/// # 示例
///
/// ```ignore
/// let user_input = "What is the weather?";
/// // 用户取消请求后回滚
/// if rollback_orphan_user_turn(&histories, &sender_key, user_input) {
///     println!("Successfully rolled back orphan user turn");
/// }
/// ```
pub fn rollback_orphan_user_turn(
    histories: &Mutex<HashMap<String, Vec<ChatMessage>>>,
    sender_key: &str,
    expected_content: &str,
) -> bool {
    let mut histories = histories.lock().unwrap_or_else(|e| e.into_inner());
    let Some(turns) = histories.get_mut(sender_key) else {
        return false;
    };

    // 验证最后一条是否为匹配内容的用户消息
    let should_pop =
        turns.last().is_some_and(|turn| turn.role == "user" && turn.content == expected_content);
    if !should_pop {
        return false;
    }

    // 移除该轮次
    turns.pop();
    // 如果历史为空，移除整个发送者条目以释放内存
    if turns.is_empty() {
        histories.remove(sender_key);
    }
    true
}

use crate::scope::ChannelChatScope;

pub fn conversation_memory_key(msg: &channel_types::ChannelMessage, tenant: vw_api_transport::id::TenantScope) -> String {
    serde_json::to_string(&(ChannelChatScope::from_message(msg, tenant), &msg.id))
        .expect("channel memory identity contains only serializable fields")
}

pub fn conversation_history_key(msg: &channel_types::ChannelMessage, tenant: vw_api_transport::id::TenantScope) -> String {
    serde_json::to_string(&ChannelChatScope::from_message(msg, tenant))
        .expect("channel identity contains only serializable fields")
}
