use vw_desktop::app::components::system_settings_skills::plugin_dependency_summary;
fn checks(status: &str) -> Vec<vw_gateway_client::DesktopPluginDependencyDto> {
    vec![vw_gateway_client::DesktopPluginDependencyDto {
        component: "中文 MCP".into(),
        dependency: "node".into(),
        status: status.into(),
        detail: "安装宿主运行时后重试；不自动安装".into(),
    }]
}
#[test]
fn codex_plugin_compat_t55_shows_missing_dependency() {
    assert!(plugin_dependency_summary(&checks("missing_host")).contains("缺少"));
}
#[test]
fn codex_plugin_compat_t55_rejects_invalid_input() {
    assert!(plugin_dependency_summary(&checks("unknown-status")).contains("未知"));
}
#[test]
fn codex_plugin_compat_t55_preserves_isolation() {
    let a = checks("missing_host");
    let b = checks("present");
    assert_ne!(plugin_dependency_summary(&a), plugin_dependency_summary(&b));
    assert_eq!(a[0].status, "missing_host");
    assert_eq!(b[0].status, "present");
}
