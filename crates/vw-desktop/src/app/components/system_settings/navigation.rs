//! Project host search targets into the settings navigation.
use super::{SystemTab, is_network_tab};
use crate::app::{App, Message};
use crate::app::components::system_settings_search::{SystemSettingsSearchEntry, SystemSettingsSearchTarget, navigation_label, search_results};
use crate::app::message::{SettingsMessage, ViewMessage};
use desktop_settings_shell::navigation::{NavigationEntry, NavigationGroup};
pub(super) use desktop_settings_shell::navigation::GROUPS;
pub(crate) use desktop_settings_shell::navigation::SETTINGS_NAVIGATION_SCROLL;
pub(super) fn view(app: &App, width: f32) -> iced::Element<'_, Message> {
    let groups = if app.system_settings_query.trim().is_empty() {
        GROUPS.iter().map(|(label, tabs)| NavigationGroup {
            label: Some(*label),
            entries: tabs.iter().map(|&tab| project(app, SystemSettingsSearchEntry::new(
                navigation_label(app.desktop_locale, tab), None, &[], SystemSettingsSearchTarget::TopLevel(tab)
            ))).collect(),
        }).collect()
    } else {
        vec![NavigationGroup { label: None, entries: search_results(app, &app.system_settings_query).into_iter().map(|entry| project(app, entry)).collect() }]
    };
    desktop_settings_shell::navigation::view(
        &app.system_settings_query, crate::app::i18n::I18n::new(app.desktop_locale), width, groups,
        Message::View(ViewMessage::ToggleSystemSettings),
        |value| Message::Settings(SettingsMessage::SystemTabQueryChanged(value)),
    )
}
fn project(app: &App, entry: SystemSettingsSearchEntry) -> NavigationEntry<Message> {
    NavigationEntry { active: search_target_is_active(app, entry.target), tab: entry.target.page(),
        parent_label: entry.parent_path(), label: entry.label,
        on_press: Message::Settings(SettingsMessage::SystemSearchTargetSelected(entry.target)),
    }
}
fn search_target_is_active(app: &App, target: SystemSettingsSearchTarget) -> bool {
    match target {
        SystemSettingsSearchTarget::GeneralField(_) => false,
        SystemSettingsSearchTarget::TopLevel(tab) => {
            tab == app.system_settings_tab
                || (tab == SystemTab::WebSearch && is_network_tab(app.system_settings_tab))
        }
        SystemSettingsSearchTarget::General(tab) => {
            app.system_settings_tab == SystemTab::General && app.general_settings_tab == tab
        }
        SystemSettingsSearchTarget::Models(tab) => {
            app.system_settings_tab == SystemTab::Models && app.model_settings.active_tab == tab
        }
        SystemSettingsSearchTarget::Cron(tab) => {
            app.system_settings_tab == SystemTab::Cron && app.cron_settings.active_tab == tab
        }
        SystemSettingsSearchTarget::Agents(tab) => {
            app.system_settings_tab == SystemTab::Agents
                && app.agents_settings.active_config_tab == tab
        }
        SystemSettingsSearchTarget::Channels(tab) => {
            app.system_settings_tab == SystemTab::Agents
                && app.agents_settings.active_config_tab
                    == crate::app::state::AgentConfigTab::Channels
                && app.channels_settings.active_tab == tab
        }
        SystemSettingsSearchTarget::Memory(tab) => {
            app.system_settings_tab == SystemTab::Memory && app.memory_settings.active_tab == tab
        }
        SystemSettingsSearchTarget::Autonomy(tab) => {
            app.system_settings_tab == SystemTab::Autonomy
                && app.autonomy_settings.active_tab == tab
        }
        SystemSettingsSearchTarget::Security(tab) => {
            app.system_settings_tab == SystemTab::Security
                && app.security_settings.active_tab == tab
        }
        SystemSettingsSearchTarget::Gateway(tab) => {
            app.system_settings_tab == SystemTab::Gateway && app.gateway_settings.active_tab == tab
        }
        SystemSettingsSearchTarget::Network(tab) => app.system_settings_tab == tab,
        SystemSettingsSearchTarget::AgentMcp(tab) => {
            app.system_settings_tab == SystemTab::Agents
                && app.agents_settings.active_config_tab == crate::app::state::AgentConfigTab::Mcp
                && app.mcp_settings.active_tab == tab
        }
        SystemSettingsSearchTarget::AgentSkills(tab) => {
            app.system_settings_tab == SystemTab::Agents
                && app.agents_settings.active_config_tab
                    == crate::app::state::AgentConfigTab::Skills
                && app.skills_settings.active_tab == tab
        }
    }
}
