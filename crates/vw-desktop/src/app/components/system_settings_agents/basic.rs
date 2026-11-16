use crate::app::App;
use crate::app::state::DelegateAgentSettingsEntry;
pub(super) fn input<'a>(app: &'a App, entry: &'a DelegateAgentSettingsEntry) -> desktop_settings_agents::basic::Input<'a> {

desktop_settings_agents::basic::Input { help: super::domain::help,
        locale: app.desktop_locale, theme: &app.app_theme,
        profile: super::domain::profile_input(app), defaults: super::domain::defaults_input(app, entry),
    }
}
#[cfg(test)]
#[path = "basic_ui_refresh_16_tests.rs"]
mod ui_refresh_16_tests;
