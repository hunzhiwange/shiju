use super::catalog;
use crate::app::{Message, state::AppsCatalogState};
use desktop_ui_controls::system_settings_common::*;
use iced::widget::{button, column, text};
pub(crate) fn view<'a>(state: &'a AppsCatalogState, app: &'a catalog::CatalogRow) -> iced::Element<'a, Message> {
    let mut cross = column![].spacing(10);
    if let Some(scope) = vw_api_project_profiles::ProjectProfileScope::for_app(app.entry.id) {
        cross = cross.push(crate::app::components::project_profile_selection::view(&state.project_profiles, scope));
    }
    cross = cross.push(crate::app::components::app_routing::config::timeout_button(app.entry.id));
    cross = cross.push(crate::app::components::app_routing::outbound::entry(app.entry.id));
    cross = cross.push(crate::app::components::app_routing::requests::entry(app.entry.id));
    if app.entry.id == vw_api_app_identity::AppId::OpenClaw {
        cross = cross.push(crate::app::components::openclaw_model::entry());
        cross = cross.push(crate::app::components::workspace_files::entry());
        cross = cross.push(crate::app::components::daily_memory::entry());
        cross = cross.push(crate::app::components::openclaw_tools::entry());
        cross = cross.push(crate::app::components::openclaw_env::entry());
        cross = cross.push(crate::app::components::openclaw_defaults::entry());
        cross = cross.push(crate::app::components::openclaw_catalog::entry());
    }
    cross = cross.push(crate::app::components::system_settings_prompts::list::view(&state.prompts, app.entry.id));
    if app.entry.id == vw_api_app_identity::AppId::Hermes {
        cross = cross.push(crate::app::components::hermes_memory::view(&state.hermes_memory));
        cross = cross.push(crate::app::components::hermes_dashboard::view(&state.hermes_dashboard));
    }
    cross = cross.push(button(text("高可用策略")).padding([8, 12]).style(rounded_action_btn_style)
        .on_press(crate::app::message::agent_apps_high_availability_policy::message(
            crate::app::message::agent_apps_high_availability_policy::Event::Open(app.entry.id))));
    desktop_settings_apps::system_settings_apps::details::view(&super::view_state(state), app, cross.into(), super::uninstall::MANUAL_GUIDANCE, super::domain_event)
}
