use super::assistant_error::{assistant_api_error_view, extract_url_from_error_message};
use crate::app::App;

#[test]
fn extract_url_from_error_message_trims_common_wrappers() {
    assert_eq!(
        extract_url_from_error_message("request failed (https://api.example.test/v1)"),
        Some("https://api.example.test/v1".to_string())
    );
    assert_eq!(
        extract_url_from_error_message("\"http://localhost:11434/api\" timed out"),
        Some("http://localhost:11434/api".to_string())
    );
}

#[test]
fn extract_url_from_error_message_returns_none_without_url() {
    assert_eq!(extract_url_from_error_message("network unavailable"), None);
}

#[test]
fn extract_url_from_error_message_picks_earliest_url() {
    assert_eq!(
        extract_url_from_error_message("see http://first.test or https://second.test"),
        Some("http://first.test".to_string())
    );
}

#[test]
fn extract_url_from_error_message_stops_at_quotes_and_whitespace() {
    assert_eq!(
        extract_url_from_error_message("failed at https://api.example.test/v1\" with 500"),
        Some("https://api.example.test/v1".to_string())
    );
    assert_eq!(
        extract_url_from_error_message("failed at https://api.example.test/v1 next"),
        Some("https://api.example.test/v1".to_string())
    );
}

#[test]
fn extract_url_from_error_message_ends_at_message_end() {
    // URL at end of message with no delimiter triggers unwrap_or(tail.len())
    assert_eq!(
        extract_url_from_error_message("request failed https://api.example.test/v1"),
        Some("https://api.example.test/v1".to_string())
    );
    assert_eq!(
        extract_url_from_error_message("https://api.example.test/v1/path"),
        Some("https://api.example.test/v1/path".to_string())
    );
}

#[test]
fn extract_url_from_error_message_trims_trailing_paren() {
    // '(' is not a delimiter in find_map but is trimmed by trim_matches
    assert_eq!(
        extract_url_from_error_message("failed at https://api.example.test/v1("),
        Some("https://api.example.test/v1".to_string())
    );
}

#[test]
fn extract_url_from_error_message_returns_none_for_empty() {
    assert_eq!(extract_url_from_error_message(""), None);
}

#[test]
fn assistant_api_error_view_rejects_invalid_or_unrelated_json() {
    let app = App::new().0;

    assert!(assistant_api_error_view(&app, 0, None, "not json").is_none());
    assert!(
        assistant_api_error_view(&app, 0, None, r#"{"name":"OtherError","message":"bad"}"#,)
            .is_none()
    );
    assert!(assistant_api_error_view(&app, 0, None, r#"{"name":"APIError"}"#).is_none());
    assert!(
        assistant_api_error_view(&app, 0, None, r#"{"name":"APIError","message":"   "}"#,)
            .is_none()
    );
}

#[test]
fn assistant_api_error_view_accepts_retryable_error_with_url() {
    let app = App::new().0;
    let content = r#"{"name":"APIError","message":"request to https://api.example.test/v1 failed","is_retryable":true}"#;

    let view = assistant_api_error_view(&app, 0, None, content);

    assert!(view.is_some());
}

#[test]
fn assistant_api_error_view_accepts_non_retryable_special_text_error() {
    let app = App::new().0;
    let content = r#"{"name":"APIError","message":"invalid API key","is_retryable":false}"#;

    let view = assistant_api_error_view(&app, 3, Some(2), content);

    assert!(view.is_some());
}

#[test]
fn assistant_api_error_view_accepts_error_without_retryability_hint() {
    let app = App::new().0;
    let content = r#"{"name":"APIError","message":"network unavailable"}"#;

    let view = assistant_api_error_view(&app, 0, None, content);

    assert!(view.is_some());
}
