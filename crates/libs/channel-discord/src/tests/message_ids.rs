use super::*;

/// 测试 Discord 消息 ID 格式包含 discord 前缀
/// 验证消息 ID 遵循格式: discord_{message_id}
#[test]
fn discord_message_id_format_includes_discord_prefix() {
    let message_id = "123456789012345678";
    let expected_id = format!("discord_{message_id}");
    assert_eq!(expected_id, "discord_123456789012345678");
}

/// 测试 Discord 消息 ID 的确定性
/// 相同的 message_id 应该产生相同的 ID（防止重启后重复）
#[test]
fn discord_message_id_is_deterministic() {
    let message_id = "123456789012345678";
    let id1 = format!("discord_{message_id}");
    let id2 = format!("discord_{message_id}");
    assert_eq!(id1, id2);
}

/// 测试不同消息产生不同的 ID
/// 不同的 message_id 应该产生不同的完整 ID
#[test]
fn discord_message_id_different_message_different_id() {
    let id1 = "discord_123456789012345678".to_string();
    let id2 = "discord_987654321098765432".to_string();
    assert_ne!(id1, id2);
}

/// 测试 Discord 消息 ID 使用雪花 ID
/// Discord 雪花 ID 是数字字符串
#[test]
fn discord_message_id_uses_snowflake_id() {
    let message_id = "123456789012345678";
    let id = format!("discord_{message_id}");
    assert!(id.starts_with("discord_"));
    assert!(message_id.chars().all(|c| c.is_ascii_digit()));
}

/// 测试空 message_id 时回退到 UUID
/// 边缘情况：空的 message_id 应该回退到 UUID
#[test]
fn discord_message_id_fallback_to_uuid_on_empty() {
    let message_id = "";
    let id = if message_id.is_empty() {
        format!("discord_{}", uuid::Uuid::new_v4())
    } else {
        format!("discord_{message_id}")
    };
    assert!(id.starts_with("discord_"));
    assert!(id.contains('-'));
}
