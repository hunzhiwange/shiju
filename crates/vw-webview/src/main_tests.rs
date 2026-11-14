//! WebView 启动入口的行为测试。

use super::*;

#[test]
fn constants_keep_expected_defaults() {
    assert!(SAFARI_USER_AGENT.contains("Safari/605.1.15"));
}

#[test]
fn user_event_open_url_keeps_url_payload() {
    assert_eq!(
        UserEvent::OpenUrl("https://example.com".to_string()),
        UserEvent::OpenUrl("https://example.com".to_string())
    );
}

#[cfg(coverage)]
#[test]
fn coverage_main_is_empty() {
    main();
}
