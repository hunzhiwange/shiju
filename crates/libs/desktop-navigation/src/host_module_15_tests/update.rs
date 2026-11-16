//! Actual update controller with only its external menu port observed locally.
#[derive(Default)]
struct UpdateFixture {
    update_panel: desktop_update::panel::PanelState,
    desktop_update: desktop_update::app::desktop_update::DesktopUpdate,
    navigation_panels: crate::panel::settings::State,
}
fn update(
    app: &mut UpdateFixture,
    message: desktop_update::panel::PanelMessage,
) -> iced::Task<desktop_update::panel::PanelMessage> {
    let mut effects = Vec::new();
    let task = desktop_update::panel::update(
        &mut app.update_panel,
        &mut app.desktop_update,
        &mut effects,
        env!("CARGO_PKG_VERSION"),
        #[cfg(not(target_arch = "wasm32"))]
        config_ui::DesktopLocale::default(),
        message,
    );
    for effect in effects {
        if let desktop_update::panel::Effect::CloseMenu = effect {
            app.navigation_panels.active_menu = None;
        }
    }
    task
}
#[test]
fn app_update_modal_initializes_application_update_state() {
    use crate::components::top_bar::menus::MenuType;
    use desktop_update::panel::PanelMessage as ViewMessage;

    let mut app = UpdateFixture::default();
    app.navigation_panels.active_menu = Some(MenuType::Help);
    app.update_panel.cli_install_modal_server_version = "stale".into();
    app.update_panel.cli_install_modal_show_install_action = true;
    app.update_panel.cli_install_modal_use_app_update_action = false;
    app.update_panel.cli_install_modal_is_checking_update = true;

    let task = update(&mut app, ViewMessage::OpenAppUpdateModal);

    assert_eq!(app.navigation_panels.active_menu, None);
    assert!(app.update_panel.show_cli_install_modal);
    assert_eq!(app.update_panel.cli_install_modal_title, "检测更新");
    assert_eq!(app.update_panel.cli_install_modal_server_version, "未检测");
    assert!(app.update_panel.cli_install_modal_show_update_action);
    assert!(!app.update_panel.cli_install_modal_show_install_action);
    assert!(app.update_panel.cli_install_modal_use_app_update_action);
    assert!(app.update_panel.cli_install_modal_is_checking_update);
    assert!(app.desktop_update.checking);
    assert!(task.units() > 0);
    assert!(app.update_panel.app_update_progress.is_none());
    assert!(app.update_panel.app_update_progress_rx.is_none());

    let request = app.desktop_update.request;
    let duplicate = update(&mut app, ViewMessage::OpenAppUpdateModal);
    assert_eq!(duplicate.units(), 0);
    assert_eq!(app.desktop_update.request, request);
    let source = app.desktop_update.source.unwrap();
    let _ = update(
        &mut app,
        ViewMessage::CheckAppUpdateFinished(request, source, Err("offline".into())),
    );
    let retry = update(&mut app, ViewMessage::CheckAppUpdate);
    assert!(retry.units() > 0);
    assert!(app.desktop_update.checking);
}

#[test]
fn install_and_update_requests_prepare_modal_state_before_async_work_runs() {
    use crate::components::top_bar::menus::MenuType;
    use desktop_update::panel::PanelMessage as ViewMessage;

    let mut app = UpdateFixture::default();
    app.navigation_panels.active_menu = Some(MenuType::Help);

    let task = update(&mut app, ViewMessage::InstallCliTool);
    assert_eq!(app.navigation_panels.active_menu, None);
    assert!(app.update_panel.show_cli_install_modal);
    assert_eq!(app.update_panel.cli_install_modal_title, "CLI 更新检测");
    assert_eq!(app.update_panel.cli_install_modal_server_version, "未检测");
    assert!(app.update_panel.cli_install_modal_show_update_action);
    assert!(app.update_panel.cli_install_modal_show_install_action);
    assert!(!app.update_panel.cli_install_modal_use_app_update_action);
    assert!(app.update_panel.cli_install_modal_is_checking_update);
    assert!(app.desktop_update.checking);
    assert!(task.units() > 0);

    let request = app.desktop_update.request;
    let duplicate = update(&mut app, ViewMessage::CheckCliToolUpdate);
    assert_eq!(duplicate.units(), 0);
    assert_eq!(app.desktop_update.request, request);
    let source = app.desktop_update.source.unwrap();
    let _ = update(
        &mut app,
        ViewMessage::CheckCliToolUpdateFinished(request, source, Err("offline".into())),
    );
    let task = update(&mut app, ViewMessage::CheckCliToolUpdate);
    assert!(task.units() > 0);
    assert!(app.update_panel.cli_install_modal_is_checking_update);
    assert!(app.update_panel.cli_install_modal_message.contains("最新 CLI 版本"));

    let request = app.desktop_update.request;
    let _ = update(
        &mut app,
        ViewMessage::CheckCliToolUpdateFinished(request, source, Err("offline".into())),
    );
    let _ = update(&mut app, ViewMessage::CheckAppUpdate);
    assert!(app.update_panel.cli_install_modal_is_checking_update);
    assert!(!app.update_panel.cli_install_modal_show_install_action);
    assert!(app.update_panel.app_update_progress.is_none());
    assert!(app.update_panel.app_update_progress_rx.is_none());

    let _ = update(&mut app, ViewMessage::RunAppUpdate);
    assert!(app.update_panel.cli_install_modal_is_checking_update);
    assert!(!app.update_panel.cli_install_modal_show_install_action);
    assert!(app.update_panel.app_update_progress_rx.is_none()); // No checked snapshot: no download.

    let _ = update(&mut app, ViewMessage::RunInstallCliTool);
    assert!(!app.update_panel.cli_install_modal_is_checking_update);
}

