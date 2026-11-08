//! 处理通用设置页签切换与时区持久化消息。

use crate::app::config::SystemSettingsSection;
use crate::app::{App, Message};
use iced::Task;

use super::{GeneralSettingsMessage, SettingsMessage};

/// 仅消费通用设置消息；其它设置分支保持不变并返回空任务。
pub(super) fn update(app: &mut App, message: SettingsMessage) -> Task<Message> {
    let SettingsMessage::General(message) = message else {
        return Task::none();
    };

    match message {
        GeneralSettingsMessage::LaunchAtLoginToggled(enabled) => {
            if app.launch_at_login_busy || cfg!(target_arch = "wasm32") {
                return Task::none();
            }
            app.launch_at_login_busy = true;
            return Task::perform(
                async move {
                    #[cfg(not(target_arch = "wasm32"))]
                    {
                        tokio::task::spawn_blocking(move || {
                            crate::app::desktop_lifecycle::set_launch_at_login(enabled)
                        })
                        .await
                        .map_err(|err| err.to_string())?
                    }
                    #[cfg(target_arch = "wasm32")]
                    {
                        crate::app::desktop_lifecycle::set_launch_at_login(enabled)
                    }
                },
                |result| {
                    Message::Settings(SettingsMessage::General(
                        GeneralSettingsMessage::LaunchAtLoginSaved(result),
                    ))
                },
            );
        }
        GeneralSettingsMessage::LaunchAtLoginSaved(result) => {
            app.launch_at_login_busy = false;
            match result {
                Ok(enabled) => app.launch_at_login = enabled,
                Err(error) => return app.show_error_toast(format!("设置开机启动失败：{error}")),
            }
        }
        GeneralSettingsMessage::CloseToTrayToggled(enabled) => {
            if enabled
                && let Err(error) = crate::app::desktop_lifecycle::ensure_tray(app.desktop_locale)
            {
                return app.show_error_toast(format!("无法启用托盘：{error}"));
            }
            if let Err(error) = crate::app::desktop_lifecycle::save_close_to_tray(enabled) {
                if !app.close_to_tray {
                    crate::app::desktop_lifecycle::remove_tray();
                }
                return app.show_error_toast(format!("保存关闭到托盘失败：{error}"));
            }
            app.close_to_tray = enabled;
            if !enabled {
                crate::app::desktop_lifecycle::remove_tray();
            }
        }
        GeneralSettingsMessage::TrayTick => {
            return Task::batch(
                crate::app::desktop_lifecycle::poll_tray().into_iter().map(|message| {
                    Task::done(Message::Settings(SettingsMessage::General(message)))
                }),
            );
        }
        GeneralSettingsMessage::TrayShow => {
            if let Some(id) = app.main_window_id {
                return crate::app::desktop_lifecycle::show_main_window(id);
            }
        }
        GeneralSettingsMessage::TrayQuit =>
        {
            #[cfg(not(target_arch = "wasm32"))]
            if let Some(id) = app.main_window_id {
                return crate::app::message::view::panel::start_shutdown(app, id);
            }
        }
        GeneralSettingsMessage::LocaleSelected(locale) => {
            app.desktop_locale = locale;
            crate::app::i18n::editor::refresh(app);
            crate::apps::workflow::refresh_top_tab_title(app);
            crate::app::i18n::apps::refresh_hub_titles(app);
            let viewing_task = app.task_board_viewing_logs.clone();
            crate::app::message::task_board::sync_task_logs_editor(app, viewing_task.as_ref());
            let chunks = app
                .active_session_view_state
                .prepared_chat_ui_chunks
                .iter()
                .copied()
                .collect::<Vec<_>>();
            for chunk in chunks {
                app.invalidate_chat_ui_chunk(chunk);
            }
            app.chat_explore_summary_animations.clear();
            super::cron::refresh_run_history_locale(app);
            app.desktop_locale_selected = true;
            if !app.desktop_locale_saving {
                app.desktop_locale_saving = true;
                return save_locale(locale);
            }
        }
        GeneralSettingsMessage::LocaleSaved(locale, result) => {
            app.desktop_locale_saving = false;
            let feedback = match result {
                Ok(()) => Task::none(),
                Err(error) => app.show_error_toast(
                    crate::__!(
                        crate::app::i18n::I18n::new(app.desktop_locale),
                        "保存失败：%s",
                        error
                    )
                    .expect("embedded save failure template has one string argument"),
                ),
            };
            if locale != app.desktop_locale {
                app.desktop_locale_saving = true;
                return Task::batch([feedback, save_locale(app.desktop_locale)]);
            }
            return feedback;
        }
        GeneralSettingsMessage::TabSelected(tab) => {
            app.general_settings_tab = tab;
            if tab == crate::app::state::GeneralSettingsTab::Themes {
                app.themes_settings.error = None;
                return Task::done(Message::Settings(SettingsMessage::Themes(
                    super::ThemesMessage::Refresh,
                )));
            }
        }
        GeneralSettingsMessage::TaskCompletionNotificationToggled(enabled) => {
            if !crate::app::desktop_notification::SUPPORTED {
                return Task::none();
            }
            app.task_completion_notification = enabled;
            #[cfg(not(target_arch = "wasm32"))]
            app.note_startup_setting_edit("task_completion_notification");
            let save = if !app.task_completion_notification_saving {
                app.task_completion_notification_saving = true;
                save_task_completion_notification(enabled)
            } else {
                Task::none()
            };
            let permission = if enabled {
                crate::app::desktop_notification::permission_task(app.desktop_locale)
            } else {
                Task::none()
            };
            return Task::batch([save, permission]);
        }
        GeneralSettingsMessage::TaskCompletionNotificationPermissionChecked(result) => {
            if let Err(error) = result {
                let feedback = app.show_error_toast(error);
                let disable = update(
                    app,
                    SettingsMessage::General(
                        GeneralSettingsMessage::TaskCompletionNotificationToggled(false),
                    ),
                );
                return Task::batch([feedback, disable]);
            }
        }
        GeneralSettingsMessage::TaskCompletionNotificationSaved(enabled, result) => {
            app.task_completion_notification_saving = false;
            let feedback = match result {
                Ok(()) => Task::none(),
                Err(_) => app.show_error_toast(
                    crate::app::i18n::I18n::new(app.desktop_locale)
                        .__("保存通知设置失败，请重试。")
                        .to_owned(),
                ),
            };
            if enabled != app.task_completion_notification {
                app.task_completion_notification_saving = true;
                return Task::batch([
                    feedback,
                    save_task_completion_notification(app.task_completion_notification),
                ]);
            }
            return feedback;
        }
        GeneralSettingsMessage::TestTaskCompletionNotification => {
            return crate::app::desktop_notification::completion_task(app.desktop_locale);
        }
        GeneralSettingsMessage::TaskCompletionNotificationSent(result) => {
            if let Err(error) = result {
                let i18n = crate::app::i18n::I18n::new(app.desktop_locale);
                return app.show_error_toast(format!(
                    "{} {error}",
                    i18n.__("系统通知发送失败，请检查系统通知权限。"),
                ));
            }
        }
        GeneralSettingsMessage::TimezoneSelected(timezone) => {
            if !vw_config_types::ui::timezone_options().contains(&timezone.as_str()) {
                return Task::none();
            }

            app.chat_timezone = timezone.clone();
            return app.save_system_settings_fields(
                SystemSettingsSection::Timezone,
                serde_json::Map::from_iter([("timezone".into(), timezone.into())]),
            );
        }
    }
    Task::none()
}

#[cfg(test)]
#[path = "general_tests.rs"]
mod general_tests;

fn save_locale(locale: vw_config_types::ui::DesktopLocale) -> Task<Message> {
    Task::perform(
        crate::app::config::update_system_settings_config_result_async(move |system| {
            system.desktop_locale = locale;
        }),
        move |result| {
            Message::Settings(SettingsMessage::General(GeneralSettingsMessage::LocaleSaved(
                locale, result,
            )))
        },
    )
}

fn save_task_completion_notification(enabled: bool) -> Task<Message> {
    Task::perform(
        crate::app::config::update_system_settings_config_result_async(move |system| {
            system.task_completion_notification = enabled;
        }),
        move |result| {
            Message::Settings(SettingsMessage::General(
                GeneralSettingsMessage::TaskCompletionNotificationSaved(enabled, result),
            ))
        },
    )
}
