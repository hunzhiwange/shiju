// 验证通用设置标签、主题画廊与时区持久化状态。
// 测试夹具围绕目标分支构造可控状态；跨模块行为由对应集成测试负责。

use super::*;
use crate::app::message::{GeneralSettingsMessage, SettingsMessage};
use crate::app::state::GeneralSettingsTab;

// 状态转换：覆盖交互前后的字段同步、选择关系与清理规则。
// 连续操作仍需保持模块不变量，并清除已经失效的瞬态反馈。
#[test]
fn tab_selection_updates_general_settings_content() {
    let mut app = App::new().0;

    let _ = update(
        &mut app,
        SettingsMessage::General(GeneralSettingsMessage::TabSelected(GeneralSettingsTab::Editor)),
    );

    assert_eq!(app.general_settings_tab, GeneralSettingsTab::Editor);
}

// 失败与边界：覆盖非法、缺失、过期或冲突场景的既定处理规则。
// 重点确认现有状态与错误反馈不会被意外的部分更新污染。
#[test]
fn theme_tab_selection_clears_error_and_refreshes_gallery() {
    let mut app = App::new().0;
    app.themes_settings.error = Some("stale".to_string());

    let task = update(
        &mut app,
        SettingsMessage::General(GeneralSettingsMessage::TabSelected(GeneralSettingsTab::Themes)),
    );

    assert_eq!(app.general_settings_tab, GeneralSettingsTab::Themes);
    assert_eq!(app.themes_settings.error, None);
    assert!(task.units() > 0);
}

// 数据边界：覆盖规范化、默认值以及相关状态字段的既定规则。
// 这里固定可观察结果，避免依赖处理过程中的临时实现细节。
#[cfg(not(target_arch = "wasm32"))]
#[test]
fn timezone_selection_updates_immediately_and_persists_last_choice() {
    use crate::app::config::{drive_system_settings_patch, with_editor_settings_gateway};
    use serde_json::json;

    with_editor_settings_gateway(|mut app, requests, responses| {
        let select = |app: &mut App, timezone: &str| {
            app.update(Message::Settings(SettingsMessage::General(
                GeneralSettingsMessage::TimezoneSelected(timezone.to_owned()),
            )))
        };
        let task = select(&mut app, "America/New_York");
        assert_eq!(app.chat_timezone, "America/New_York");
        assert!(requests.try_recv().is_err());

        let saved = drive_system_settings_patch(
            task,
            &requests,
            &responses,
            json!({"timezone": "America/New_York"}),
            Some("{}"),
            || {
                assert_eq!(select(&mut app, "Asia/Shanghai").units(), 0);
                assert_eq!(select(&mut app, "UTC").units(), 0);
                assert_eq!(select(&mut app, "Mars/Olympus_Mons").units(), 0);
                assert_eq!(app.chat_timezone, "UTC");
            },
        );
        assert!(matches!(&saved, Message::SystemSettingsSaved(_, _, Ok(()))));
        let saved = drive_system_settings_patch(
            app.update(saved),
            &requests,
            &responses,
            json!({"timezone": "UTC"}),
            Some("{}"),
            || {},
        );
        assert!(matches!(&saved, Message::SystemSettingsSaved(_, _, Ok(()))));
        assert_eq!(app.update(saved).units(), 0);
        assert_eq!(app.chat_timezone, "UTC");
    });
}

#[test]
fn desktop_language_selection_survives_reload() {
    use vw_config_types::ui::{AppSystemSettingsConfig, DesktopLocale};
    let mut app = App::new().0;
    for locale in [
        DesktopLocale::SimplifiedChinese,
        DesktopLocale::TraditionalChinese,
        DesktopLocale::English,
    ] {
        let _ = update(
            &mut app,
            SettingsMessage::General(GeneralSettingsMessage::LocaleSelected(locale)),
        );
        app.apply_bootstrap_system_settings(AppSystemSettingsConfig::default());
        assert_eq!(app.desktop_locale, locale);
    }
}

