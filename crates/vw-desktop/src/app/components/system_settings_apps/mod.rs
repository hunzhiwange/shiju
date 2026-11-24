pub(crate) mod catalog;
pub(crate) mod commands;
pub(super) mod details;
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

pub(super) fn message(value: AppsCatalogMessage) -> Message {
    Message::Settings(SettingsMessage::AppsCatalog(value))
}

pub(crate) use list::view;

pub(crate) fn view_overlays<'a>(
    state: &'a AppsCatalogState,
    i18n: I18n,
    theme: &Theme,
    base: Element<'a, Message>,
) -> Element<'a, Message> {
    let base = iced::widget::stack![base,
        desktop_apps_ui::app_routing::outbound::overlay(i18n, &state.routing, iced::widget::Space::new().into())
            .map(|event| message(AppsCatalogMessage::Routing(event)))
    ].into();
    let base = crate::app::components::app_routing::requests::overlay(i18n, state, base);
    let base = crate::app::components::app_routing::config::overlay(i18n, state, base);
    let base = crate::app::components::openclaw_catalog::overlay(i18n, &state.openclaw_catalog, base);
    let base = crate::app::components::workspace_files::overlay(i18n, &state.workspace_files, &state.host, base);
    let base = crate::app::components::daily_memory::overlay(i18n, &state.daily_memory, base);
    let base = crate::app::components::openclaw_tools::overlay(i18n, &state.openclaw_tools, base);
    let base = crate::app::components::openclaw_env::overlay(i18n, &state.openclaw_env, base);
    let base = crate::app::components::openclaw_model::overlay(i18n, &state.openclaw_model, base);
    let base = crate::app::components::openclaw_defaults::overlay(i18n, &state.openclaw_defaults, base);
    let base = crate::app::components::system_settings_prompts::templates::overlay(
        i18n,
        &state.prompts.templates,
        base,
    );
    let base = crate::app::components::system_settings_prompts::pi_files::overlay(
        i18n,
        &state.prompts.pi_files,
        base,
    );
    use desktop_ui_controls::settings_navigation::{ThirdPartyPage, ThirdPartySettingsTab};
    let base = if state.third_party_page == ThirdPartyPage::Settings(ThirdPartySettingsTab::Routing)
    {
        base
    } else {
        crate::app::components::agent_apps_high_availability_policy::overlay(i18n,
            &state.high_availability,
            base,
        )
    };
    let base = commands::overlay(i18n, state, base);
    let base = launch::overlay(
        state, i18n, theme,
        preferences::overlay(state, i18n, theme,
            lifecycle::overlay(i18n, &state.lifecycle, &state.host, base)),
    );
    crate::app::components::app_routing::lifecycle::restore_overlay(i18n, state, base)
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
        Action::CommandsOpen(panel) => AppsCatalogMessage::Commands(commands::Event::Open(panel)),
        Action::BatchUpgrade => AppsCatalogMessage::BatchUpgrade,
        Action::CheckUpdates => AppsCatalogMessage::CheckUpdates,
        Action::Refresh => AppsCatalogMessage::Refresh,
        Action::QueryInput(value) => AppsCatalogMessage::QueryInput(value),
        Action::ConnectionOpen => AppsCatalogMessage::ConnectionOpen,
        Action::LocalConnect => AppsCatalogMessage::LocalConnect,
        Action::ToggleDetails(id) => AppsCatalogMessage::ToggleDetails(id),
        Action::RefreshObservation(id) => AppsCatalogMessage::RefreshObservation(id),
        Action::SetVisible(id, visible) => AppsCatalogMessage::Preferences(preferences::PreferencesMessage::SetVisible(id, visible)),
        Action::PreferencesOpen => AppsCatalogMessage::Preferences(preferences::PreferencesMessage::Open),
        Action::LaunchOpen(id, installation) => AppsCatalogMessage::Launch(launch::LaunchMessage::Open(id, installation)),
        Action::Preview(request) => AppsCatalogMessage::Lifecycle(LifecycleMessage::Preview(request)),
        Action::LifecycleClose => AppsCatalogMessage::Lifecycle(LifecycleMessage::Close),
        Action::LifecycleConfirm => AppsCatalogMessage::Lifecycle(LifecycleMessage::Confirm),
        Action::LifecycleRefresh => AppsCatalogMessage::Lifecycle(LifecycleMessage::Refresh),
        Action::LifecycleCancel => AppsCatalogMessage::Lifecycle(LifecycleMessage::Cancel),
        Action::Drag(value) => AppsCatalogMessage::Drag(value),
        Action::Providers(id) => return crate::app::message::app_providers::list::message(crate::app::message::app_providers::list::ListMessage::Open(id)),
        Action::Prompts(id) => AppsCatalogMessage::SelectPromptApp(id),
        Action::Download => return Message::View(crate::app::message::ViewMessage::OpenUrlExternal(desktop_settings_apps::system_settings_apps::uninstall::MANUAL_DOWNLOAD_URL.into())),
    };
    message(value)
}
