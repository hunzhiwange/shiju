#[test]
fn cookie_config_example_documents_required_cookie_fields() {
    let example = crate::bookmark_modals::WEB_BOOKMARK_COOKIE_CONFIG_EXAMPLE;

    assert!(example.starts_with('['));
    assert!(example.ends_with(']'));
    for field in ["\"name\"", "\"domain\"", "\"days\"", "\"url_filter\""] {
        assert!(example.contains(field));
    }
}