#[test]
fn desktop_language_restores_and_reports_save_failure() {
    use vw_config_types::ui::{AppSystemSettingsConfig, DesktopLocale};
    let mut app = App::new().0;
    app.apply_bootstrap_system_settings(AppSystemSettingsConfig {
        desktop_locale: DesktopLocale::English,
        ..Default::default()
    });
    assert_eq!(app.desktop_locale, DesktopLocale::English);
    let _ = update(
        &mut app,
        SettingsMessage::General(GeneralSettingsMessage::LocaleSelected(
            DesktopLocale::TraditionalChinese,
        )),
    );
    assert!(app.desktop_locale_saving);
    let _ = update(
        &mut app,
        SettingsMessage::General(GeneralSettingsMessage::LocaleSaved(
            DesktopLocale::TraditionalChinese,
            Err("raw error".into()),
        )),
    );
    assert!(!app.desktop_locale_saving);
    assert!(app.active_toast.is_some());
}

#[test]
fn login_launch_failure_preserves_previous_state_and_unlocks_control() {
    let mut app = App::new().0;
    app.launch_at_login = true;
    app.launch_at_login_busy = true;
    let feedback = update(
        &mut app,
        SettingsMessage::General(GeneralSettingsMessage::LaunchAtLoginSaved(Err(
            "access denied".into()
        ))),
    );
    assert!(app.launch_at_login);
    assert!(!app.launch_at_login_busy);
    assert!(feedback.units() > 0);
    let _ = update(
        &mut app,
        SettingsMessage::General(GeneralSettingsMessage::LaunchAtLoginSaved(Ok(false))),
    );
    assert!(!app.launch_at_login);
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn tray_quit_starts_shutdown_even_when_close_to_tray_is_enabled() {
    let mut app = App::new().0;
    app.main_window_id = Some(iced::window::Id::unique());
    app.close_to_tray = true;
    let _ = update(&mut app, SettingsMessage::General(GeneralSettingsMessage::TrayQuit));
    assert!(app.shutdown_requested);
}

#[test]
fn notification_settings_restore_and_report_delivery_failure() {
    let mut app = App::new().0;
    app.apply_bootstrap_system_settings(vw_config_types::ui::AppSystemSettingsConfig {
        task_completion_notification: true,
        ..Default::default()
    });
    assert!(app.task_completion_notification);
    let _ = update(
        &mut app,
        SettingsMessage::General(GeneralSettingsMessage::TaskCompletionNotificationSent(Err(
            "denied".into(),
        ))),
    );
    assert!(app.active_toast.is_some());
}

#[cfg(any(target_os = "macos", target_os = "windows"))]
#[test]
fn notification_setting_serializes_rapid_toggle_saves() {
    let mut app = App::new().0;
    let first = update(
        &mut app,
        SettingsMessage::General(GeneralSettingsMessage::TaskCompletionNotificationToggled(true)),
    );
    assert!(first.units() > 0);
    assert!(app.task_completion_notification_saving);
    let second = update(
        &mut app,
        SettingsMessage::General(GeneralSettingsMessage::TaskCompletionNotificationToggled(false)),
    );
    assert_eq!(second.units(), 0);
    assert!(!app.task_completion_notification);
    let pending = update(
        &mut app,
        SettingsMessage::General(GeneralSettingsMessage::TaskCompletionNotificationSaved(
            true,
            Ok(()),
        )),
    );
    assert!(pending.units() > 0);
    assert!(app.task_completion_notification_saving);
    let _ = update(
        &mut app,
        SettingsMessage::General(GeneralSettingsMessage::TaskCompletionNotificationSaved(
            false,
            Ok(()),
        )),
    );
    assert!(!app.task_completion_notification_saving);
    assert!(!app.task_completion_notification);
}

#[cfg(any(target_os = "macos", target_os = "windows"))]
#[test]
fn notification_permission_failure_disables_and_persists_notification_setting() {
    let mut app = App::new().0;
    app.task_completion_notification = true;
    let task = update(
        &mut app,
        SettingsMessage::General(
            GeneralSettingsMessage::TaskCompletionNotificationPermissionChecked(Err(
                "permission denied".into(),
            )),
        ),
    );
    assert!(!app.task_completion_notification);
    assert!(app.task_completion_notification_saving);
    assert!(app.active_toast.is_some());
    assert!(task.units() > 0);
}
