use super::{App, Message};
use iced::Element;
pub(crate) fn with_account_details<'a>(
    app: &'a App,
    root: Element<'a, Message>,
) -> Element<'a, Message> {
    desktop_account_ui::account_details::with_account_details(
        &app.account,
        app.general_settings.desktop_locale,
        root,
        Message::Account,
    )
}
