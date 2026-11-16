//! 请求体必需字段的反序列化测试。

use super::*;

/// 验证 Webhook 请求体必须包含 message 字段
///
/// 测试场景：
/// - 有效的 JSON（包含 message 字段）应成功解析
/// - 缺少 message 字段的 JSON 应解析失败
#[test]
fn webhook_body_requires_message_field() {
    // 测试有效的 Webhook 请求体：包含必需的 message 字段
    let valid = r#"{"message": "hello"}"#;
    let parsed: Result<WebhookBody, _> = serde_json::from_str(valid);
    assert!(parsed.is_ok());
    assert_eq!(parsed.unwrap().message, "hello");

    // 测试无效的 Webhook 请求体：缺少必需的 message 字段
    let missing = r#"{"other": "field"}"#;
    let parsed: Result<WebhookBody, _> = serde_json::from_str(missing);
    assert!(parsed.is_err());
}

/// 验证 Agent 请求体必须包含 message 字段
///
/// 测试场景：
/// - 有效的 JSON（包含 message 字段）应成功解析
/// - 缺少 message 字段的 JSON 应解析失败
#[test]
fn agent_body_requires_message_field() {
    // 测试有效的 Agent 请求体：包含必需的 message 字段
    let valid = r#"{"message": "hello"}"#;
    let parsed: Result<AgentBody, _> = serde_json::from_str(valid);
    assert!(parsed.is_ok());
    assert_eq!(parsed.unwrap().message, "hello");

    // 测试无效的 Agent 请求体：缺少必需的 message 字段
    let missing = r#"{"other": "field"}"#;
    let parsed: Result<AgentBody, _> = serde_json::from_str(missing);
    assert!(parsed.is_err());
}
