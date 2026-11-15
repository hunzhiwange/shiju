//! 验证设置页 URL 编码、布尔能力标签与上下文额度格式化。
//! 空格编码和整千缩写用例用于锁定面向用户的紧凑表示规则。

use super::utils::{bool_support_label, format_context_limit, url_encode};

#[test]
fn url_encode_preserves_unreserved_and_encodes_space() {
    assert_eq!(url_encode("abc-_.~"), "abc-_.~");
    assert_eq!(url_encode("a b"), "a%20b");
}

#[test]
fn bool_support_label_maps_boolean_state() {
    assert_eq!(bool_support_label(true), "支持");
    assert_eq!(bool_support_label(false), "不支持");
}

#[test]
fn format_context_limit_prefers_k_suffix_for_even_thousands() {
    assert_eq!(format_context_limit(2048), "2K");
    assert_eq!(format_context_limit(3000), "3K");
    assert_eq!(format_context_limit(1234), "1234");
}
