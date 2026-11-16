//! Compose account, editor and notification domain views.
use super::{App, Message};
use iced::Element;
pub(crate) use desktop_navigation::status_bar::STATUS_BAR_HEIGHT;

pub(super) fn status_bar(app: &App) -> Element<'_, Message> {
    #[cfg(not(target_arch = "wasm32"))]
    let lsp_content = desktop_editor::status_bar::lsp_content(
        app.navigation_tabs.active_preview_path.as_deref(), &app.navigation_tabs.preview_tabs, &app.lsp_progress,
        app.spinner_frame, app.desktop_locale, crate::app::i18n::editor::lsp_status(app),
    );
    #[cfg(target_arch = "wasm32")]
    let lsp_content = iced::widget::Space::new().into();
    let notification = desktop_navigation::status_bar::notification_button(app.notifications.len())
        .map(|_| Message::Notification(crate::app::message::NotificationMessage::ToggleExpanded));
    desktop_navigation::status_bar::view(
        crate::app::components::account_status::view(app),
        crate::app::components::account_status::monthly_quota(app), lsp_content, notification,
    )
}
#[cfg(test)]
#[path = "app_view_status_tests.rs"]
mod app_view_status_tests;
