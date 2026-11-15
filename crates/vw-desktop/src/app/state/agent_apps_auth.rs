pub use desktop_settings_account::types::*;
pub type State = desktop_apps_ui::agent_apps_auth::state::State<crate::app::components::apps_usage_cards::Cards>;
impl desktop_apps_ui::agent_apps_auth::state::Usage for crate::app::components::apps_usage_cards::Cards {
    fn clear(&mut self) { self.clear(); }
}
