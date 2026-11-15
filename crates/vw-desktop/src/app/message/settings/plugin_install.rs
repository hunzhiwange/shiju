use crate::app::{App, Message, message::SettingsMessage as S};
use desktop_settings_skills::plugin_install::{Event as E, State, ToastKind};
use iced::Task;
pub(crate) fn message(event: E) -> Message { match event {

        other => Message::Settings(S::PluginInstallEffect(other)),
    }
}
pub(super) fn update(app: &mut App, message_in: S) -> Task<Message> {
    let event = match message_in {
        S::PluginsInstallOpen => E::PluginsInstallOpen,
        S::PluginsUpdateOpen(v) => E::PluginsUpdateOpen(v),
        S::PluginsInstallSourceChanged(v) => E::PluginsInstallSourceChanged(v),
        S::PluginsInstallPrepare => E::PluginsInstallPrepare,
        S::PluginsInstallPrepared{ request_id, result } => E::PluginsInstallPrepared{ request_id, result },
        S::PluginsInstallConfirm => E::PluginsInstallConfirm,
        S::PluginsInstallCompleted{ request_id, result } => E::PluginsInstallCompleted{ request_id, result },
        S::PluginsInstallClose => E::PluginsInstallClose,
        S::PluginsPreviewDiscarded(v) => E::PluginsPreviewDiscarded(v),

        S::PluginInstallEffect(e) => e,
        _ => return Task::none(),
    };
    match event {
        E::Prepare(id, source, update) => Task::perform(crate::app::session_gateway::gateway_plugin_prepare_async(source, update), move |result| message(E::PluginsInstallPrepared { request_id: id, result })),
        E::Activate(id, token, update) => Task::perform(crate::app::session_gateway::gateway_plugin_activate_async(token, update), move |result| message(E::PluginsInstallCompleted { request_id: id, result })),
        E::Discard(token) => Task::perform(crate::app::session_gateway::gateway_plugin_discard_async(token), |r| message(E::PluginsPreviewDiscarded(r))),
        E::Refresh => super::skills::update(app, S::PluginsRefresh),
        E::Toast(kind, text) => app.show_localized_toast(match kind { ToastKind::Info => crate::app::state::ToastKind::Info, ToastKind::Success => crate::app::state::ToastKind::Success, ToastKind::Warning => crate::app::state::ToastKind::Warning, ToastKind::Error => crate::app::state::ToastKind::Error }, &text, vec![]),
        event => desktop_settings_skills::plugin_install::update(&mut State { plugin_operation_id: &mut app.skills_settings.plugin_operation_id, plugin_install: &mut app.skills_settings.plugin_install }, event).map(message),
    }
}
