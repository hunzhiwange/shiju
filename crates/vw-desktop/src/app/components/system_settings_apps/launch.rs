pub use desktop_settings_apps::system_settings_apps::launch::{LaunchState, LaunchChoice};
pub type LaunchMessage = desktop_settings_apps::system_settings_apps::launch::LaunchMessage<crate::app::state::AgentAppsContext>;
pub type ReplyScope = desktop_settings_apps::system_settings_apps::launch::ReplyScope<crate::app::state::AgentAppsContext>;
fn event(value: LaunchMessage) -> crate::app::Message { super::message(crate::app::message::settings::AppsCatalogMessage::Launch(value)) }
pub(crate) fn update(app: &mut crate::app::App, message: LaunchMessage) -> iced::Task<crate::app::Message> {
    match message { LaunchMessage::Error(error) => return app.show_error_toast(error), LaunchMessage::Success(text) => return app.show_success_toast(text), _ => {} }
    let gateway = crate::app::config::gateway_client().map_err(|e| e.to_string());
    let context = gateway.as_ref().ok().map(|client| crate::app::state::AgentAppsContext { gateway: client.endpoint().base_url(), tenant: client.endpoint().tenant_scope().clone(), project: app.navigation_tabs.project_path.clone() });
    let Some(context) = context else { app.apps_catalog.launch.invalidate(); return app.show_error_toast("网关不可用，请重新加载应用"); };
    let state = &mut app.apps_catalog;
    let mut input = desktop_settings_apps::system_settings_apps::launch::Input { context: &state.context, host: &state.host, token: &state.token, generation: state.generation, loading: state.loading, launch: &mut state.launch, rows: &state.rows, };
    desktop_settings_apps::system_settings_apps::launch::update(&mut input, &app.navigation_tabs.project_path, gateway, context, message).map(event)
}
#[path = "launch_view.rs"]
mod view;
pub(crate) use view::overlay;
