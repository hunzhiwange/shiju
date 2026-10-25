//! 内存键生成函数的单元测试模块
//!
//! 本模块测试各通道的内存键生成逻辑，确保每个通道生成的键：
//! - 具有唯一性（避免冲突）
//! - 包含必要标识信息（如发送者 ID、消息 ID）
//! - 符合预期格式

use super::*;

/// 测试 webhook 内存键的唯一性
///
/// 验证每次调用 `webhook_memory_key()` 都会生成不同的键，
/// 确保多个 webhook 消息不会因键冲突而互相覆盖。
#[test]
fn webhook_memory_key_is_unique() {
    // 生成两个内存键
    let key1 = webhook_memory_key();
    let key2 = webhook_memory_key();

    // 验证键格式：都应以 "webhook_msg_" 开头
    assert!(key1.starts_with("webhook_msg_"));
    assert!(key2.starts_with("webhook_msg_"));

    // 验证键的唯一性：两个键不应相同
    assert_ne!(key1, key2);
}

/// 测试 WhatsApp 内存键包含发送者和消息 ID
///
/// 验证 `whatsapp_memory_key()` 生成的键格式为：
/// `whatsapp_{sender}_{message_id}`
///
/// 这种格式确保：
/// - 可通过键追溯消息来源（发送者）
/// - 可通过键定位具体消息（消息 ID）
#[test]
fn whatsapp_memory_key_includes_sender_and_message_id() {
    // 构造测试用的 WhatsApp 消息
    let msg = crate::app::agent::channels::traits::ChannelMessage {
        instance_id: "default".to_string(),
        id: "wamid-123".into(),             // WhatsApp 消息 ID
        sender: "+1234567890".into(),       // 发送者手机号
        reply_target: "+1234567890".into(), // 回复目标（同发送者）
        content: "hello".into(),            // 消息内容
        channel: "whatsapp".into(),         // 通道标识
        timestamp: 1,                       // 时间戳
        thread_ts: None,                    // 线程时间戳（不适用）
    };

    // 生成内存键并验证格式
    let key = whatsapp_memory_key(&msg);
    assert_eq!(key, "whatsapp_+1234567890_wamid-123");
}

/// 测试 QQ 内存键包含发送者和消息 ID
///
/// 验证 `qq_memory_key()` 生成的键格式为：
/// `qq_{sender}_{message_id}`
///
/// 这种格式确保：
/// - 可通过键追溯消息来源（发送者 OpenID）
/// - 可通过键定位具体消息（消息 ID）
/// - 与 WhatsApp 键格式保持一致的设计模式
#[test]
fn qq_memory_key_includes_sender_and_message_id() {
    // 构造测试用的 QQ 消息
    let msg = crate::app::agent::channels::traits::ChannelMessage {
        instance_id: "default".to_string(),
        id: "msg-123".into(),                    // QQ 消息 ID
        sender: "user_openid".into(),            // 发送者 OpenID
        reply_target: "user:user_openid".into(), // 回复目标（QQ 特有格式）
        content: "hello".into(),                 // 消息内容
        channel: "qq".into(),                    // 通道标识
        timestamp: 1,                            // 时间戳
        thread_ts: Some("msg-123".into()),       // 线程时间戳（QQ 消息支持线程）
    };

    // 生成内存键并验证格式
    let key = qq_memory_key(&msg);
    assert_eq!(key, "qq_user_openid_msg-123");
}
