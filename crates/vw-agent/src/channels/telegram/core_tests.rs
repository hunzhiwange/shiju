use super::TelegramChannel;

#[test]
fn api_url_uses_configured_base_and_token() {
    let channel = TelegramChannel::new("123:ABC".to_string(), vec![], false)
        .with_api_base("https://tapi.example.test".to_string());

    assert_eq!(channel.api_url("sendMessage"), "https://tapi.example.test/bot123:ABC/sendMessage");
}

#[test]
fn sanitize_telegram_error_redacts_bot_token() {
    let sanitized = TelegramChannel::sanitize_telegram_error(
        "failed https://api.telegram.org/bot123:SECRET/getMe",
    );

    assert!(!sanitized.contains("SECRET"));
    assert!(sanitized.contains("[redacted]"));
}

#[test]
fn sanitize_telegram_error_preserves_non_sensitive_text() {
    let sanitized = TelegramChannel::sanitize_telegram_error("Telegram API returned 429");

    assert_eq!(sanitized, "Telegram API returned 429");
}

#[test]
fn http_client_builds_request() {
    let channel = TelegramChannel::new("token".to_string(), vec![], false);
    let request =
        channel.http_client().get("https://api.telegram.org").build().expect("request builds");

    assert_eq!(request.url().as_str(), "https://api.telegram.org/");
}

#[test]
fn api_url_preserves_method_query_string() {
    let channel = TelegramChannel::new("123:ABC".to_string(), vec![], false)
        .with_api_base("https://tapi.example.test/".to_string());

    assert_eq!(
        channel.api_url("getFile?file_id=abc"),
        "https://tapi.example.test/bot123:ABC/getFile?file_id=abc"
    );
}

#[test]
fn is_any_user_allowed_supports_wildcard_and_multiple_identities() {
    let wildcard = TelegramChannel::new("token".to_string(), vec!["*".to_string()], false);
    assert!(wildcard.is_any_user_allowed(["anyone"].into_iter()));

    let channel = TelegramChannel::new(
        "token".to_string(),
        vec!["alice".to_string(), "42".to_string()],
        false,
    );
    assert!(channel.is_any_user_allowed(["unknown", "42"].into_iter()));
    assert!(channel.is_any_user_allowed(["alice"].into_iter()));
    assert!(!channel.is_any_user_allowed(["bob", "7"].into_iter()));
}

#[test]
fn is_any_user_allowed_recovers_from_poisoned_lock() {
    let channel = std::sync::Arc::new(TelegramChannel::new(
        "token".to_string(),
        vec!["alice".to_string()],
        false,
    ));
    let cloned = channel.clone();
    let _ = std::thread::spawn(move || {
        let _guard = cloned.allowed_users.write().expect("write lock");
        panic!("poison allowed_users lock");
    })
    .join();

    assert!(channel.is_any_user_allowed(["alice"].into_iter()));
}

#[test]
fn sanitize_telegram_error_redacts_multiple_bot_token_shapes() {
    let sanitized = TelegramChannel::sanitize_telegram_error(
        "bot123:SECRET/sendMessage and botabcDEF/getMe should disappear",
    );

    assert!(!sanitized.contains("SECRET"));
    assert!(!sanitized.contains("abcDEF"));
    assert_eq!(sanitized.matches("bot[redacted]").count(), 2);
}

#[tokio::test]
async fn handle_unauthorized_message_is_non_failing() {
    let channel = TelegramChannel::new("token".to_string(), vec![], false);
    channel.handle_unauthorized_message(&serde_json::json!({"message": {"text": "nope"}})).await;
}
// 本文件验证 Telegram API 地址、许可判断、客户端构建与错误脱敏。
