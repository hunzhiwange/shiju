//! Plugin dialog requests carry generation IDs; stale replies only discard their staging token.
use crate::app::{
    App, Message,
    message::SettingsMessage,
    state::{PluginInstallForm, PluginInstallPhase},
};
use iced::Task;
fn discard(token: String) -> Task<Message> {
    Task::perform(crate::app::session_gateway::gateway_plugin_discard_async(token), |result| {
        Message::Settings(SettingsMessage::PluginsPreviewDiscarded(result))
    })
}
fn next_id(app: &mut App) -> u64 {
    app.skills_settings.plugin_operation_id =
        app.skills_settings.plugin_operation_id.wrapping_add(1);
    app.skills_settings.plugin_operation_id
}
pub(super) fn update(app: &mut App, message: SettingsMessage) -> Task<Message> {
    match message {
        SettingsMessage::PluginsInstallOpen | SettingsMessage::PluginsUpdateOpen(_) => {
            if app.skills_settings.plugin_install.is_some() {
                return Task::none();
            }
            let update = match message {
                SettingsMessage::PluginsUpdateOpen(id) => Some(id),
                _ => None,
            };
            let id = next_id(app);
            app.skills_settings.plugin_install = Some(PluginInstallForm::new(id, update));
            Task::none()
        }
        SettingsMessage::PluginsInstallSourceChanged(value) => {
            let Some(form) = app.skills_settings.plugin_install.as_mut() else {
                return Task::none();
            };
            if form.busy() {
                return Task::none();
            }
            let token = form.preview().map(|p| p.token.clone());
            form.source = value;
            form.phase = PluginInstallPhase::Input;
            token.map(discard).unwrap_or_else(Task::none)
        }
        SettingsMessage::PluginsInstallPrepare => {
            if app.skills_settings.plugin_install.as_ref().is_none_or(|form| form.busy()) {
                return Task::none();
            }
            let id = next_id(app);
            let form = app.skills_settings.plugin_install.as_mut().unwrap();
            let old = form.preview().map(|p| p.token.clone());
            form.request_id = id;
            if let Err(error) = form.begin_prepare() {
                form.phase = PluginInstallPhase::Failed(error.clone());
                return app.show_localized_toast(
                    crate::app::state::ToastKind::Error,
                    &error,
                    vec![],
                );
            }
            let source = form.source.clone();
            let update = form.update_id.clone();
            let prepare = Task::perform(
                crate::app::session_gateway::gateway_plugin_prepare_async(source, update),
                move |result| {
                    Message::Settings(SettingsMessage::PluginsInstallPrepared {
                        request_id: id,
                        result,
                    })
                },
            );
            Task::batch([prepare, old.map(discard).unwrap_or_else(Task::none)])
        }
        SettingsMessage::PluginsInstallPrepared { request_id, result } => {
            let failure = result.as_ref().err().cloned();
            let stale = match app.skills_settings.plugin_install.as_mut() {
                Some(form) => form.receive_preview(request_id, result),
                None => result.ok(),
            };
            if let Some(stale) = stale {
                return discard(stale.token);
            }
            if let Some(error) = failure {
                if app
                    .skills_settings
                    .plugin_install
                    .as_ref()
                    .is_some_and(|f| f.request_id == request_id)
                {
                    return app.show_error_toast(error);
                }
            }
            if app.skills_settings.plugin_install.as_ref().is_some_and(|form| {
                form.request_id == request_id && matches!(form.phase, PluginInstallPhase::Review(_))
            }) {
                return app.show_localized_toast(
                    crate::app::state::ToastKind::Info,
                    "审计预览已就绪；请核对来源与声明后确认",
                    vec![],
                );
            }
            Task::none()
        }
        SettingsMessage::PluginsInstallConfirm => {
            let Some(form) = app.skills_settings.plugin_install.as_mut() else {
                return Task::none();
            };
            let Some(preview) = form.confirm() else {
                return Task::none();
            };
            let id = form.request_id;
            let update = form.update_id.is_some();
            Task::perform(
                crate::app::session_gateway::gateway_plugin_activate_async(preview.token, update),
                move |result| {
                    Message::Settings(SettingsMessage::PluginsInstallCompleted {
                        request_id: id,
                        result,
                    })
                },
            )
        }
        SettingsMessage::PluginsInstallCompleted { request_id, result } => {
            if !app
                .skills_settings
                .plugin_install
                .as_ref()
                .is_some_and(|f| f.request_id == request_id)
            {
                return Task::none();
            }
            match result {
                Ok(_) => {
                    app.skills_settings.plugin_install = None;
                    let toast = app.show_localized_toast(
                        crate::app::state::ToastKind::Success,
                        "插件操作已完成；未自动运行引导技能或钩子",
                        vec![],
                    );
                    let refresh = super::skills::update(app, SettingsMessage::PluginsRefresh);
                    Task::batch([toast, refresh])
                }
                Err(error) => {
                    app.skills_settings.plugin_install.as_mut().unwrap().phase =
                        PluginInstallPhase::Failed(error.clone());
                    app.show_error_toast(error)
                }
            }
        }
        SettingsMessage::PluginsInstallClose => {
            if app
                .skills_settings
                .plugin_install
                .as_ref()
                .is_some_and(|f| matches!(f.phase, PluginInstallPhase::Committing(_)))
            {
                return app.show_localized_toast(
                    crate::app::state::ToastKind::Info,
                    "正在提交插件操作，完成后关闭",
                    vec![],
                );
            }
            let token = app
                .skills_settings
                .plugin_install
                .take()
                .and_then(|f| f.preview().map(|p| p.token.clone()));
            next_id(app);
            Task::batch([
                app.show_localized_toast(
                    crate::app::state::ToastKind::Info,
                    "已取消插件操作",
                    vec![],
                ),
                token.map(discard).unwrap_or_else(Task::none),
            ])
        }
        SettingsMessage::PluginsPreviewDiscarded(result) => match result {
            Ok(()) => Task::none(),
            Err(_) => app.show_localized_toast(
                crate::app::state::ToastKind::Warning,
                "预览已关闭；暂存清理未确认，服务端将在过期后清理",
                vec![],
            ),
        },
        _ => Task::none(),
    }
}
