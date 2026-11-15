use super::assistant_error::extract_url_from_error_message;

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
