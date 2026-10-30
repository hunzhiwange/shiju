use super::*;
use crate::app::desktop_update::DesktopUpdate;
use vw_shared::update::{DesktopRelease, UpdateSource};

fn release(version: &str) -> DesktopRelease {
    DesktopRelease { source: UpdateSource::Shiju, version: version.into(), assets: vec![] }
}

#[test]
fn automatic_update_is_silent_preserves_results_on_failure_and_rejects_stale_replies() {
    let mut app = App::new().0;
    let _ = update(&mut app, ViewMessage::AutoCheckAppUpdate);
    let request = app.desktop_update.request;
    let _ = update(&mut app, ViewMessage::AutoCheckAppUpdate);
    assert_eq!(app.desktop_update.request, request);
    let _ = update(
        &mut app,
        ViewMessage::AutoCheckAppUpdateFinished(
            request,
            UpdateSource::Shiju,
            Ok(release("99.0.0")),
        ),
    );
    assert!(app.desktop_update.notification_release().is_some());
    assert!(!app.show_cli_install_modal);
    let _ = update(&mut app, ViewMessage::AutoCheckAppUpdate);
    let request = app.desktop_update.request;
    let _ = update(
        &mut app,
        ViewMessage::AutoCheckAppUpdateFinished(
            request,
            UpdateSource::Shiju,
            Err("offline".into()),
        ),
    );
    assert!(app.desktop_update.notification_release().is_some());
    let _ = update(&mut app, ViewMessage::AutoCheckAppUpdate);
    let request = app.desktop_update.request;
    app.desktop_update.request += 1;
    let _ = update(
        &mut app,
        ViewMessage::AutoCheckAppUpdateFinished(
            request,
            UpdateSource::Shiju,
            Ok(release("100.0.0")),
        ),
    );
    assert_eq!(app.desktop_update.release.as_ref().unwrap().version, "99.0.0");
    assert!(app.desktop_update.background_request.is_none());
}

#[test]
fn skipped_release_survives_restart_but_newer_release_notifies_and_manual_install_remains_available()
 {
    let skipped = SkippedRelease { source: UpdateSource::Shiju, version: "99.0.0".into() };
    let mut state =
        DesktopUpdate::from_config(&serde_json::json!({"app_update_skipped_release": skipped}));
    state.release = Some(release("99.0.0"));
    assert!(state.notification_release().is_none());
    assert!(state.installable());
    state.release = Some(release("100.0.0"));
    assert!(state.notification_release().is_some());
    state.source = Some(UpdateSource::Github);
    assert!(state.notification_release().is_none());
    state.source = Some(UpdateSource::Shiju);
    state.release = Some(release(env!("CARGO_PKG_VERSION")));
    assert!(state.notification_release().is_none());
    assert_eq!(crate::app::desktop_update::UPDATE_CHECK_INTERVAL.as_secs(), 10800);
}

#[test]
fn skip_only_hides_notification_after_successful_save() {
    let mut app = App::new().0;
    app.desktop_update.release = Some(release("99.0.0"));
    let _ = update(&mut app, ViewMessage::SkipAppUpdate);
    assert!(app.desktop_update.saving_skip);
    assert!(app.desktop_update.notification_release().is_some());
    let skipped = SkippedRelease { source: UpdateSource::Shiju, version: "99.0.0".into() };
    let _ =
        update(&mut app, ViewMessage::SkipAppUpdateSaved(skipped.clone(), Err("offline".into())));
    assert!(app.desktop_update.notification_release().is_some());
    assert!(!app.desktop_update.saving_skip);
    let _ = update(&mut app, ViewMessage::SkipAppUpdateSaved(skipped, Ok(())));
    assert!(app.desktop_update.notification_release().is_none());
}

#[test]
fn automatic_update_does_not_interrupt_manual_update_or_installation() {
    let mut app = App::new().0;
    app.desktop_update.checking = true;
    let _ = update(&mut app, ViewMessage::AutoCheckAppUpdate);
    assert!(app.desktop_update.background_request.is_none());
    app.desktop_update.checking = false;
    app.desktop_update.busy = true;
    let _ = update(&mut app, ViewMessage::AutoCheckAppUpdate);
    assert!(app.desktop_update.background_request.is_none());
}