#[test]
fn app_update_progress_drains_available_values_and_drops_disconnected_receiver() {
    use desktop_update::panel::PanelMessage as ViewMessage;

    let mut app = UpdateFixture::default();
    let (sender, receiver) = std::sync::mpsc::channel();
    let first = update_types::UpdateProgress {
        asset_name: "app".into(),
        download_url: "https://example.invalid/first".into(),
        downloaded_bytes: 10,
        total_bytes: Some(100),
        bytes_per_second: 2.0,
        average_bytes_per_second: 1.0,
    };
    let mut latest = first.clone();
    latest.downloaded_bytes = 20;
    sender.send(first).expect("first progress should be queued");
    sender.send(latest.clone()).expect("latest progress should be queued");
    drop(sender);
    app.update_panel.app_update_progress_rx = Some(receiver);

    let _ = update(&mut app, ViewMessage::AppUpdateProgressTick);

    assert_eq!(app.update_panel.app_update_progress, Some(latest));
    assert!(app.update_panel.app_update_progress_rx.is_none());
}

#[test]
fn install_result_resets_update_state_and_sets_outcome_copy() {
    use desktop_update::panel::PanelMessage as ViewMessage;

    let mut app = UpdateFixture::default();
    app.update_panel.cli_install_modal_current_version = "old".into();
    app.update_panel.cli_install_modal_server_version = "new".into();
    app.update_panel.cli_install_modal_show_update_action = true;
    app.update_panel.cli_install_modal_show_install_action = true;
    app.update_panel.cli_install_modal_use_app_update_action = true;
    app.update_panel.cli_install_modal_is_checking_update = true;

    let _ = update(&mut app, ViewMessage::InstallCliToolFinished(Ok("installed".into())));

    assert!(app.update_panel.show_cli_install_modal);
    assert_eq!(app.update_panel.cli_install_modal_title, "CLI 安装完成");
    assert_eq!(app.update_panel.cli_install_modal_message, "installed");
    assert!(app.update_panel.cli_install_modal_current_version.is_empty());
    assert!(app.update_panel.cli_install_modal_server_version.is_empty());
    assert!(!app.update_panel.cli_install_modal_show_update_action);
    assert!(!app.update_panel.cli_install_modal_show_install_action);
    assert!(!app.update_panel.cli_install_modal_use_app_update_action);
    assert!(!app.update_panel.cli_install_modal_is_checking_update);

    let _ = update(&mut app, ViewMessage::InstallCliToolFinished(Err("permission denied".into())));
    assert_eq!(app.update_panel.cli_install_modal_title, "CLI 安装失败");
    assert_eq!(app.update_panel.cli_install_modal_message, "permission denied");
}

#[test]
fn app_update_download_survives_close_and_blocks_duplicate_and_source_change() {
    use desktop_release::{DesktopRelease, UpdateSource};
    use desktop_update::panel::PanelMessage as ViewMessage;
    let mut app = UpdateFixture::default();
    let _ = update(&mut app, ViewMessage::OpenAppUpdateModal);
    app.desktop_update.checking = false;
    app.desktop_update.release = Some(DesktopRelease {
        source: UpdateSource::Shiju,
        version: "99.0.0".into(),
        assets: vec![],
    });
    let snapshot = app.desktop_update.release.clone();
    let _ = update(&mut app, ViewMessage::RunAppUpdate);
    assert!(app.desktop_update.busy);
    assert!(app.update_panel.app_update_progress_rx.is_some());
    let _ = update(&mut app, ViewMessage::CloseInstallCliModal);
    let _ = update(&mut app, ViewMessage::OpenAppUpdateModal);
    let duplicate = update(&mut app, ViewMessage::RunAppUpdate);
    assert!(duplicate.units() == 0);
    let _ = update(&mut app, ViewMessage::AppUpdateSourceSelected(UpdateSource::Github));
    let _ = update(&mut app, ViewMessage::CheckAppUpdate);
    assert_eq!(app.desktop_update.release, snapshot);
    assert_eq!(app.desktop_update.source, Some(UpdateSource::Shiju));
    assert!(app.update_panel.app_update_progress_rx.is_some());
}

#[tokio::test(flavor = "multi_thread")]
async fn app_update_download_consumes_checked_snapshot_without_refetching_latest() {
    use desktop_release::{DesktopRelease, UpdateSource};
    use desktop_update::panel::PanelMessage as ViewMessage;
    use iced_runtime::futures::futures::StreamExt;
    let mut app = UpdateFixture::default();
    app.desktop_update.source = Some(UpdateSource::Github);
    // A deliberately incomplete snapshot must fail its own asset validation before network I/O.
    app.desktop_update.release = Some(DesktopRelease {
        source: UpdateSource::Github,
        version: "99.0.0".into(),
        assets: vec![],
    });
    let task = update(&mut app, ViewMessage::RunAppUpdate);
    let mut stream = iced_runtime::task::into_stream(task).unwrap();
    let action = tokio::time::timeout(std::time::Duration::from_secs(5), stream.next())
        .await
        .unwrap()
        .unwrap();
    match action {
        iced_runtime::Action::Output(ViewMessage::AppUpdateDownloadPrepared(_, Err(error))) => {
            assert!(error.contains("exactly six"), "{error}");
        }
        _ => panic!("expected snapshot validation failure"),
    }
}
