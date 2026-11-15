pub use desktop_settings_apps::system_settings_apps::lifecycle::*;
use crate::app::Message;
pub(crate) fn action_button<'a>(label: &'a str, request: vw_api_app_operations::AppActionPreviewRequest, enabled: bool) -> iced::Element<'a, Message> {
    desktop_settings_apps::system_settings_apps::lifecycle_view::action_button(label, request, enabled, super::domain_event)
}
pub(crate) fn progress(state: &LifecycleState) -> iced::Element<'_, Message> {
    desktop_settings_apps::system_settings_apps::lifecycle_view::progress(state, super::domain_event)
}
pub(crate) fn overlay<'a>(state: &'a LifecycleState, host: &'a str, base: iced::Element<'a, Message>) -> iced::Element<'a, Message> {
    let guidance = state.preview.as_ref().map(|p| super::uninstall::guidance(p.request.app_id)).unwrap_or("");
    desktop_settings_apps::system_settings_apps::lifecycle_view::overlay(state, host, base, guidance, super::domain_event)
}
#[cfg(test)]
#[path = "../../desktop_integration_20_tests.rs"]
mod desktop_integration_20_tests;
