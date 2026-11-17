//! Borrowed input assembly for page rendering assertions.
use super::profile_update::{ProfileFixture, defaults_input};
use crate::{message::Event, state::DelegateAgentSettingsEntry};
use iced::Element;
pub(super) fn profile_input(app: &ProfileFixture) -> crate::basic_profile::Input<'_> {
    crate::basic_profile::Input {
        locale: app.desktop_locale,
        entries: &app.agents_settings.entries,
        acp_agents: &app.acp_agents,
        saving: app.agents_settings.profile_saving,
        acp_loading: app.acp_settings.loading,
        acp_error: app.acp_settings.save_error.as_ref(),
    }
}
pub(super) fn basic<'a>(
    app: &'a ProfileFixture,
    draft: &'a DelegateAgentSettingsEntry,
    providers: Vec<String>,
) -> Element<'a, Event> {
    crate::basic::view(
        crate::basic::Input {
            help: super::selectable_help,
            locale: app.desktop_locale,
            theme: &app.app_theme,
            profile: profile_input(app),
            defaults: defaults_input(app, draft),
        },
        draft,
        providers,
    )
}
pub(super) fn identity<'a>(
    app: &'a ProfileFixture,
    draft: &'a DelegateAgentSettingsEntry,
) -> Element<'a, Event> {
    let state = &app.agents_settings;
    crate::identity::view(
        crate::identity::Input {
            help: super::selectable_help,
            locale: app.desktop_locale,
            theme: &app.app_theme,
            active_prompt_tab: &state.active_prompt_tab,
            workspace_identity_files: &state.workspace_identity_files,
            workspace_identity_root_path: state.workspace_identity_root_path.as_deref(),
            selected_agent: &state.selected_agent,
            workspace_metadata: None,
        },
        draft,
    )
}
pub(super) fn defaults<'a>(
    app: &'a ProfileFixture,
    draft: &'a DelegateAgentSettingsEntry,
) -> Element<'a, Event> {
    crate::defaults::view(defaults_input(app, draft), draft)
}
pub(super) fn tools<'a>(
    app: &'a ProfileFixture,
    draft: &'a DelegateAgentSettingsEntry,
) -> Element<'a, Event> {
    crate::tools::view(
        crate::tools::Input {
            help: super::selectable_help,
            locale: app.desktop_locale,
            theme: &app.app_theme,
            available_tools: &app.agents_settings.available_tools,
            active_tools_list_tab: &app.agents_settings.active_tools_list_tab,
        },
        draft,
    )
}
pub(super) fn skills<'a>(
    app: &'a ProfileFixture,
    draft: &'a DelegateAgentSettingsEntry,
) -> Element<'a, Event> {
    let state = &app.agents_settings;
    crate::skills::view(
        crate::skills::Input {
            help: super::selectable_help,
            locale: app.desktop_locale,
            theme: &app.app_theme,
            project_options: crate::agents_view::agent_project_options(
                app.project_path.as_deref(),
                &app.recent_projects,
            ),
            skill_directory: &state.skill_directory,
            skill_directory_scope: state.skill_directory_scope,
            skill_catalog: &state.skill_catalog,
            skill_catalog_loading: state.skill_catalog_loading,
            skill_catalog_error: &state.skill_catalog_error,
        },
        draft,
    )
}
pub(super) fn mcp<'a>(
    app: &'a ProfileFixture,
    draft: &'a DelegateAgentSettingsEntry,
) -> Element<'a, Event> {
    crate::mcp::view(
        crate::mcp::Input {
            help: super::selectable_help,
            locale: app.desktop_locale,
            theme: &app.app_theme,
            inspect: app.mcp_settings.inspect.as_ref(),
            inspect_loading: app.mcp_settings.inspect_loading,
            inspect_error: app.mcp_settings.inspect_error.as_ref(),
            defaults: defaults_input(app, draft),
        },
        draft,
    )
}
pub(super) fn catalog_input(app: &ProfileFixture) -> crate::catalog::Input<'_> {
    let state = &app.agents_settings;
    crate::catalog::Input {
        locale: app.desktop_locale,
        entries: &state.entries,
        selected_agent: &state.selected_agent,
        catalog_official: state.catalog_official,
        catalog_query: &state.catalog_query,
        catalog_busy: state.catalog_busy,
        save_in_flight: state.save_in_flight,
        profile_saving: state.profile_saving,
        dirty: state.dirty,
        profile_entering_key: state.profile_entering_key,
        diagnostic_results: &app.acp_settings.diagnostic_results,
        new_session_creating: false,
        icon: |agent, size| desktop_ui_controls::icons::acp_agent_icon(agent, size),
    }
}
pub(super) fn agents(app: &ProfileFixture) -> Element<'_, Event> {
    crate::agents_view::agents_view(crate::agents_view::Input {
        state: &app.agents_settings,
        catalog: catalog_input(app),
        theme: &app.app_theme,
    })
}
