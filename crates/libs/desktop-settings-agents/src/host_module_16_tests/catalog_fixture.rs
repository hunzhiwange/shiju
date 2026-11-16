use crate::{acp_state::AcpDiagnosticStatus, agents_state::AgentsSettingsState, message::Event};
use std::collections::BTreeMap;
#[derive(Default)]
pub(super) struct CatalogFixture {
    pub agents_settings: AgentsSettingsState,
    pub new_session_creating: bool,
    diagnostics: BTreeMap<String, AcpDiagnosticStatus>,
}
impl CatalogFixture {
    pub fn catalog_input(&self) -> crate::catalog::Input<'_> {
        let state = &self.agents_settings;
        crate::catalog::Input {
            locale: config_ui::DesktopLocale::SimplifiedChinese,
            entries: &state.entries,
            selected_agent: &state.selected_agent,
            catalog_official: state.catalog_official,
            catalog_query: &state.catalog_query,
            catalog_busy: state.catalog_busy,
            save_in_flight: state.save_in_flight,
            profile_saving: state.profile_saving,
            dirty: state.dirty,
            profile_entering_key: state.profile_entering_key,
            diagnostic_results: &self.diagnostics,
            new_session_creating: self.new_session_creating,
            icon,
        }
    }
    pub fn view_input(&self) -> crate::agents_view::Input<'_> {
        static THEME: iced::Theme = iced::Theme::Light;
        crate::agents_view::Input {
            state: &self.agents_settings,
            catalog: self.catalog_input(),
            theme: &THEME,
        }
    }
}
fn icon(agent: &str, size: f32) -> iced::Element<'static, Event> {
    desktop_ui_controls::icons::acp_agent_icon(agent, size)
}
