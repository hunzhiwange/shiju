//! Download completion preserves measured progress and permits a fresh retry.
#[cfg(not(target_arch = "wasm32"))]
#[test]
fn desktop_integration_48_preserves_expected_behavior() {
    use super::message::{ViewMessage, view};
    use vw_shared::update::{DesktopRelease, UpdateProgress, UpdateSource};

    let mut app = super::App::new().0;
    app.desktop_update = super::desktop_update::DesktopUpdate::from_config(&serde_json::json!({}));
    let release = DesktopRelease {
        source: UpdateSource::Shiju,
        version: "99.0.0".into(),
        assets: vec![],
    };
    app.desktop_update.release = Some(release.clone());
    let _ = view::update(&mut app, ViewMessage::RunAppUpdate);
    let (sender, receiver) = std::sync::mpsc::channel();
    app.app_update_progress_rx = Some(receiver);
    let mut progress = UpdateProgress {
        asset_name: "installer.dmg".into(),
        download_url: "https://example.invalid/installer.dmg".into(),
        downloaded_bytes: 17,
        total_bytes: Some(100),
        bytes_per_second: 17.0,
        average_bytes_per_second: 17.0,
    };
    sender.send(progress.clone()).unwrap();
    let _ = view::update(&mut app, ViewMessage::AppUpdateProgressTick);
    assert_eq!(app.app_update_progress, Some(progress.clone()));
    // Completion can beat the next tick, including a queue longer than its 48-event budget.
    for bytes in 18..=80 {
        progress.downloaded_bytes = bytes;
        sender.send(progress.clone()).unwrap();
    }
    drop(sender);
    let _ = view::update(&mut app, ViewMessage::AppUpdatePrepared(Err("connection lost".into())));
    assert_eq!(app.app_update_progress, Some(progress));
    assert!(!app.desktop_update.busy);
    assert!(app.app_update_progress_rx.is_none());
    assert!(app.cli_install_modal_show_install_action);
    assert_eq!(app.desktop_update.release, Some(release));
    let retry = view::update(&mut app, ViewMessage::RunAppUpdate);
    assert!(retry.units() > 0);
    assert!(app.desktop_update.busy);
    assert!(app.app_update_progress.is_none());
    assert!(app.app_update_progress_rx.is_some());
}
