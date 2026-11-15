//! 账号领域视图映射到桌面全局消息。
use crate::app::{App, Message};
use iced::Element;
fn map(message: desktop_account_ui::view::Message) -> Message {
    match message {
        desktop_account_ui::view::Message::Account(message) => Message::Account(message),
        desktop_account_ui::view::Message::OpenInstallGuide => {
            #[cfg(not(target_arch = "wasm32"))]
            {
                Message::SkillHub(crate::app::message::SkillHubMessage::OpenInstallGuide)
            }
            #[cfg(target_arch = "wasm32")]
            {
                Message::Account(crate::app::message::AccountMessage::CloseMenu)
            }
        }
    }
}
pub(crate) fn trigger_label(app: &App) -> String {
    desktop_account_ui::view::trigger_label(&app.account, app.desktop_locale)
}
pub(crate) fn view(app: &App) -> Element<'_, Message> {
    desktop_account_ui::view::view(&app.account, app.desktop_locale).map(map)
}
pub(crate) fn monthly_quota(app: &App) -> Option<Element<'_, Message>> {
    desktop_account_ui::view::monthly_quota(&app.account, app.desktop_locale)
        .map(|view| view.map(map))
}
#[cfg(test)]
fn menu(app: &App) -> Element<'_, Message> {
    desktop_account_ui::view::menu(&app.account, app.desktop_locale).map(map)
}
#[cfg(test)]
#[path = "account_status_ui_refresh_31_tests.rs"]
mod ui_refresh_31_tests;
