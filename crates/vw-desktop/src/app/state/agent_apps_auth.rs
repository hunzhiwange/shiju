pub use desktop_settings_account::types::*;
pub type State = desktop_provider_apps::auth::state::State<crate::app::components::apps_usage_cards::Cards>;
impl desktop_provider_apps::auth::state::Usage for crate::app::components::apps_usage_cards::Cards {
    fn clear(&mut self) { self.clear(); }
}
