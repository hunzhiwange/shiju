//! 验证默认 HTTP 请求头包含预期的中英文语言偏好。
//! 该契约防止调用方因默认值漂移而获得不同的内容协商结果。

use super::*;

#[test]
fn default_headers_include_chinese_and_english_language_preferences() {
    assert_eq!(
        default_headers().get(ACCEPT_LANGUAGE).and_then(|value| value.to_str().ok()),
        Some(ACCEPT_LANGUAGE_VALUE)
    );
}
