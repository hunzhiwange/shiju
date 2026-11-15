//! 通道历史存储操作；调用方提供原共享存储。
use model_types::ChatMessage;
use std::collections::HashMap;
use std::sync::Mutex;

pub fn clear_sender_history(histories: &Mutex<HashMap<String, Vec<ChatMessage>>>, sender_key: &str) {
    histories.lock().unwrap_or_else(|e| e.into_inner()).remove(sender_key);
}

pub fn append_sender_turn(histories: &Mutex<HashMap<String, Vec<ChatMessage>>>, sender_key: &str, turn: ChatMessage) {
    let mut histories = histories.lock().unwrap_or_else(|e| e.into_inner());
    histories.entry(sender_key.to_string()).or_default().push(turn);
}

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
