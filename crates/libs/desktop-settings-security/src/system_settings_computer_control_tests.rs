use super::*;
use crate::computer_control_state::PairingCredential;
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
        chrome: report(true, ControlReadiness::Disconnected),
        excel: report(true, ControlReadiness::NotInstalled),
        word: report(false, ControlReadiness::Disabled),
        locked_use: LockedUseStatus { status: report(false, ControlReadiness::Unsupported), probe: LockedUseProbe::Unsupported },
    }, ControlApprovals { permission_version: version, approvals: vec![ControlGrantScope::Application("com.example.editor".into()), ControlGrantScope::ChromeOrigin("https://example.org".into())] })
}
fn loaded_state() -> ComputerControlSettingsState {
    let mut state = ComputerControlSettingsState { loading: true, generation: 1, ..Default::default() };
    assert!(state.apply_loaded(1, Ok(snapshot())));
    state
}
#[test]
fn cc_control_19_five_reports_remain_independent() {
    let state = loaded_state();
    assert_eq!(FEATURES.len(), 5);
    assert_eq!(state.feature(ControlSetting::NativeApps).unwrap().readiness, ControlReadiness::Unauthorized);
    assert_eq!(state.feature(ControlSetting::Chrome).unwrap().readiness, ControlReadiness::Disconnected);
    assert_eq!(state.feature(ControlSetting::Excel).unwrap().readiness, ControlReadiness::NotInstalled);
    assert!(!state.feature(ControlSetting::Word).unwrap().enabled);
    assert_eq!(state.feature(ControlSetting::LockedUse).unwrap().readiness, ControlReadiness::Unsupported);
}
#[test]
fn cc_control_19_enabled_never_implies_ready() {
    let state = loaded_state();
    for feature in [ControlSetting::NativeApps, ControlSetting::Chrome, ControlSetting::Excel] {
        let report = state.feature(feature).unwrap();
        assert!(report.enabled);
        assert_ne!(readiness_label(report.readiness), readiness_label(ControlReadiness::Ready));
    }
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
    assert_eq!(state.approvals, vec![ControlGrantScope::ChromeOrigin("https://example.org".into())]);
}
#[test]
fn cc_control_19_credential_debug_is_redacted() {
    let credential = PairingCredential("one-use-test-credential".into());
    assert!(!format!("{credential:?}").contains(&credential.0));
}
