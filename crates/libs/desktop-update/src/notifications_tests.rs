use super::*;
fn release(version: &str) -> DesktopRelease { DesktopRelease { source: UpdateSource::Shiju, version: version.into(), assets: vec![] } }
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
