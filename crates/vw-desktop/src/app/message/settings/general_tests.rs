// 验证通用设置标签、主题画廊与时区持久化状态。
// 测试夹具围绕目标分支构造可控状态；跨模块行为由对应集成测试负责。

use super::*;
use crate::app::message::{GeneralSettingsMessage, SettingsMessage};

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
        assert_eq!(app.general_settings.chat_timezone, "America/New_York");
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
                assert_eq!(app.general_settings.chat_timezone, "UTC");
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
        assert_eq!(app.general_settings.chat_timezone, "UTC");
    });
}

#[test]
fn desktop_language_selection_survives_reload() {
    use config_ui::{AppSystemSettingsConfig, DesktopLocale};
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
        assert_eq!(app.general_settings.desktop_locale, locale);
    }
}

#[test]
fn desktop_language_restores_and_reports_save_failure() {
    use config_ui::{AppSystemSettingsConfig, DesktopLocale};
    let mut app = App::new().0;
    app.apply_bootstrap_system_settings(AppSystemSettingsConfig {
        desktop_locale: DesktopLocale::English,
        ..Default::default()
    });
    assert_eq!(app.general_settings.desktop_locale, DesktopLocale::English);
    let _ = update(
        &mut app,
        SettingsMessage::General(GeneralSettingsMessage::LocaleSelected(
            DesktopLocale::TraditionalChinese,
        )),
    );
    assert!(app.general_settings.desktop_locale_saving);
    let _ = update(
        &mut app,
        SettingsMessage::General(GeneralSettingsMessage::LocaleSaved(
            DesktopLocale::TraditionalChinese,
            Err("raw error".into()),
        )),
    );
    assert!(!app.general_settings.desktop_locale_saving);
    assert!(app.toast_state.active_toast.is_some());
}

