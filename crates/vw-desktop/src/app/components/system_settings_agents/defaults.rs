//! Project cross-domain catalogs for the profile save boundary.
use crate::app::App;
use crate::app::state::DelegateAgentSettingsEntry;
pub(crate) fn validation_error(app: &App, draft: &DelegateAgentSettingsEntry) -> Option<String> {
    desktop_settings_agents::defaults::validation_error(
        &super::domain::defaults_input(app, draft),
        draft,
    )
}
