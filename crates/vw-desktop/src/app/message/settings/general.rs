//! 处理通用设置页签切换与时区持久化消息。

use crate::app::{App, Message};
use iced::Task;

use super::{GeneralSettingsMessage, SettingsMessage};

#[cfg(test)]
static LAST_PERSISTED_TIMEZONE: std::sync::Mutex<Option<String>> = std::sync::Mutex::new(None);

#[cfg(test)]
fn persist_timezone(timezone: String) {
    *LAST_PERSISTED_TIMEZONE.lock().expect("timezone recorder lock") = Some(timezone);
}

#[cfg(not(test))]
fn persist_timezone(timezone: String) {
    crate::app::update_system_settings_config(move |system| {
        system.timezone = timezone;
    });
}

/// 仅消费通用设置消息；其它设置分支保持不变并返回空任务。
pub(super) fn update(app: &mut App, message: SettingsMessage) -> Task<Message> {
    let SettingsMessage::General(message) = message else {
        return Task::none();
    };

    match message {
        GeneralSettingsMessage::LocaleSelected(locale) => {
            app.desktop_locale = locale;
            crate::apps::workflow::refresh_top_tab_title(app);
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
        GeneralSettingsMessage::TimezoneSelected(timezone) => {
            if !vw_config_types::ui::timezone_options().contains(&timezone.as_str()) {
                return Task::none();
            }

            app.chat_timezone = timezone.clone();
            persist_timezone(timezone);
        }
    }
    Task::none()
}

#[cfg(test)]
fn clear_last_persisted_timezone() {
    *LAST_PERSISTED_TIMEZONE.lock().expect("timezone recorder lock") = None;
}

#[cfg(test)]
fn last_persisted_timezone() -> Option<String> {
    LAST_PERSISTED_TIMEZONE.lock().expect("timezone recorder lock").clone()
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
