//! Lark 群聊响应和 token 刷新测试。
//!
//! 本模块覆盖群聊 mention 判定、tenant access token 失效识别、刷新时间
//! 计算和发送响应错误处理。

use super::*;
use std::time::{Duration, Instant};

#[test]
fn lark_group_response_requires_matching_bot_mention_when_ids_available() {
    // 当机器人 open_id 已解析时，只响应明确 @ 当前机器人的群消息。
    let mentions = vec![serde_json::json!({
        "id": { "open_id": "ou_other" }
    })];
    assert!(!should_respond_in_group(true, "ou_user", &[], Some("ou_bot"), &mentions, &[]));

    let mentions = vec![serde_json::json!({
        "id": { "open_id": "ou_bot" }
    })];
    assert!(should_respond_in_group(true, "ou_user", &[], Some("ou_bot"), &mentions, &[]));
}

#[test]
fn lark_group_response_requires_resolved_open_id_when_mention_only_enabled() {
    // mention_only 开启但机器人 id 未知时保持拒绝，避免误响应所有群消息。
    let mentions = vec![serde_json::json!({
        "id": { "open_id": "ou_any" }
    })];
    assert!(!should_respond_in_group(true, "ou_user", &[], None, &mentions, &[]));
}

#[test]
fn lark_group_response_allows_post_mentions_for_bot_open_id() {
    assert!(should_respond_in_group(
        true,
        "ou_user",
        &[],
        Some("ou_bot"),
        &[],
        &[String::from("ou_bot")]
    ));
}

#[test]
fn lark_group_response_allows_sender_override_without_mention() {
    // 白名单发送者可作为人工配置的例外，用于运维或固定触发账号。
    assert!(should_respond_in_group(
        true,
        "ou_priority_user",
        &[String::from("ou_priority_user")],
        Some("ou_bot"),
        &[],
        &[]
    ));
}

#[test]
fn lark_should_refresh_token_on_http_401() {
    // HTTP 401 是传输层明确认证失败信号，应触发 token 刷新。
    let body = serde_json::json!({ "code": 0 });
    assert!(should_refresh_lark_tenant_token(reqwest::StatusCode::UNAUTHORIZED, &body));
}

#[test]
fn lark_should_refresh_token_on_body_code_99991663() {
    // Lark 可能用业务 code 表达 token 失效，即使 HTTP 状态为 200 也要刷新。
    let body = serde_json::json!({
        "code": LARK_INVALID_ACCESS_TOKEN_CODE,
        "msg": "Invalid access token for authorization."
    });
    assert!(should_refresh_lark_tenant_token(reqwest::StatusCode::OK, &body));
}

#[test]
fn lark_should_not_refresh_token_on_success_body() {
    let body = serde_json::json!({ "code": 0, "msg": "ok" });
    assert!(!should_refresh_lark_tenant_token(reqwest::StatusCode::OK, &body));
}

#[test]
fn lark_extract_token_ttl_seconds_supports_expire_and_expires_in() {
    let body_expire = serde_json::json!({ "expire": 7200 });
    let body_expires_in = serde_json::json!({ "expires_in": 3600 });
    let body_missing = serde_json::json!({});

    assert_eq!(extract_lark_token_ttl_seconds(&body_expire), 7200);
    assert_eq!(extract_lark_token_ttl_seconds(&body_expires_in), 3600);
    assert_eq!(extract_lark_token_ttl_seconds(&body_missing), LARK_DEFAULT_TOKEN_TTL.as_secs());
}

#[test]
fn lark_next_token_refresh_deadline_reserves_refresh_skew() {
    let now = Instant::now();
    // 长 TTL 提前刷新，短 TTL 至少保留极小有效窗口，避免算出过去时间。
    let regular = next_token_refresh_deadline(now, 7200);
    let short_ttl = next_token_refresh_deadline(now, 60);

    assert_eq!(regular.duration_since(now), Duration::from_secs(7080));
    assert_eq!(short_ttl.duration_since(now), Duration::from_secs(1));
}

#[test]
fn lark_ensure_send_success_rejects_non_zero_code() {
    // 发送接口 HTTP 成功不代表业务成功，必须同时检查响应体 code。
    let ok = serde_json::json!({ "code": 0 });
    let bad = serde_json::json!({ "code": 12345, "msg": "bad request" });

    assert!(ensure_lark_send_success(reqwest::StatusCode::OK, &ok, "test").is_ok());
    assert!(ensure_lark_send_success(reqwest::StatusCode::OK, &bad, "test").is_err());
}
