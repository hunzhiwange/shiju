use vw_desktop::app::state::{PluginInstallForm, PluginInstallPhase};
use vw_gateway_client::{DesktopPluginCapabilitiesDto, DesktopPluginPreviewDto};
fn preview(token: &str) -> DesktopPluginPreviewDto {
    DesktopPluginPreviewDto {
        token: token.into(),
        plugin_id: "fixture".into(),
        source: "local".into(),
        old_version: None,
        version: Some("1".into()),
        capabilities: DesktopPluginCapabilitiesDto {
            skills: 0,
            mcp_servers: 0,
            hooks: 0,
            interface: false,
        },
        audit_status: "passed".into(),
        dependency_status: "no_declared_dependencies".into(),
        dependency_checks: Vec::new(),
        origin: Default::default(),
        previous_capabilities: None,
    }
}
#[test]
fn codex_plugin_compat_t56_installs_only_after_explicit_action() {
    let mut form = PluginInstallForm::new(1, None);
    form.source = "local".into();
    assert!(form.confirm().is_none());
    assert!(form.begin_prepare().is_ok());
    assert!(form.receive_preview(1, Ok(preview("audited-token"))).is_none());
    assert!(matches!(form.phase, PluginInstallPhase::Review(_)));
    assert!(!form.busy());
    let confirmed = form.confirm().expect("only explicit confirmation yields an activation token");
    assert_eq!(confirmed.token, "audited-token");
    assert!(form.busy());
    assert!(form.confirm().is_none());
}
#[test]
fn codex_plugin_compat_t56_rejects_invalid_input() {
    let mut form = PluginInstallForm::new(1, None);
    assert!(form.begin_prepare().unwrap_err().contains("来源"));
    form.phase = PluginInstallPhase::Review(preview(""));
    assert!(!form.can_confirm());
    assert!(form.confirm().is_none());
}
#[test]
fn codex_plugin_compat_t56_preserves_isolation() {
    let mut form = PluginInstallForm::new(2, None);
    form.source = "new".into();
    assert!(form.begin_prepare().is_ok());
    let stale = form.receive_preview(1, Ok(preview("old-token"))).unwrap();
    assert_eq!(stale.token, "old-token");
    assert!(form.preview().is_none());
    assert!(form.receive_preview(2, Ok(preview("current-token"))).is_none());
    assert_eq!(form.preview().unwrap().token, "current-token");
}
