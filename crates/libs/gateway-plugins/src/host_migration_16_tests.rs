#[test]
fn plugin_text_helpers_trim_values_and_fall_back_for_empty_text() {
    assert_eq!(super::non_empty_string(Some("  Plugin Name  ")), Some("Plugin Name".to_string()));
    assert_eq!(super::non_empty_string(Some(" \t ")), None);
    assert_eq!(super::non_empty_string(None), None);
}
