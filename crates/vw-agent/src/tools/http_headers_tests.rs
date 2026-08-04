use super::*;

#[test]
fn default_headers_include_chinese_and_english_language_preferences() {
    assert_eq!(
        default_headers().get(ACCEPT_LANGUAGE).and_then(|value| value.to_str().ok()),
        Some(ACCEPT_LANGUAGE_VALUE)
    );
}
