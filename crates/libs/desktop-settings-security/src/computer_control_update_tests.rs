use super::safe_error;
use gateway_computer_control::ComputerControlApiError;

#[test]
fn unauthorized_error_explains_required_gateway_credentials() {
    let message = safe_error(ComputerControlApiError::Http(401));
    assert!(message.contains("当前网关已启用鉴权"));
    assert!(message.contains("检查网关连接设置中的 skey"));
}

#[test]
fn other_errors_preserve_status_without_authentication_guidance() {
    let message = safe_error(ComputerControlApiError::Http(503));
    assert!(message.contains("HTTP 503"));
    assert!(!message.contains("SKey"));
}

#[test]
fn cc_runtime_ui06_stale_permission_response_is_discarded() {
    use super::*;
    let endpoint = GatewayEndpoint::new("127.0.0.1", 8765);
    let identity = ControlIdentity::from_endpoint(&endpoint);
    let mut state = ComputerControlSettingsState::default();
    state.identity = Some(identity.clone());
    state.generation = 4;
    state.requesting_permission = true;
    let (_, notice) = update(&mut state, &endpoint, None, None, Event::NativePermissionRequested {
        identity, generation: 3, result: Err("old response".into()),
    });
    assert!(notice.is_none());
    assert!(state.requesting_permission);
}
#[test]
fn cc_runtime_ui06_not_installed_report_does_not_grant_permissions() {
    let report = vw_api_computer_control::NativeServiceReport {
        state: vw_api_computer_control::NativeServiceState::NotInstalled, permissions: None,
    };
    assert!(report.permissions.is_none());
    let mut state = crate::computer_control_state::ComputerControlSettingsState::default();
    state.requesting_permission = true;
    assert!(state.busy());
}

#[test]
fn cc_runtime_ui06_changed_identity_discards_report() {
    use super::*;
    let old = GatewayEndpoint::new("127.0.0.1", 8765);
    let current = GatewayEndpoint::new("remote.example", 8765);
    let mut state = ComputerControlSettingsState::default();
    let (_, notice) = update(&mut state, &current, None, None, Event::NativeServiceLoaded {
        identity: ControlIdentity::from_endpoint(&old), generation: 0,
        result: Ok(NativeServiceReport { state: NativeServiceState::NotInstalled, permissions: None }),
    });
    assert!(notice.is_none());
    assert!(state.native_service.is_none());
}
