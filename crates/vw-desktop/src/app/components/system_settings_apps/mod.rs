pub(crate) mod catalog;
mod details;
pub(crate) mod drag;
pub(crate) mod lifecycle;
mod list;
pub(crate) mod local;
mod presentation;
pub(crate) mod uninstall;

use crate::app::{
    Message,
    i18n::I18n,
    message::{SettingsMessage, settings::AppsCatalogMessage},
    state::AppsCatalogState,
};
use iced::{Element, Theme};

fn message(value: AppsCatalogMessage) -> Message {
    Message::Settings(SettingsMessage::AppsCatalog(value))
}

pub(crate) use list::view;

pub(crate) fn view_overlays<'a>(
    state: &'a AppsCatalogState,
    i18n: I18n,
    theme: &Theme,
    base: Element<'a, Message>,
) -> Element<'a, Message> {
    let base = crate::app::components::app_routing::requests::overlay(state, base);
    let base = crate::app::components::app_routing::config::overlay(state, base);
    let base = crate::app::components::openclaw_catalog::overlay(&state.openclaw_catalog, base);
    let base = crate::app::components::workspace_files::overlay(&state.workspace_files, &state.host, base);
    let base = crate::app::components::daily_memory::overlay(&state.daily_memory, base);
    let base = crate::app::components::openclaw_tools::overlay(&state.openclaw_tools, base);
    let base = crate::app::components::openclaw_env::overlay(&state.openclaw_env, base);
    let base = crate::app::components::openclaw_model::overlay(&state.openclaw_model, base);
    let base = crate::app::components::openclaw_defaults::overlay(&state.openclaw_defaults, base);
    let base = crate::app::components::system_settings_prompts::templates::overlay(&state.prompts.templates, base);
    let base = crate::app::components::system_settings_prompts::pi_files::overlay(&state.prompts.pi_files, base);
    let base = crate::app::components::agent_apps_high_availability_policy::overlay(&state.high_availability, base);
    if !state.connection_open {
        return launch::overlay(
            state,
            i18n,
            theme,
            preferences::overlay(
                state,
                i18n,
                theme,
                lifecycle::overlay(&state.lifecycle, &state.host, base),
            ),
        );
    }
    desktop_settings_apps::system_settings_apps::connection::overlay(&state.host, &state.token, base, |event| {
        use desktop_settings_apps::system_settings_apps::connection::ConnectionMessage;
        message(match event {
            ConnectionMessage::Close => AppsCatalogMessage::ConnectionClose,
            ConnectionMessage::HostInput(value) => AppsCatalogMessage::HostInput(value),
            ConnectionMessage::TokenInput(value) => AppsCatalogMessage::TokenInput(value),
            ConnectionMessage::Refresh => AppsCatalogMessage::Refresh,
        })
    })
}


pub(crate) mod preferences;

pub(crate) mod launch;

pub(super) fn view_state(state: &AppsCatalogState) -> desktop_settings_apps::system_settings_apps::ViewState<'_> {
    desktop_settings_apps::system_settings_apps::ViewState {
        rows: &state.rows, preferences: &state.preferences, lifecycle: &state.lifecycle, drag: &state.drag,
        query: &state.query, host: &state.host, loading: state.loading, error: &state.error,
        expanded: state.expanded, hovered: state.hovered,
    }
}

pub(super) fn domain_event(action: desktop_settings_apps::system_settings_apps::Action) -> Message {
    use desktop_settings_apps::system_settings_apps::Action;
    use crate::app::message::settings::apps_lifecycle::LifecycleMessage;
    let value = match action {
        Action::Refresh => AppsCatalogMessage::Refresh,
        Action::QueryInput(value) => AppsCatalogMessage::QueryInput(value),
        Action::ConnectionOpen => AppsCatalogMessage::ConnectionOpen,
        Action::LocalConnect => AppsCatalogMessage::LocalConnect,
        Action::ToggleDetails(id) => AppsCatalogMessage::ToggleDetails(id),
        Action::RefreshObservation(id) => AppsCatalogMessage::RefreshObservation(id),
        Action::PreferencesOpen => AppsCatalogMessage::Preferences(preferences::PreferencesMessage::Open),
        Action::LaunchOpen(id, installation) => AppsCatalogMessage::Launch(launch::LaunchMessage::Open(id, installation)),
        Action::Preview(request) => AppsCatalogMessage::Lifecycle(LifecycleMessage::Preview(request)),
        Action::LifecycleClose => AppsCatalogMessage::Lifecycle(LifecycleMessage::Close),
        Action::LifecycleConfirm => AppsCatalogMessage::Lifecycle(LifecycleMessage::Confirm),
        Action::LifecycleRefresh => AppsCatalogMessage::Lifecycle(LifecycleMessage::Refresh),
        Action::LifecycleCancel => AppsCatalogMessage::Lifecycle(LifecycleMessage::Cancel),
        Action::Drag(value) => AppsCatalogMessage::Drag(value),
        Action::Providers(id) => return crate::app::message::app_providers::list::message(crate::app::message::app_providers::list::ListMessage::Open(id)),
        Action::Prompts(id) => AppsCatalogMessage::Prompts(crate::app::message::settings::prompts::Event::Open(id)),
        Action::Download => return Message::View(crate::app::message::ViewMessage::OpenUrlExternal(desktop_settings_apps::system_settings_apps::uninstall::MANUAL_DOWNLOAD_URL.into())),
    };
    message(value)
}
