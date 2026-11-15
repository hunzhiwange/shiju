pub use desktop_settings_apps::system_settings_apps::preferences::{PreferencesState};
pub type PreferencesMessage = desktop_settings_apps::system_settings_apps::preferences::PreferencesMessage<crate::app::state::AgentAppsContext>;
fn event(value: PreferencesMessage) -> crate::app::Message { super::message(crate::app::message::settings::AppsCatalogMessage::Preferences(value)) }
pub(crate) fn update(app: &mut crate::app::App, message: PreferencesMessage) -> iced::Task<crate::app::Message> {
    match message { PreferencesMessage::Error(error) => return app.show_error_toast(error), PreferencesMessage::Success(text) => return app.show_success_toast(text), _ => {} }
    let gateway = crate::app::config::gateway_client().map_err(|e| e.to_string());
    let context = gateway.as_ref().ok().map(|client| crate::app::state::AgentAppsContext { gateway: client.endpoint().base_url(), tenant: client.endpoint().tenant_scope().clone(), project: app.project_path.clone() });
    let state = &mut app.apps_catalog;
    let mut input = desktop_settings_apps::system_settings_apps::preferences::Input { context: &state.context, host: &state.host, token: &state.token, generation: state.generation, preferences: &mut state.preferences, };
    desktop_settings_apps::system_settings_apps::preferences::update(&mut input, context, gateway, message).map(event)
}
#[path = "preferences_view.rs"]
mod view;
pub(crate) use view::overlay;
pub(crate) fn visible_rows(state: &crate::app::state::AppsCatalogState) -> Vec<&super::catalog::CatalogRow> { desktop_settings_apps::system_settings_apps::preferences::visible_rows(&super::view_state(state)) }
