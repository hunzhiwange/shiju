//! 处理通用设置页签切换与时区持久化消息。

use crate::app::config::SystemSettingsSection;
use crate::app::{App, Message};
use desktop_settings_appearance::general_update as domain;
use iced::Task;

use super::{GeneralSettingsMessage, SettingsMessage};

/// 仅消费通用设置消息；其它设置分支保持不变并返回空任务。
pub(super) fn update(app: &mut App, message: SettingsMessage) -> Task<Message> {
    let SettingsMessage::General(message) = message else {
        return Task::none();
    };

    match message {
        GeneralSettingsMessage::LaunchAtLoginToggled(enabled) => {
            if !domain::begin_launch_at_login(
                &mut app.launch_at_login_busy,
                !cfg!(target_arch = "wasm32"),
            ) {
                return Task::none();
            }
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
            match domain::launch_at_login_saved(
                &mut app.launch_at_login,
                &mut app.launch_at_login_busy,
                result,
                true,
            ) {
                Ok(()) => {}
                Err(error) => {
                    // 系统注册或旧项清理可能已部分成功；回读期间禁止再次写入。
                    let toast = app.show_error_toast(format!("设置开机启动失败：{error}"));
                    let refresh = Task::perform(
                        async {
                            #[cfg(not(target_arch = "wasm32"))]
                            {
                                tokio::task::spawn_blocking(
                                    crate::app::desktop_lifecycle::read_launch_at_login,
                                )
                                .await
                                .map_err(|err| err.to_string())?
                            }
                            #[cfg(target_arch = "wasm32")]
                            {
                                crate::app::desktop_lifecycle::read_launch_at_login()
                            }
                        },
                        |result| {
                            Message::Settings(SettingsMessage::General(
                                GeneralSettingsMessage::LaunchAtLoginRefreshed(result),
                            ))
                        },
                    );
                    return Task::batch([toast, refresh]);
                }
            }
        }
        GeneralSettingsMessage::LaunchAtLoginRefreshed(result) => {
            if let Err(error) = domain::launch_at_login_saved(
                &mut app.launch_at_login,
                &mut app.launch_at_login_busy,
                result,
                false,
            ) {
                return app.show_error_toast(format!("读取开机启动状态失败：{error}"));
            }
        }
        #[cfg(target_os = "macos")]
        GeneralSettingsMessage::McpLinkPreview(id) => {
            return crate::app::mcp_link_preview::handoff(app, id);
        }
        #[cfg(target_os = "macos")]
        GeneralSettingsMessage::McpLinkRejected(id) => {
            if crate::app::deep_links::acknowledge(id).is_err() {
                return app.show_error_toast("无法拒绝 MCP 链接");
            }
        }
        #[cfg(target_os = "macos")]
        GeneralSettingsMessage::ProviderLinkRejected(id) => {
            if crate::app::deep_links::acknowledge(id).is_err() {
                return app.show_error_toast("无法拒绝供应商链接".to_string());
            }
        }
        #[cfg(target_os = "macos")]
        GeneralSettingsMessage::SkillLinkPreview(id) => {
            return crate::app::skill_link_preview::handoff(app, id);
        }
        #[cfg(target_os = "macos")]
        GeneralSettingsMessage::SkillLinkRejected(id) => {
            if crate::app::deep_links::acknowledge(id).is_err() {
                return app.show_error_toast("无法拒绝技能链接");
            }
        }
        #[cfg(target_os = "macos")]
        GeneralSettingsMessage::PromptLinkPreview(id) => {
            return crate::app::prompt_link_preview::handoff(app, id);
        }
        #[cfg(target_os = "macos")]
        GeneralSettingsMessage::PromptLinkRejected(id) => {
            if crate::app::deep_links::acknowledge(id).is_err() {
                return app.show_error_toast("无法取消提示词链接");
            }
        }
        GeneralSettingsMessage::CloseToTrayToggled(enabled) => {
            if enabled
                && let Err(error) = crate::app::desktop_lifecycle::ensure_tray(app.general_settings.desktop_locale)
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
        #[cfg(target_os = "macos")]
        GeneralSettingsMessage::InstanceTick => {
            if app.shutdown_requested {
                return Task::none();
            }
            let requests = crate::app::single_instance::poll();
            let mut unsupported = false;
            for args in &requests {
                match crate::app::deep_links::receive_arguments(args) {
                    Ok(other) => unsupported |= other,
                    Err(_) => return app.show_error_toast("无法暂存启动链接".to_string()),
                }
            }
            let links = crate::app::deep_links::take_notification();
            if requests.is_empty() && !links {
                return Task::none();
            }
            if crate::app::provider_link_preview::pending().is_some()
                || crate::app::mcp_link_preview::pending().is_some()
                || crate::app::prompt_link_preview::pending().is_some()
            {
                app.navigation_panels.show_settings = true;
            }
            let wake = app.reopen_main_window();
            if unsupported {
                return Task::batch([
                    wake,
                    app.show_error_toast("已唤醒主窗口；当前尚不支持启动参数".to_string()),
                ]);
            }
            return wake;
        }
        GeneralSettingsMessage::TrayProviderSwitch(id) => {
            return crate::app::desktop_lifecycle::tray_switch::start(app, id);
        }
        GeneralSettingsMessage::TrayProviderSwitched { scope, generation, id, result } => {
            return crate::app::desktop_lifecycle::tray_switch::finish(
                app, scope, generation, id, result,
            );
        }
        GeneralSettingsMessage::TrayTick => {
            #[cfg(not(target_arch = "wasm32"))]
            crate::app::desktop_lifecycle::refresh_tray_providers(app);
            #[cfg(not(target_arch = "wasm32"))]
            crate::app::desktop_lifecycle::refresh_tray_state(
                app.general_settings.desktop_locale,
                app.shutdown_requested,
                cfg!(target_os = "macos") && app.main_window_id.is_none(),
            );
            if app.shutdown_requested {
                return Task::none();
            }
            return Task::batch(
                crate::app::desktop_lifecycle::poll_tray().into_iter().map(|message| {
                    Task::done(Message::Settings(SettingsMessage::General(message)))
                }),
            );
        }
        GeneralSettingsMessage::TrayOpenApp(id) => {
            if app.shutdown_requested {
                return Task::none();
            }
            app.navigation_panels.show_system_settings = true;
            app.navigation_panels.system_settings_help_tab = None;
            app.navigation_panels.active_menu = None;
            app.apps_catalog.query.clear();
            let page = super::update(
                app,
                SettingsMessage::SystemTabSelected(
                    crate::app::components::system_settings::SystemTab::Apps,
                ),
            );
            app.apps_catalog.pending_tray_app = Some(id);
            // Catalog connection/loading may reset the expanded row. Resolve after loading.
            let details = if !app.apps_catalog.loading
                && app.apps_catalog.rows.iter().any(|row| row.entry.id == id)
            {
                app.apps_catalog.pending_tray_app = None;
                if app.apps_catalog.expanded != Some(id) {
                    super::apps_catalog::update(app, super::AppsCatalogMessage::ToggleDetails(id))
                } else {
                    Task::none()
                }
            } else {
                Task::none()
            };
            return Task::batch([app.reopen_main_window(), page, details]);
        }
        GeneralSettingsMessage::TrayShow => {
            return app.reopen_main_window();
        }
        GeneralSettingsMessage::TrayQuit => {
            #[cfg(not(target_arch = "wasm32"))]
            {
                let window_id = app.main_window_id;
                return crate::app::message::view::panel::shutdown(app, window_id);
            }
        }
        GeneralSettingsMessage::LocaleSelected(locale) => {
            let save = domain::select_locale(
                &mut app.general_settings.desktop_locale,
                &mut app.general_settings.desktop_locale_selected,
                &mut app.general_settings.desktop_locale_saving,
                locale,
            );
            #[cfg(not(target_arch = "wasm32"))]
            crate::app::desktop_lifecycle::refresh_tray_locale(locale);
            crate::app::i18n::editor::refresh(app);
            crate::apps::workflow::refresh_top_tab_title(app);
            crate::app::i18n::apps::refresh_hub_titles(app);
            let viewing_task = app.task_board.task_board_viewing_logs.clone();
            crate::app::message::task_board::sync_task_logs_editor(app, viewing_task.as_ref());
            let chunks = app.chat_view.active_session_view_state
                .prepared_chat_ui_chunks
                .iter()
                .copied()
                .collect::<Vec<_>>();
            for chunk in chunks {
                app.invalidate_chat_ui_chunk(chunk);
            }
            app.chat_interaction.chat_explore_summary_animations.clear();
            super::cron::refresh_run_history_locale(app);
            if save {
                return save_locale(locale);
            }
        }
        GeneralSettingsMessage::LocaleSaved(locale, result) => {
            let save_again =
                domain::locale_saved(app.general_settings.desktop_locale, &mut app.general_settings.desktop_locale_saving, locale);
            let feedback = match result {
                Ok(()) => Task::none(),
                Err(error) => app.show_error_toast(
                    crate::__!(
                        crate::app::i18n::I18n::new(app.general_settings.desktop_locale),
                        "保存失败：%s",
                        error
                    )
                    .expect("embedded save failure template has one string argument"),
                ),
            };
            if save_again {
                return Task::batch([feedback, save_locale(app.general_settings.desktop_locale)]);
            }
            return feedback;
        }
        GeneralSettingsMessage::TabSelected(tab) => {
            if domain::select_tab(
                &mut app.general_settings.general_settings_tab,
                &mut app.themes_settings.error,
                tab,
            ) {
                return Task::done(Message::Settings(SettingsMessage::Themes(
                    super::ThemesMessage::Refresh,
                )));
            }
        }
        GeneralSettingsMessage::TaskCompletionNotificationToggled(enabled) => {
            if !crate::app::desktop_notification::SUPPORTED {
                return Task::none();
            }
            let persist = domain::select_notification(
                &mut app.general_settings.task_completion_notification,
                &mut app.general_settings.task_completion_notification_saving,
                enabled,
            );
            #[cfg(not(target_arch = "wasm32"))]
            app.note_startup_setting_edit("task_completion_notification");
            let save =
                if persist { save_task_completion_notification(enabled) } else { Task::none() };
            let permission = if enabled {
                crate::app::desktop_notification::permission_task(app.general_settings.desktop_locale)
            } else {
                Task::none()
            };
            return Task::batch([save, permission]);
        }
        GeneralSettingsMessage::TaskCompletionNotificationPermissionChecked(result) => {
            if let Err(error) = result {
                let feedback = crate::app::desktop_notification::failure_feedback(app, error);
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
            let persist = domain::notification_saved(
                app.general_settings.task_completion_notification,
                &mut app.general_settings.task_completion_notification_saving,
                enabled,
            );
            let feedback = match result {
                Ok(()) => Task::none(),
                Err(_) => app.show_error_toast(
                    crate::app::i18n::I18n::new(app.general_settings.desktop_locale)
                        .__("保存通知设置失败，请重试。")
                        .to_owned(),
                ),
            };
            if persist {
                return Task::batch([
                    feedback,
                    save_task_completion_notification(app.general_settings.task_completion_notification),
                ]);
            }
            return feedback;
        }
        GeneralSettingsMessage::TestTaskCompletionNotification => {
            return crate::app::desktop_notification::completion_task(app.general_settings.desktop_locale);
        }
        GeneralSettingsMessage::TaskCompletionNotificationSent(result) => {
            if let Err(error) = result {
                let i18n = crate::app::i18n::I18n::new(app.general_settings.desktop_locale);
                return crate::app::desktop_notification::failure_feedback(
                    app,
                    format!("{} {error}", i18n.__("系统通知发送失败，请检查系统通知权限。"),),
                );
            }
        }
        GeneralSettingsMessage::TimezoneSelected(timezone) => {
            if !domain::select_timezone(&mut app.general_settings.chat_timezone, &timezone) {
                return Task::none();
            }

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

fn save_locale(locale: config_ui::DesktopLocale) -> Task<Message> {
    Task::perform(
        desktop_settings_appearance::general_update::save_locale(config_access(), locale),
        move |result| {
            Message::Settings(SettingsMessage::General(GeneralSettingsMessage::LocaleSaved(
                locale, result,
            )))
        },
    )
}

fn save_task_completion_notification(enabled: bool) -> Task<Message> {
    Task::perform(
        desktop_settings_appearance::general_update::save_task_completion_notification(
            config_access(),
            enabled,
        ),
        move |result| {
            Message::Settings(SettingsMessage::General(
                GeneralSettingsMessage::TaskCompletionNotificationSaved(enabled, result),
            ))
        },
    )
}

pub(super) fn config_access() -> desktop_config::ConfigAccess {
    crate::app::config::config_access()
}
