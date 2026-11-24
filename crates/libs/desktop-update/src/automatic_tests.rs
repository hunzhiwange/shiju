use super::*;

#[test]
fn automatic_download_is_opt_in_and_old_preferences_default_to_off() {
    let policy: DownloadPolicy = serde_json::from_value(serde_json::json!({"wifi": true})).unwrap();
    assert!(!policy.allows(23, || true, || true, || true));
    let state = crate::app::desktop_update::DesktopUpdate::from_config(&serde_json::json!({}));
    assert_eq!(state.download_policy, DownloadPolicy::default());
}

#[test]
fn idle_or_night_still_requires_each_selected_device_condition() {
    let policy =
        DownloadPolicy { enabled: true, idle: true, night: true, wifi: true, plugged_in: true };
    assert!(policy.allows(23, || false, || true, || true));
    assert!(policy.allows(6, || false, || true, || true));
    assert!(!policy.allows(7, || false, || true, || true));
    assert!(policy.allows(7, || true, || true, || true));
    assert!(!policy.allows(23, || true, || false, || true));
    assert!(!policy.allows(23, || true, || true, || false));
}

#[test]
fn enabling_without_conditions_does_not_probe_devices() {
    let policy = DownloadPolicy { enabled: true, ..DownloadPolicy::default() };
    let unused = || panic!("unselected condition must not probe the device");
    assert!(policy.allows(12, unused, unused, unused));
}
