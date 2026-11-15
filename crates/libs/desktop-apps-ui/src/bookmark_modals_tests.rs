#[test]
fn can_add_web_bookmark_requires_non_blank_title() {
    assert!(!super::can_add_web_bookmark("", "https://example.com"));
    assert!(!super::can_add_web_bookmark(" \n\t", "https://example.com"));
}

#[test]
fn can_add_web_bookmark_requires_non_blank_url() {
    assert!(!super::can_add_web_bookmark("Example", ""));
    assert!(!super::can_add_web_bookmark("Example", " \n\t"));
}

#[test]
fn can_add_web_bookmark_accepts_trimmed_non_empty_fields() {
    assert!(super::can_add_web_bookmark("  Example  ", "  https://example.com  "));
}
