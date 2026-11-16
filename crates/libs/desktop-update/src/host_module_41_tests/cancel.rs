use super::*;
// Cancellation invalidates in-flight results and leaves the release retryable.
#[cfg(not(target_arch = "wasm32"))]
#[test]
fn desktop_integration_49_preserves_expected_behavior() {
    use desktop_release::{DesktopRelease, UpdateSource};

    let mut app = Fixture::new();
    app.desktop_update =
        crate::app::desktop_update::DesktopUpdate::from_config(&serde_json::json!({}));
    let release =
        DesktopRelease { source: UpdateSource::Shiju, version: "99.0.0".into(), assets: vec![] };
    app.desktop_update.release = Some(release.clone());
    let task = app.update(ViewMessage::RunAppUpdate);
    assert!(task.units() > 0);
    let cancelled_request = app.desktop_update.request;
    assert!(app.desktop_update.download_abort.is_some());
    let _ = app.update(ViewMessage::CancelAppUpdate);
    assert!(!app.desktop_update.busy);
    assert!(app.desktop_update.ready.is_none());
    assert!(app.desktop_update.download_abort.is_none());
    assert!(app.update_panel.app_update_progress_rx.is_none());
    assert!(app.update_panel.cli_install_modal_show_install_action);
    assert!(!app.update_panel.cli_install_modal_is_checking_update);
    assert_eq!(app.desktop_update.release, Some(release));

    let retry = app.update(ViewMessage::RunAppUpdate);
    assert!(retry.units() > 0);
    let retry_request = app.desktop_update.request;
    assert_ne!(retry_request, cancelled_request);
    let _ = app.update(ViewMessage::AppUpdateDownloadPrepared(
        cancelled_request,
        Err("old download".into()),
    ));
    assert!(app.desktop_update.busy);
    assert!(app.desktop_update.download_abort.is_some());
    assert!(app.update_panel.app_update_progress_rx.is_some());
    assert!(!app.update_panel.cli_install_modal_message.contains("old download"));
    let _ = app
        .update(ViewMessage::AppUpdateDownloadPrepared(retry_request, Err("retry failed".into())));
    assert!(!app.desktop_update.busy);
    assert!(app.desktop_update.download_abort.is_none());
    assert!(app.update_panel.cli_install_modal_show_install_action);
    // Cancellation cannot interrupt an installer once download preparation has ended.
    app.desktop_update.busy = true;
    let _ = app.update(ViewMessage::CancelAppUpdate);
    assert!(app.desktop_update.busy);
}
