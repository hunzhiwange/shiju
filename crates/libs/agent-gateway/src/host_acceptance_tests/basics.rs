//! 网关基础功能测试模块
//!
//! 本模块包含对网关核心安全配置和数据结构的单元测试，验证以下内容：
//! - Webhook 请求体大小限制的安全配置
//! - 请求超时时间的默认设置
//!
//! 这些测试确保网关的安全默认值和数据结构符合预期行为。

use super::*;
use std::time::Duration;

/// 验证第三方 webhook 请求体大小限制为 32MiB
///
/// 此测试确保 `MAX_BODY_SIZE` 常量被设置为 33,554,432 字节（32MiB），
/// 客户端模型请求不使用此限制。
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
