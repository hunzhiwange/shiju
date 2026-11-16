//! Local agents settings coverage migrated from the desktop host.
//! Only the selectable chat help boundary is supplied by this fixture.
use crate::{agents_state::AgentsSettingsState, message::Event, state::DelegateAgentSettingsEntry};
use config_ui::DesktopLocale;
use iced::{Element, Theme};

static THEME: Theme = Theme::Light;

mod basic;
mod defaults;
mod identity;
mod mcp;
mod shared;
mod skills;
mod tools;

fn help<'a>(
    _: desktop_i18n::I18n,
    _: Option<&Theme>,
    _: &'a str,
    label: iced::widget::Text<'a>,
    description: std::borrow::Cow<'a, str>,
) -> iced::widget::Row<'a, Event> {
    desktop_ui_controls::system_settings_common::help::settings_hover_help_label(
        label,
        iced::widget::text(description.into_owned()).into(),
    )
}

fn basic_view<'a>(
    state: &'a AgentsSettingsState,
    entry: &'a DelegateAgentSettingsEntry,
    providers: Vec<String>,
) -> Element<'a, Event> {
    crate::basic::view(
        crate::basic::Input {
            help,
            locale: DesktopLocale::SimplifiedChinese,
            theme: &THEME,
            profile: crate::basic_profile::Input {
                locale: DesktopLocale::SimplifiedChinese,
                entries: &state.entries,
                acp_agents: &[],
                saving: state.profile_saving,
                acp_loading: false,
                acp_error: None,
            },
            defaults: crate::defaults::Input {
                help,
                locale: DesktopLocale::SimplifiedChinese,
                theme: &THEME,
                model_source_available: true,
                mcp_ready: false,
                models: vec![],
                skills: vec![],
                mcp: vec![],
            },
        },
        entry,
        providers,
    )
}

fn identity_view<'a>(
    state: &'a AgentsSettingsState,
    entry: &'a DelegateAgentSettingsEntry,
) -> Element<'a, Event> {
    let workspace_metadata = state
        .workspace_identity_files
        .iter()
        .find(|file| file.file_name == state.active_prompt_tab)
        .and_then(|file| {
            crate::identity::workspace_identity_meta_line(
                DesktopLocale::SimplifiedChinese,
                file.size_bytes,
                file.modified_at_ms.map(|_| (0, 0)),
            )
        });
    crate::identity::view(
        crate::identity::Input {
            help,
            locale: DesktopLocale::SimplifiedChinese,
            theme: &THEME,
            active_prompt_tab: &state.active_prompt_tab,
            workspace_identity_files: &state.workspace_identity_files,
            workspace_identity_root_path: state.workspace_identity_root_path.as_deref(),
            selected_agent: &state.selected_agent,
            workspace_metadata,
        },
        entry,
    )
}

fn selected_entry(state: &AgentsSettingsState) -> Option<&DelegateAgentSettingsEntry> {
    crate::shared::selected_entry(
        state.profile_modal.as_ref(),
        &state.entries,
        &state.selected_agent,
    )
}

fn tools_view<'a>(
    state: &'a AgentsSettingsState,
    entry: &'a DelegateAgentSettingsEntry,
) -> Element<'a, Event> {
    crate::tools::view(
        crate::tools::Input {
            help,
            locale: DesktopLocale::SimplifiedChinese,
            theme: &THEME,
            available_tools: &state.available_tools,
            active_tools_list_tab: &state.active_tools_list_tab,
        },
        entry,
    )
}

fn skills_input(state: &AgentsSettingsState) -> crate::skills::Input<'_> {
    crate::skills::Input {
        help,
        locale: DesktopLocale::SimplifiedChinese,
        theme: &THEME,
        project_options: vec![],
        skill_directory: &state.skill_directory,
        skill_directory_scope: state.skill_directory_scope,
        skill_catalog: &state.skill_catalog,
        skill_catalog_loading: state.skill_catalog_loading,
        skill_catalog_error: &state.skill_catalog_error,
    }
}

fn skills_view<'a>(
    state: &'a AgentsSettingsState,
    entry: &'a DelegateAgentSettingsEntry,
) -> Element<'a, Event> {
    crate::skills::view(skills_input(state), entry)
}

fn discovery_order_text(state: &AgentsSettingsState) -> String {
    crate::skills::discovery_order_text(&skills_input(state))
}
