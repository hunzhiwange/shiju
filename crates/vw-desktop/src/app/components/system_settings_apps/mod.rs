pub(crate) mod catalog;
#[cfg(test)]
mod catalog_tests;
mod details;
#[cfg(test)]
mod details_tests;
pub(crate) mod drag;
#[cfg(test)]
mod drag_tests;
pub(crate) mod lifecycle;
#[cfg(test)]
mod lifecycle_tests;
mod list;
pub(crate) mod local;
mod presentation;
pub(crate) mod uninstall;

use super::system_settings_common::*;
use crate::app::{
    Message,
    i18n::I18n,
    message::{SettingsMessage, settings::AppsCatalogMessage},
    state::AppsCatalogState,
};
use iced::widget::{button, column, container, row, text};
use iced::{Element, Length, Theme};

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
    let close = message(AppsCatalogMessage::ConnectionClose);
    let host = settings_text_input("管理员登记的主机 ID", &state.host)
        .on_input(|value| message(AppsCatalogMessage::HostInput(value)));
    let token = settings_text_input("应用管理令牌", &state.token)
        .secure(true)
        .on_input(|value| message(AppsCatalogMessage::TokenInput(value)));
    let connect = button(text("加载应用").size(SETTINGS_CONTROL_TEXT_SIZE))
        .padding([8, 16])
        .style(primary_action_btn_style)
        .on_press_maybe(
            (!state.host.trim().is_empty() && !state.token.is_empty())
                .then_some(message(AppsCatalogMessage::Refresh)),
        );
    let cancel = button(text("取消").size(SETTINGS_CONTROL_TEXT_SIZE))
        .padding([8, 16])
        .style(rounded_action_btn_style)
        .on_press(close.clone());
    let header = settings_modal_header(
        settings_page_heading(
            "选择执行主机",
            "主机 ID 与令牌由网关管理员提供；令牌仅在本次会话内存中保留。",
        ),
        settings_close_button(close.clone()),
    );
    let fields = column![
        column![settings_field_label("执行主机"), host].spacing(6),
        column![settings_field_label("应用管理令牌"), token].spacing(6),
    ]
    .spacing(14);
    let footer = row![iced::widget::Space::new().width(Length::Fill), cancel, connect]
        .spacing(10)
        .align_y(iced::Alignment::Center);
    let card = settings_modal_layout(header, fields, Some(footer.into()))
        .width(Length::Fill)
        .max_width(520.0)
        .max_height(342.0);
    let card_with_gutter = container(card).padding(12).center(Length::Fill);
    settings_modal_overlay(Some(base), close, card_with_gutter)
}

#[cfg(test)]
mod uninstall_tests;

pub(crate) mod preferences;
#[cfg(test)]
pub(crate) mod preferences_tests;

pub(crate) mod launch;
#[cfg(test)]
mod launch_tests;

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
        Action::Download => return Message::View(crate::app::message::ViewMessage::OpenUrlExternal("https://claude.com/download".into())),
    };
    message(value)
}
