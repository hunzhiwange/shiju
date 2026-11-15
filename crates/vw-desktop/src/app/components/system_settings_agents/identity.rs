use crate::app::App;
#[cfg(test)] use crate::app::Message;
use crate::app::state::DelegateAgentSettingsEntry;
#[cfg(test)] use iced::Element;
pub(super) fn workspace_identity_meta_line(locale: config_ui::DesktopLocale, size: Option<u64>, modified: Option<u64>) -> Option<String> {
    desktop_settings_agents::identity::workspace_identity_meta_line(locale, size, modified.map(|timestamp|
        crate::app::components::chat_panel::utils::relative_time_bucket(timestamp, crate::app::time::now_ms())))
}
pub(super) fn input<'a>(app: &'a App, _entry: &'a DelegateAgentSettingsEntry) -> desktop_settings_agents::identity::Input<'a> {

    let settings = &app.agents_settings;
    let workspace_metadata = settings.workspace_identity_files.iter().find(|file| file.file_name == settings.active_prompt_tab)
        .and_then(|file| workspace_identity_meta_line(app.desktop_locale, file.size_bytes, file.modified_at_ms));
desktop_settings_agents::identity::Input { help: super::domain::help,
        locale: app.desktop_locale, theme: &app.app_theme,
        active_prompt_tab: &settings.active_prompt_tab, workspace_identity_files: &settings.workspace_identity_files,
        workspace_identity_root_path: settings.workspace_identity_root_path.as_deref(),
        selected_agent: &settings.selected_agent, workspace_metadata,
    }
}
#[cfg(test)]
pub(super) fn view<'a>(app: &'a App, entry: &'a DelegateAgentSettingsEntry) -> Element<'a, Message> { desktop_settings_agents::identity::view(input(app, entry), entry).map(super::domain::event) }
