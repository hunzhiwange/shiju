use super::*;
struct Fixture { panel: PanelState, desktop_update: DesktopUpdate }
impl Fixture {
    fn new() -> Self {
        Self { panel: PanelState::default(), desktop_update: DesktopUpdate::from_config(&serde_json::json!({})) }
    }
}
impl std::ops::Deref for Fixture {
    type Target = PanelState;
    fn deref(&self) -> &Self::Target { &self.panel }
}
impl std::ops::DerefMut for Fixture {
    fn deref_mut(&mut self) -> &mut Self::Target { &mut self.panel }
}
fn update_fixture(fixture: &mut Fixture, message: PanelMessage) -> Task<PanelMessage> {
    update(&mut fixture.panel, &mut fixture.desktop_update, &mut Vec::new(), env!("CARGO_PKG_VERSION"), #[cfg(not(target_arch = "wasm32"))] config_ui::DesktopLocale::default(), message)
}
#[test]
fn closing_install_modal_hides_it_and_resets_update_actions() {
    use super::PanelMessage as ViewMessage;

    let mut app = Fixture::new();
    app.show_cli_install_modal = true;
    app.cli_install_modal_current_version = "old".into();
    app.cli_install_modal_server_version = "new".into();
    app.cli_install_modal_show_update_action = true;
    app.cli_install_modal_show_install_action = true;
    app.cli_install_modal_use_app_update_action = true;
    app.cli_install_modal_is_checking_update = true;

    let _ = update_fixture(&mut app, ViewMessage::CloseInstallCliModal);

    assert!(!app.show_cli_install_modal);
    assert_eq!(app.cli_install_modal_current_version, "old");
    assert!(app.cli_install_modal_use_app_update_action);
}

#[test]
fn cli_update_result_distinguishes_matching_new_and_failed_versions() {
    use super::PanelMessage as ViewMessage;

    let mut app = Fixture::new();
    app.cli_install_modal_current_version = "v1.2.3".into();
    app.cli_install_modal_is_checking_update = true;

    app.desktop_update.checking = true;
    let request = app.desktop_update.request;
    let source = app.desktop_update.source.unwrap();
    let _ = update_fixture(
        &mut app,
        ViewMessage::CheckCliToolUpdateFinished(request, source, Ok("v1.2.3".into())),
    );
    assert!(!app.cli_install_modal_is_checking_update);
    assert_eq!(app.cli_install_modal_server_version, "v1.2.3");
    assert!(app.cli_install_modal_message.contains("已是最新版本"));

    app.desktop_update.checking = true;
    let request = app.desktop_update.request;
    let source = app.desktop_update.source.unwrap();
    let _ = update_fixture(
        &mut app,
        ViewMessage::CheckCliToolUpdateFinished(request, source, Ok("v2.0.0".into())),
    );
    assert!(app.cli_install_modal_message.contains("发现新的 CLI 版本"));

    app.desktop_update.checking = true;
    let request = app.desktop_update.request;
    let source = app.desktop_update.source.unwrap();
    let _ = update_fixture(
        &mut app,
        ViewMessage::CheckCliToolUpdateFinished(request, source, Err("offline".into())),
    );
    assert_eq!(app.cli_install_modal_server_version, "检测失败");
    assert_eq!(app.cli_install_modal_message, "offline");
}

#[test]
fn app_update_source_switch_invalidates_pending_release() {
    use super::PanelMessage as ViewMessage;
    use desktop_release::UpdateSource;
    let mut app = Fixture::new();
    let _ = update_fixture(&mut app, ViewMessage::OpenAppUpdateModal);
    let _ = update_fixture(&mut app, ViewMessage::CheckAppUpdate);
    let request = app.desktop_update.request;
    let _ = update_fixture(&mut app, ViewMessage::AppUpdateSourceSelected(UpdateSource::Github));
    let _ = update_fixture(
        &mut app,
        ViewMessage::CheckAppUpdateFinished(
            request,
            UpdateSource::Shiju,
            Ok(desktop_release::DesktopRelease {
                source: UpdateSource::Shiju,
                version: "99.0.0".into(),
                assets: vec![],
            }),
        ),
    );
    assert_eq!(app.desktop_update.source, Some(UpdateSource::Github));
    assert!(app.desktop_update.release.is_none());
    assert!(!app.cli_install_modal_show_install_action);
}

#[test]
fn app_update_old_request_from_same_source_cannot_override_new_check() {
    use super::PanelMessage as ViewMessage;
    use desktop_release::UpdateSource;
    let mut app = Fixture::new();
    let _ = update_fixture(&mut app, ViewMessage::CheckAppUpdate);
    let old = app.desktop_update.request;
    let _ = update_fixture(&mut app, ViewMessage::AppUpdateSourceSelected(UpdateSource::Shiju));
    let _ = update_fixture(&mut app, ViewMessage::CheckAppUpdate);
    let _ = update_fixture(
        &mut app,
        ViewMessage::CheckAppUpdateFinished(old, UpdateSource::Shiju, Err("obsolete".into())),
    );
    assert!(app.desktop_update.checking);
    assert!(!app.cli_install_modal_message.contains("obsolete"));
}

#[test]
fn cli_update_ignores_result_after_source_switch() {
    use super::PanelMessage as ViewMessage;
    use desktop_release::UpdateSource;
    let mut app = Fixture::new();
    let _ = update_fixture(&mut app, ViewMessage::CheckCliToolUpdate);
    let request = app.desktop_update.request;
    let _ = update_fixture(&mut app, ViewMessage::AppUpdateSourceSelected(UpdateSource::Github));
    let _ = update_fixture(
        &mut app,
        ViewMessage::CheckCliToolUpdateFinished(request, UpdateSource::Shiju, Ok("99.0.0".into())),
    );
    assert_eq!(app.cli_install_modal_server_version, "未检测");
    assert!(!app.desktop_update.checking);
    assert_eq!(app.desktop_update.source, Some(UpdateSource::Github));
}
