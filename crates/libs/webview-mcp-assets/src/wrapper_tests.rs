use super::render_wrapper;

#[test]
fn mcp_app_form_sandbox_limits_capabilities() {
    if cfg!(any(target_os = "windows", coverage)) {
        assert!(!render_wrapper("host", "app", false).contains("allow-forms"));
        return;
    }
    let html = render_wrapper("host", "app", true);
    assert!(html.contains("sandbox=\"allow-scripts allow-forms\""));
    assert!(!html.contains("allow-popups"));
    assert!(!html.contains("allow-top-navigation"));
    assert!(!html.contains("allow-same-origin"));
}
