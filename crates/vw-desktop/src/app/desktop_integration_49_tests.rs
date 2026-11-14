//! Cancellation invalidates in-flight results and leaves the release retryable.
#[cfg(not(target_arch = "wasm32"))]
#[test]
fn desktop_integration_49_preserves_expected_behavior() {
    use crate::app::message::{ViewMessage, view};
    use vw_shared::update::{DesktopRelease, UpdateSource};

    let mut app = crate::app::App::new().0;
    app.desktop_update = crate::app::desktop_update::DesktopUpdate::from_config(&serde_json::json!({}));
    let release = DesktopRelease {
        source: UpdateSource::Shiju,
        version: "99.0.0".into(),
        assets: vec![],
    };
    app.desktop_update.release = Some(release.clone());
    let task = view::update(&mut app, ViewMessage::RunAppUpdate);
    assert!(task.units() > 0);
    let cancelled_request = app.desktop_update.request;
    assert!(app.desktop_update.download_abort.is_some());
    let _ = view::update(&mut app, ViewMessage::CancelAppUpdate);
    assert!(!app.desktop_update.busy);
    assert!(app.desktop_update.ready.is_none());
    assert!(app.desktop_update.download_abort.is_none());
    assert!(app.app_update_progress_rx.is_none());
    assert!(app.cli_install_modal_show_install_action);
    assert!(!app.cli_install_modal_is_checking_update);
    assert_eq!(app.desktop_update.release, Some(release));

    let retry = view::update(&mut app, ViewMessage::RunAppUpdate);
    assert!(retry.units() > 0);
    let retry_request = app.desktop_update.request;
    assert_ne!(retry_request, cancelled_request);
    let _ = view::update(
        &mut app,
        ViewMessage::AppUpdateDownloadPrepared(cancelled_request, Err("old download".into())),
    );
    assert!(app.desktop_update.busy);
    assert!(app.desktop_update.download_abort.is_some());
    assert!(app.app_update_progress_rx.is_some());
    assert!(!app.cli_install_modal_message.contains("old download"));
    let _ = view::update(
        &mut app,
        ViewMessage::AppUpdateDownloadPrepared(retry_request, Err("retry failed".into())),
    );
    assert!(!app.desktop_update.busy);
    assert!(app.desktop_update.download_abort.is_none());
    assert!(app.cli_install_modal_show_install_action);
    // Cancellation cannot interrupt an installer once download preparation has ended.
    app.desktop_update.busy = true;
    let _ = view::update(&mut app, ViewMessage::CancelAppUpdate);
    assert!(app.desktop_update.busy);
}
