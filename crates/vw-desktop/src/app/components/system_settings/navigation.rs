//! Project host search targets into the settings navigation.

use crate::app::components::system_settings_search::{
    SystemSettingsSearchEntry, SystemSettingsSearchTarget, navigation_label, search_results,
};
use crate::app::message::{SettingsMessage, ViewMessage};
use crate::app::{App, Message};
pub(super) use desktop_settings_shell::navigation::GROUPS;
pub(crate) use desktop_settings_shell::navigation::SETTINGS_NAVIGATION_SCROLL;
use desktop_settings_shell::navigation::{NavigationEntry, NavigationGroup};
pub(super) fn view(app: &App, width: f32) -> iced::Element<'_, Message> {
    let groups = if app.general_settings.system_settings_query.trim().is_empty() {
        GROUPS
            .iter()
            .map(|(label, tabs)| NavigationGroup {
                label: Some(*label),
                entries: tabs
                    .iter()
                    .map(|&tab| {
                        project(
                            app,
                            SystemSettingsSearchEntry::new(
                                navigation_label(app.general_settings.desktop_locale, tab),
                                None,
                                &[],
                                SystemSettingsSearchTarget::TopLevel(tab),
                            ),
                        )
                    })
                    .collect(),
            })
            .collect()
    } else {
        vec![NavigationGroup {
            label: None,
            entries: search_results(app, &app.general_settings.system_settings_query)
                .into_iter()
                .map(|entry| project(app, entry))
                .collect(),
        }]
    };
    desktop_settings_shell::navigation::view(
        &app.general_settings.system_settings_query,
        crate::app::i18n::I18n::new(app.general_settings.desktop_locale),
        width,
        groups,
        Message::View(ViewMessage::ToggleSystemSettings),
        |value| Message::Settings(SettingsMessage::SystemTabQueryChanged(value)),
    )
}
fn project(app: &App, entry: SystemSettingsSearchEntry) -> NavigationEntry<Message> {
    NavigationEntry {
        active: search_target_is_active(app, entry.target),
        tab: entry.target.page(),
        parent_label: entry.parent_path(),
        label: entry.label,
        on_press: Message::Settings(SettingsMessage::SystemSearchTargetSelected(entry.target)),
    }
}
fn search_target_is_active(app: &App, target: SystemSettingsSearchTarget) -> bool {
    use desktop_settings_shell::search::{ActiveNavigation, search_target_is_active};
    if let SystemSettingsSearchTarget::ThirdParty(page) = target {
        return app.navigation_panels.system_settings_tab == desktop_settings_shell::SystemTab::Apps
            && app.apps_catalog.third_party_page == page;
    }
    search_target_is_active(
        &ActiveNavigation {
            page: app.navigation_panels.system_settings_tab,
            general: app.general_settings.general_settings_tab,
            models: app.model_settings.active_tab,
            cron: app.cron_settings.active_tab,
            agents: app.agents_settings.active_config_tab,
            channels: app.channels_settings.active_tab,
            memory: app.memory_settings.active_tab,
            autonomy: app.autonomy_settings.active_tab,
            security: app.security_settings.active_tab,
            gateway: app.gateway_settings.active_tab,
            mcp: app.mcp_settings.active_tab,
            skills: app.skills_settings.active_tab,
        },
        target,
    )
}
