//! 网关基础功能测试模块
//!
//! 本模块包含对网关核心安全配置和数据结构的单元测试，验证以下内容：
//! - 请求体大小限制的安全配置
//! - 请求超时时间的默认设置
//! - Webhook 和 Agent 请求体的 JSON 解析和字段验证
//! - WhatsApp 验证查询参数的可选性
//!
//! 这些测试确保网关的安全默认值和数据结构符合预期行为。

use super::*;
use std::time::Duration;

/// 验证请求体大小限制为 32MiB
///
/// 此测试确保 `MAX_BODY_SIZE` 常量被设置为 33,554,432 字节（32MiB），
/// 以覆盖多模态图片与知识库 base64 上传，同时仍限制异常大请求。
#[test]
fn security_body_limit_is_32mib() {
    assert_eq!(MAX_BODY_SIZE, 32 * 1024 * 1024);
}

/// 验证请求超时时间为 30 秒
///
/// 此测试确保 `REQUEST_TIMEOUT_SECS` 常量被设置为 30 秒，
/// 以防止长时间挂起的请求占用资源。
#[test]
fn security_timeout_is_30_seconds() {
    assert_eq!(REQUEST_TIMEOUT_SECS, 30);
}

#[test]
fn workflow_chat_messages_timeout_is_one_hour() {
    assert_eq!(WORKFLOW_CHAT_MESSAGES_TIMEOUT_SECS, 3_600);
    assert_eq!(
        request_timeout_for_path("/v1/workflow/applications/chat-messages"),
        Duration::from_secs(WORKFLOW_CHAT_MESSAGES_TIMEOUT_SECS)
    );
    assert_eq!(
        request_timeout_for_path("/v1/workflow/applications/demo/chat-messages"),
        Duration::from_secs(WORKFLOW_CHAT_MESSAGES_TIMEOUT_SECS)
    );
    assert_eq!(
        request_timeout_for_path("/v1/chat/stream"),
        Duration::from_secs(WORKFLOW_CHAT_MESSAGES_TIMEOUT_SECS)
    );
    assert_eq!(
        request_timeout_for_path("/v1/task-plan-run/stream"),
        Duration::from_secs(WORKFLOW_CHAT_MESSAGES_TIMEOUT_SECS)
    );
    assert_eq!(
        request_timeout_for_path("/v1/workflow/applications"),
        Duration::from_secs(REQUEST_TIMEOUT_SECS)
    );
}

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

/// 验证 WhatsApp 验证查询的所有字段均为可选
///
/// 此测试确保 `WhatsAppVerifyQuery` 结构体的所有字段
/// （mode、verify_token、challenge）都可以为 None，
/// 以支持 WhatsApp webhook 验证流程的灵活性。
#[test]
fn whatsapp_query_fields_are_optional() {
    let q = WhatsAppVerifyQuery { mode: None, verify_token: None, challenge: None };
    assert!(q.mode.is_none());
}
