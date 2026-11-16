#![cfg(not(target_arch = "wasm32"))]
use crate::computer_control_state::*;
use crate::computer_control_update::Event;
use vw_api_computer_control::*;
fn report(enabled: bool, readiness: ControlReadiness) -> ControlFeatureStatus {
    ControlFeatureStatus {
        enabled,
        readiness,
        system_authorization: SystemAuthorization::NotChecked,
        connection: ControlConnection::Disconnected,
        backend: ControlBackend::Available,
        capabilities: vec![],
    }
}
fn snapshot() -> (ComputerControlStatus, ControlApprovals) {
    let version = PermissionVersion(7);
    (
        ComputerControlStatus {
            permission_version: version,
            native_apps: report(true, ControlReadiness::Unauthorized),
            chrome: report(true, ControlReadiness::Disconnected),
            excel: report(true, ControlReadiness::NotInstalled),
            word: report(false, ControlReadiness::Disabled),
            locked_use: LockedUseStatus {
                status: report(false, ControlReadiness::Unsupported),
                probe: LockedUseProbe::Unsupported,
            },
        },
        ControlApprovals {
            permission_version: version,
            approvals: vec![
                ControlGrantScope::Application("com.example.editor".into()),
                ControlGrantScope::ChromeOrigin("https://example.org".into()),
            ],
        },
    )
}
fn loaded_state() -> ComputerControlSettingsState {
    let mut state =
        ComputerControlSettingsState { loading: true, generation: 1, ..Default::default() };
    assert!(state.apply_loaded(1, Ok(snapshot())));
    state
}
#[test]
fn cc_control_19_toggle_waits_for_server_confirmation() {
    let mut state = loaded_state();
    let endpoint = gateway_endpoint::GatewayEndpoint::new("127.0.0.1", 9);
    state.identity = Some(ControlIdentity::from_endpoint(&endpoint));
    let _task = crate::computer_control_update::update(
        &mut state,
        &endpoint,
        None,
        None,
        Event::Toggle(ControlSetting::Word, true),
    );
    assert!(state.saving);
    assert!(!state.feature(ControlSetting::Word).unwrap().enabled);
    assert!(state.feature(ControlSetting::Chrome).unwrap().enabled);
}
