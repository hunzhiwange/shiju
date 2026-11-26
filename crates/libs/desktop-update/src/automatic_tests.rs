use super::*;

#[test]
fn automatic_download_is_opt_in() {
    let state = crate::app::desktop_update::DesktopUpdate::from_config(&serde_json::json!({}));
    assert_eq!(state.download_policy, DownloadPolicy::default());
    assert!(!state.download_policy.allows(23, || true, || true));
}

#[test]
fn idle_or_night_still_requires_power_when_selected() {
    let policy = DownloadPolicy { enabled: true, idle: true, night: true, plugged_in: true };
    assert!(policy.allows(23, || false, || true));
    assert!(policy.allows(6, || false, || true));
    assert!(!policy.allows(7, || false, || true));
    assert!(policy.allows(7, || true, || true));
    assert!(!policy.allows(23, || true, || false));
}

#[test]
fn legacy_wifi_preference_is_ignored() {
    let state = crate::app::desktop_update::DesktopUpdate::from_config(&serde_json::json!({
        (CONFIG_KEY): {"enabled": true, "wifi": true}
    }));
    let unused = || panic!("unselected condition must not probe the device");
    assert!(state.download_policy.allows(12, unused, unused));
}
