//! Deferred focused coverage for task 19; no backend execution is inferred from a toggle.
use super::*;
use crate::app::message::settings::computer_control::update;
use crate::app::App;
use crate::app::message::settings::computer_control::Event;
use vw_api_computer_control::*;

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
fn cc_control_19_save_failure_displays_error_and_requests_readback() {
    let mut app = App::new().0;
    let identity = ControlIdentity::from_endpoint(&crate::app::gateway_client_endpoint());
    app.computer_control_settings = loaded_state();
    app.computer_control_settings.identity = Some(identity.clone());
    app.computer_control_settings.saving = true;
    let _task = update(&mut app, Event::Completed { identity, generation: 1, result: Err("保存失败".into()) });
    assert!(!app.computer_control_settings.saving);
    assert!(app.computer_control_settings.loading);
    assert!(app.computer_control_settings.status.is_none());
    assert_eq!(app.active_toast.as_ref().unwrap().message, "保存失败");
}
#[test]
fn cc_control_19_navigation_has_independent_entry() {
    use crate::app::components::system_settings::{SystemTab, system_tab_matches_query};
    assert!(SystemTab::all().any(|tab| tab == SystemTab::ComputerControl));
    assert!(system_tab_matches_query(SystemTab::ComputerControl, "Excel"));
    assert!(system_tab_matches_query(SystemTab::ComputerControl, "电脑操控"));
}
#[test]
fn cc_control_19_toggle_waits_for_server_confirmation() {
    let mut app = App::new().0;
    app.computer_control_settings = loaded_state();
    app.computer_control_settings.identity = Some(ControlIdentity::from_endpoint(&crate::app::gateway_client_endpoint()));
    let _task = update(&mut app, Event::Toggle(ControlSetting::Word, true));
    assert!(app.computer_control_settings.saving);
    assert!(!app.computer_control_settings.feature(ControlSetting::Word).unwrap().enabled);
    assert!(app.computer_control_settings.feature(ControlSetting::Chrome).unwrap().enabled);
}
#[test]
fn cc_control_19_stop_failure_does_not_claim_success() {
    let mut app = App::new().0;
    let identity = ControlIdentity::from_endpoint(&crate::app::gateway_client_endpoint());
    app.computer_control_settings.identity = Some(identity.clone());
    app.computer_control_settings.stopping = true;
    let _task = update(&mut app, Event::Stopped { identity, result: Err("停止请求失败".into()) });
    assert!(!app.computer_control_settings.stopping);
    assert_eq!(app.active_toast.as_ref().unwrap().message, "停止请求失败");
}
