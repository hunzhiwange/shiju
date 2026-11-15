#[test]
fn cc_route_034_native_chat_expected_behavior() {
    for path in [
        "/chat/completions",
        "/v1/chat/completions",
        "/v1/v1/chat/completions",
        "/codex/v1/chat/completions",
    ] {
        assert!(super::native_chat::is_path(path), "{path}");
    }
    for path in [
        "/v2/chat/completions",
        "/other/v1/chat/completions",
        "/v1/chat/completions/",
        "/v1/responses",
    ] {
        assert!(!super::native_chat::is_path(path), "{path}");
    }
}
