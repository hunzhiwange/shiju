use super::*;
fn report(enabled: bool, readiness: ControlReadiness) -> ControlFeatureStatus {
    ControlFeatureStatus {
        enabled, readiness, system_authorization: SystemAuthorization::NotChecked,
        connection: ControlConnection::Disconnected, backend: ControlBackend::Available,
        capabilities: vec![],
    }
}
fn snapshot() -> (ComputerControlStatus, ControlApprovals) {
    let version = PermissionVersion(7);
    (ComputerControlStatus {
        permission_version: version,
        native_apps: report(true, ControlReadiness::Unauthorized),
        locked_use: LockedUseStatus { status: report(false, ControlReadiness::Unsupported), probe: LockedUseProbe::Unsupported },
    }, ControlApprovals { permission_version: version, approvals: vec![ControlGrantScope::Application("com.example.editor".into()), ControlGrantScope::Application("com.example.viewer".into())] })
}
fn loaded_state() -> ComputerControlSettingsState {
    let mut state = ComputerControlSettingsState { loading: true, generation: 1, ..Default::default() };
    assert!(state.apply_loaded(1, Ok(snapshot())));
    state
}
#[test]
fn cc_control_19_native_and_locked_reports_remain_independent() {
    let state = loaded_state();
    assert_eq!(FEATURES.len(), 2);
    assert_eq!(state.feature(ControlSetting::NativeApps).unwrap().readiness, ControlReadiness::Unauthorized);
    assert_eq!(state.feature(ControlSetting::LockedUse).unwrap().readiness, ControlReadiness::Unsupported);
}
#[test]
fn cc_control_19_enabled_never_implies_ready() {
    let state = loaded_state();
    let report = state.feature(ControlSetting::NativeApps).unwrap();
    assert!(report.enabled);
    assert_ne!(readiness_label(report.readiness), readiness_label(ControlReadiness::Ready));
}
#[test]
fn cc_control_19_loading_failure_clears_authorization_snapshot() {
    let mut state = loaded_state();
    state.loading = true;
    assert!(state.apply_loaded(1, Err("服务不可用".into())));
    assert!(state.status.is_none());
    assert!(state.approvals.is_empty());
    assert_eq!(state.error.as_deref(), Some("服务不可用"));
    assert!(!state.loading);
}
#[test]
fn cc_control_19_stale_load_cannot_overwrite_current_state() {
    let mut state = loaded_state();
    state.loading = true;
    state.generation = 2;
    assert!(!state.apply_loaded(1, Err("旧请求失败".into())));
    assert!(state.status.is_some());
    assert!(state.loading);
}
#[test]
fn cc_control_19_mixed_permission_versions_reject_snapshot() {
    let mut state = ComputerControlSettingsState { loading: true, ..Default::default() };
    let (status, mut approvals) = snapshot();
    approvals.permission_version = PermissionVersion(8);
    assert!(state.apply_loaded(0, Ok((status, approvals))));
    assert!(state.status.is_none());
    assert!(state.error.is_some());
}
#[test]
fn cc_control_19_revoke_uses_server_readback_to_remove_identity() {
    let mut state = loaded_state();
    assert_eq!(state.approvals.len(), 2);
    let (mut status, mut approvals) = snapshot();
    status.permission_version = PermissionVersion(8);
    approvals.permission_version = PermissionVersion(8);
    approvals.approvals.remove(0);
    state.loading = true;
    assert!(state.apply_loaded(1, Ok((status, approvals))));
    assert_eq!(state.approvals, vec![ControlGrantScope::Application("com.example.viewer".into())]);
}
#[test]
fn cc_control_19_page_builds_for_loading_and_reported_states() {
    // Both paths must remain renderable while the server snapshot is unavailable or partial.
    let loading = ComputerControlSettingsState::default();
    let _ = page(desktop_i18n::I18n::new(config_ui::DesktopLocale::SimplifiedChinese), &loading);
    let loaded = loaded_state();
    let _ = page(desktop_i18n::I18n::new(config_ui::DesktopLocale::SimplifiedChinese), &loaded);
}
