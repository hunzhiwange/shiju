//! Validated resynchronization state projection and message mapping.
use crate::app::{App, Message};
use crate::app::message::settings::{mcp_management_resync as resync, mcp_management_import as import, mcp_management_bindings as bindings};
use desktop_settings_mcp::resync as ui;
use iced::Element;
fn project(state: &resync::ResyncState) -> ui::ResyncView<'_> {
    ui::ResyncView { locked: resync::locked(state), has_operation: state.operation.is_some(),
        busy: state.busy, cancelling: state.cancelling, status: &state.status, request: &state.request,
        observed: &state.observed, error: &state.error, selected: &state.selected, account: &state.account }
}
fn message(event: ui::ResyncMessage) -> Message {
    use resync::ResyncMessage as M;
    resync::message(match event {
        ui::ResyncMessage::Open => M::Open, ui::ResyncMessage::RetryFailed => M::RetryFailed,
        ui::ResyncMessage::Refresh => M::Refresh, ui::ResyncMessage::Cancel => M::Cancel,
        ui::ResyncMessage::Close => M::Close, ui::ResyncMessage::Select(v, b) => M::Select(v, b),
        ui::ResyncMessage::Account(v) => M::Account(v), ui::ResyncMessage::Start => M::Start,
    })
}
pub(crate) fn panel(app: &App) -> Element<'_, Message> {
    ui::panel(resync::visible(app).map(project)).map(message)
}
pub(crate) fn overlay<'a>(app: &'a App, dialog: Element<'a, Message>) -> Element<'a, Message> {
    let Some(state) = resync::visible(app).filter(|s| s.open) else { return dialog; };
    let scope = if app.mcp_settings.bindings.selected.is_empty() { "当前租户".to_owned() } else {
        match bindings::project(&app.mcp_settings.bindings.selected) {
            Ok(Some(project)) => format!("项目 {}", project.0), Ok(None) => "当前租户".to_owned(),
            Err(error) => format!("无效：{error}"),
        }
    };
    desktop_ui_controls::system_settings_common::settings_modal_overlay(
        Some(dialog), message(ui::ResyncMessage::Close), ui::card(project(state), scope, import::targets(app)).map(message))
}
