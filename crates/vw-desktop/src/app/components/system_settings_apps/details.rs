use super::catalog;
use crate::app::{Message, state::AppsCatalogState};
use desktop_ui_controls::system_settings_common::*;
use iced::widget::{button, column, row, text};
pub(crate) fn view<'a>(state: &'a AppsCatalogState, app: &'a catalog::CatalogRow) -> iced::Element<'a, Message> {
    let id = app.entry.id;
    let routing = row![
        crate::app::components::app_routing::config::timeout_button(id),
        crate::app::components::app_routing::outbound::entry(id),
        crate::app::components::app_routing::requests::entry(id),
        button(text("高可用策略").size(SETTINGS_CONTROL_TEXT_SIZE))
            .padding([8, 12])
            .style(rounded_action_btn_style)
            .on_press(crate::app::message::agent_apps_high_availability_policy::message(
                crate::app::message::agent_apps_high_availability_policy::Event::Open(id),
            )),
    ]
    .spacing(8);
    let mut cross = column![].spacing(12);
    if let Some(scope) = vw_api_project_profiles::ProjectProfileScope::for_app(id) {
        cross = cross.push(settings_panel(
            crate::app::components::project_profile_selection::view(&state.project_profiles, scope),
        ));
    }
    cross = cross
        .push(settings_panel(column![text("路由与诊断").size(16), routing.wrap()].spacing(10)));
    let mut tools = row![
        button(text("管理提示词").size(SETTINGS_CONTROL_TEXT_SIZE))
            .padding([8, 12])
            .style(rounded_action_btn_style)
            .on_press(super::message(
                crate::app::message::settings::AppsCatalogMessage::SelectPromptApp(id)
            )),
    ]
    .spacing(8);
    if id == vw_api_app_identity::AppId::OpenClaw {
        tools = tools
            .push(crate::app::components::openclaw_model::entry())
            .push(crate::app::components::workspace_files::entry())
            .push(crate::app::components::daily_memory::entry())
            .push(crate::app::components::openclaw_tools::entry())
            .push(crate::app::components::openclaw_env::entry())
            .push(crate::app::components::openclaw_defaults::entry())
            .push(crate::app::components::openclaw_catalog::entry());
    }
    let mut app_tools = column![text("应用工具").size(16), tools.wrap()].spacing(10);
    if id == vw_api_app_identity::AppId::Hermes {
        app_tools = app_tools
            .push(crate::app::components::hermes_memory::view(&state.hermes_memory))
            .push(crate::app::components::hermes_dashboard::view(&state.hermes_dashboard));
    }
    cross = cross.push(settings_panel(app_tools));
    desktop_settings_apps::system_settings_apps::details::view(&super::view_state(state), app, cross.into(), super::uninstall::MANUAL_GUIDANCE, super::domain_event)
}
