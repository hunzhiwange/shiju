use desktop_release::{DesktopRelease, UpdateSource};
use desktop_update::app::desktop_update::SkippedRelease;
use desktop_update::{app::desktop_update::DesktopUpdate, notifications as domain};
use domain::Message as ViewMessage;

fn release(version: &str) -> DesktopRelease {
    DesktopRelease { source: UpdateSource::Shiju, version: version.into(), assets: vec![] }
}

#[test]
fn automatic_update_is_silent_preserves_results_on_failure_and_rejects_stale_replies() {
    let mut app = NotificationFixture::default();
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
    assert!(!app.update_panel.show_cli_install_modal);
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
fn skip_only_hides_notification_after_successful_save() {
    let mut app = NotificationFixture::default();
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
    let mut app = NotificationFixture::default();
    app.desktop_update.checking = true;
    let _ = update(&mut app, ViewMessage::AutoCheckAppUpdate);
    assert!(app.desktop_update.background_request.is_none());
    app.desktop_update.checking = false;
    app.desktop_update.busy = true;
    let _ = update(&mut app, ViewMessage::AutoCheckAppUpdate);
    assert!(app.desktop_update.background_request.is_none());
}

struct NotificationFixture {
    desktop_update: DesktopUpdate,
    update_panel: desktop_update::panel::PanelState,
}
fn update(
    app: &mut NotificationFixture,
    message: domain::Message,
) -> (iced::Task<domain::Message>, Option<domain::Effect>) {
    domain::update(
        &mut app.desktop_update,
        app.update_panel.show_cli_install_modal,
        env!("CARGO_PKG_VERSION"),
        message,
    )
}

impl Default for NotificationFixture { fn default() -> Self { Self { update_panel: Default::default(), desktop_update: DesktopUpdate::from_config(&serde_json::json!({})) } } }