#[test]
fn login_launch_failure_preserves_previous_state_and_unlocks_control() {
    let mut app = App::new().0;
    app.lifecycle.launch_at_login = true;
    app.lifecycle.launch_at_login_busy = true;
    let feedback = update(
        &mut app,
        SettingsMessage::General(GeneralSettingsMessage::LaunchAtLoginSaved(Err(
            "access denied".into()
        ))),
    );
    assert!(app.lifecycle.launch_at_login);
    assert!(!app.lifecycle.launch_at_login_busy);
    assert!(feedback.units() > 0);
    let _ = update(
        &mut app,
        SettingsMessage::General(GeneralSettingsMessage::LaunchAtLoginSaved(Ok(false))),
    );
    assert!(!app.lifecycle.launch_at_login);
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn tray_quit_starts_shutdown_even_when_close_to_tray_is_enabled() {
    let mut app = App::new().0;
    app.main_window_id = Some(iced::window::Id::unique());
    app.lifecycle.close_to_tray = true;
    let _ = update(&mut app, SettingsMessage::General(GeneralSettingsMessage::TrayQuit));
    assert!(app.shutdown_requested);
}

#[test]
fn notification_settings_restore_and_report_delivery_failure() {
    let mut app = App::new().0;
    app.apply_bootstrap_system_settings(config_ui::AppSystemSettingsConfig {
        task_completion_notification: true,
        ..Default::default()
    });
    assert!(app.general_settings.task_completion_notification);
    let _ = update(
        &mut app,
        SettingsMessage::General(GeneralSettingsMessage::TaskCompletionNotificationSent(Err(
            "denied".into(),
        ))),
    );
    assert!(app.toast_state.active_toast.is_some());
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
    assert!(app.general_settings.task_completion_notification_saving);
    let second = update(
        &mut app,
        SettingsMessage::General(GeneralSettingsMessage::TaskCompletionNotificationToggled(false)),
    );
    assert_eq!(second.units(), 0);
    assert!(!app.general_settings.task_completion_notification);
    let pending = update(
        &mut app,
        SettingsMessage::General(GeneralSettingsMessage::TaskCompletionNotificationSaved(
            true,
            Ok(()),
        )),
    );
    assert!(pending.units() > 0);
    assert!(app.general_settings.task_completion_notification_saving);
    let _ = update(
        &mut app,
        SettingsMessage::General(GeneralSettingsMessage::TaskCompletionNotificationSaved(
            false,
            Ok(()),
        )),
    );
    assert!(!app.general_settings.task_completion_notification_saving);
    assert!(!app.general_settings.task_completion_notification);
}

#[cfg(any(target_os = "macos", target_os = "windows"))]
#[test]
fn notification_permission_failure_disables_and_persists_notification_setting() {
    let mut app = App::new().0;
    app.general_settings.task_completion_notification = true;
    let task = update(
        &mut app,
        SettingsMessage::General(
            GeneralSettingsMessage::TaskCompletionNotificationPermissionChecked(Err(
                "permission denied".into(),
            )),
        ),
    );
    assert!(!app.general_settings.task_completion_notification);
    assert!(app.general_settings.task_completion_notification_saving);
    assert!(app.toast_state.active_toast.is_some());
    assert!(task.units() > 0);
}

#[tokio::test(flavor = "multi_thread")]
async fn desktop_language_switch_refreshes_visible_labels_and_preserves_source_fallback() {
    use crate::app::task::{Task as BoardTask, TaskLogEntry, TaskStatus};
    use crate::app::{AppTab, Screen};
    use config_ui::DesktopLocale;
    use desktop_i18n::I18n;

    let (mut app, startup) = App::new();
    // Drop effect tasks: no bootstrap/config write is polled, and no native window,
    // tray, browser child or MCP panel is created by this fixture.
    drop(startup);
    let pet_window = iced::window::Id::unique();
    app.task_pet_window_id = Some(pet_window);
    let editor_id = desktop_navigation::panel::tabs::EDITOR_TAB_ID;
    for (id, title, screen) in [
        ("cron", "定时任务", Screen::Cron),
        ("projects", "项目管理", Screen::Projects),
        (editor_id, "编辑器", Screen::Preview),
        ("find:language-test", "查找结果 1", Screen::Project),
        ("user-tab", "用户/%s/%n", Screen::Project),
    ] {
        app.navigation_tabs.open_tabs.push(AppTab {
            id: id.into(),
            title: title.into(),
            screen,
            project_path: None,
        });
    }
    app.project_find.open_scope("/tmp/用户/%s/%n".into(), "language-test".into());
    app.navigation_tabs.active_tab_id = Some("user-tab".into());
    app.navigation_tabs.screen = Screen::Project;
    let mut task = BoardTask::default();
    task.logs.push(TaskLogEntry::new_status_change(TaskStatus::Pending, TaskStatus::Running));
    task.logs.push(TaskLogEntry::new_message("用户日志/%s/%n".into()));
    app.task_board.task_board_viewing_logs = Some(task);

    for (locale, cron, projects, editor, search, pet, status) in [
        (
            DesktopLocale::SimplifiedChinese,
            "定时任务",
            "项目管理",
            "编辑器",
            "查找结果 1",
            "VibeWindow 伙伴",
            "状态变更: 待执行 → 执行中",
        ),
        (
            DesktopLocale::English,
            "Scheduled tasks",
            "Project management",
            "Editor",
            "Search results 1",
            "VibeWindow Pet",
            "Status changed: Pending → Running",
        ),
        (
            DesktopLocale::TraditionalChinese,
            "排程任務",
            "專案管理",
            "編輯器",
            "查詢結果 1",
            "VibeWindow 夥伴",
            "狀態變更: 待執行 → 執行中",
        ),
        (
            DesktopLocale::SimplifiedChinese,
            "定时任务",
            "项目管理",
            "编辑器",
            "查找结果 1",
            "VibeWindow 伙伴",
            "状态变更: 待执行 → 执行中",
        ),
    ] {
        // Use the host settings entry, rather than calling refresh helpers directly.
        let save = super::super::update(
            &mut app,
            SettingsMessage::General(GeneralSettingsMessage::LocaleSelected(locale)),
        );
        assert!(save.units() > 0);
        drop(save);
        assert!(app.general_settings.desktop_locale_saving);
        assert!(app.general_settings.desktop_locale_selected);
        assert_eq!(app.general_settings.desktop_locale, locale);
        for (id, expected) in [
            ("cron", cron),
            ("projects", projects),
            (editor_id, editor),
            ("find:language-test", search),
            ("user-tab", "用户/%s/%n"),
        ] {
            assert_eq!(
                app.navigation_tabs.open_tabs.iter().find(|tab| tab.id == id).unwrap().title,
                expected
            );
        }
        assert_eq!(app.title_for_window(pet_window), pet);
        let logs = app.task_board.task_board_logs_editor.text();
        assert!(logs.contains(status), "{logs}");
        assert!(logs.contains("用户日志/%s/%n"));
        assert_eq!(app.navigation_tabs.active_tab_id.as_deref(), Some("user-tab"));
        assert_eq!(app.navigation_tabs.screen, Screen::Project);
        assert_eq!(app.project_find.find_results_tabs[0].scope_path, "/tmp/用户/%s/%n");
        let source = "集成回归专用未收录文案/%s/%n";
        assert_eq!(I18n::new(app.general_settings.desktop_locale).__(source), source);
        assert_eq!(app.general_settings.desktop_locale, locale);
        let saved = super::super::update(
            &mut app,
            SettingsMessage::General(GeneralSettingsMessage::LocaleSaved(locale, Ok(()))),
        );
        assert_eq!(saved.units(), 0);
        assert!(!app.general_settings.desktop_locale_saving);
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn desktop_language_stale_save_keeps_latest_selection_pending() {
    use config_ui::DesktopLocale;
    let (mut app, startup) = App::new();
    drop(startup);
    let first = update(
        &mut app,
        SettingsMessage::General(GeneralSettingsMessage::LocaleSelected(DesktopLocale::English)),
    );
    assert!(first.units() > 0);
    drop(first);
    let second = update(
        &mut app,
        SettingsMessage::General(GeneralSettingsMessage::LocaleSelected(
            DesktopLocale::TraditionalChinese,
        )),
    );
    assert_eq!(second.units(), 0);
    let retry = update(
        &mut app,
        SettingsMessage::General(GeneralSettingsMessage::LocaleSaved(
            DesktopLocale::English,
            Ok(()),
        )),
    );
    assert!(retry.units() > 0);
    drop(retry);
    assert_eq!(app.general_settings.desktop_locale, DesktopLocale::TraditionalChinese);
    assert!(app.general_settings.desktop_locale_saving);
    let _ = update(
        &mut app,
        SettingsMessage::General(GeneralSettingsMessage::LocaleSaved(
            DesktopLocale::TraditionalChinese,
            Err("用户/%s/%n".into()),
        )),
    );
    assert!(!app.general_settings.desktop_locale_saving);
    assert_eq!(app.general_settings.desktop_locale, DesktopLocale::TraditionalChinese);
    assert_eq!(app.toast_state.active_toast.as_ref().unwrap().message, "儲存失敗：用户/%s/%n");
}
