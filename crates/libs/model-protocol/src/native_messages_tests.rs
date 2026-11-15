#[test]
fn cc_route_035_native_messages_expected_behavior() {
    let body = serde_json::json!({"model":"fixture","messages":[{"role":"assistant","content":[{"type":"thinking","thinking":"reason","signature":"fixture"}]}],"vendor_option":true});
    // The existing session converter remains strict for native-only blocks.
    assert!(super::messages::to_chat(&body).is_err());
}

#[test]
fn cc_route_035_native_messages_paths_exclude_desktop_and_unknown_aliases() {
    for path in ["/v1/messages", "/claude/v1/messages"] {
        assert!(super::native_messages::is_path(path));
    }
    for path in ["/messages", "/v1/v1/messages", "/claude-desktop/v1/messages", "/v1/messages/"] {
        assert!(!super::native_messages::is_path(path));
    }
}
