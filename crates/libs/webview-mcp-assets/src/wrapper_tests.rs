use super::render_wrapper;

#[test]
fn mcp_app_form_sandbox_limits_capabilities() {
    if cfg!(any(target_os = "windows", coverage)) {
        assert!(!render_wrapper(desktop_i18n::I18n::new(config_ui::DesktopLocale::SimplifiedChinese), "host", "app", false).contains("allow-forms"));
        return;
    }
    let html = render_wrapper(desktop_i18n::I18n::new(config_ui::DesktopLocale::SimplifiedChinese), "host", "app", true);
    assert!(html.contains("sandbox=\"allow-scripts allow-forms\""));
    assert!(!html.contains("allow-popups"));
    assert!(!html.contains("allow-top-navigation"));
    assert!(!html.contains("allow-same-origin"));
}
