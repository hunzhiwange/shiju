//! 验证 macOS 授权与展示能力的边界，不访问真实系统设置。

use super::validate_macos_settings;
use mac_usernotifications::{AuthorizationStatus, NotificationSettingStatus, NotificationSettings};

fn settings() -> NotificationSettings {
    NotificationSettings {
        authorization_status: AuthorizationStatus::Authorized,
        alert_enabled: NotificationSettingStatus::Enabled,
        badge_enabled: NotificationSettingStatus::Enabled,
        sound_enabled: NotificationSettingStatus::Enabled,
        lock_screen_enabled: NotificationSettingStatus::Enabled,
        notification_center_enabled: NotificationSettingStatus::Enabled,
    }
}

#[test]
fn notification_permission_rejects_denied_pending_and_quiet_authorization() {
    for status in [
        AuthorizationStatus::Denied,
        AuthorizationStatus::NotDetermined,
        AuthorizationStatus::Provisional,
        AuthorizationStatus::Unknown,
    ] {
        let mut current = settings();
        current.authorization_status = status;
        assert!(validate_macos_settings(current).is_err());
    }
}

#[test]
fn notification_permission_requires_visible_alerts() {
    let mut current = settings();
    assert!(validate_macos_settings(current).is_ok());
    current.alert_enabled = NotificationSettingStatus::Disabled;
    assert!(validate_macos_settings(current).is_err());
}
