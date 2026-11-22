//! Desktop projection of resources owned by other domains.
use crate::app::{App, Message};
use desktop_settings_agents as domain;
use iced::Element;
pub(in crate::app::components) fn overlay<'a>(
    app: &'a App,
    base: Element<'a, Message>,
) -> Element<'a, Message> {
    let state = &app.agents_settings;
    let workspace_metadata = state
        .workspace_identity_files
        .iter()
        .find(|file| file.file_name == state.active_prompt_tab)
        .and_then(|file| {
            super::identity::workspace_identity_meta_line(
                app.general_settings.desktop_locale,
                file.size_bytes,
                file.modified_at_ms,
            )
        });
    let card = domain::profile_modal::page_card(domain::profile_modal::PageInput {
        state,
        locale: app.general_settings.desktop_locale,
        theme: &app.appearance.app_theme,
        help: super::domain::help,
        acp_agents: &app.session_state.acp_agents,
        acp_loading: app.acp_settings.loading,
        acp_error: app.acp_settings.save_error.as_ref(),
        inspect: app.mcp_settings.inspect.as_ref(),
        inspect_loading: app.mcp_settings.inspect_loading,
        inspect_error: app.mcp_settings.inspect_error.as_ref(),
        workspace_metadata,
        project_options: super::agent_project_options(app),
    });
    crate::app::components::system_settings_common::settings_optional_modal_overlay(
        base,
        card.map(|card| {
            (
                super::domain::event(domain::message::Event::Agents(
                    domain::message::AgentsMessage::ProfileCancel,
                )),
                card.map(super::domain::event),
            )
        }),
    )
}
