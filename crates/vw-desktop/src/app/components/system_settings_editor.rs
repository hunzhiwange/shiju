//! 编辑器设置字段注入与页面委派。
use crate::app::{App, Message};
use desktop_settings_appearance::editor as domain;
use iced::{Element, Length};
fn fields(app: &App) -> Element<'_, Message> {
    iced::widget::responsive(move |size| {
        super::editor_settings_fields::view(app, (size.width * 0.40).min(250.0))
    })
    .height(Length::Shrink)
    .into()
}
pub(crate) fn content(app: &App) -> Element<'_, Message> {
    domain::content(app.general_settings.desktop_locale, &app.appearance.app_theme, fields(app), render_help)
}
pub fn view(app: &App) -> Element<'_, Message> {
    domain::view(app.general_settings.desktop_locale, &app.appearance.app_theme, fields(app), render_help)
}
fn render_help<'a>(
    title: &'a str,
    body: std::borrow::Cow<'a, str>,
    color: Option<iced::Color>,
    i18n: desktop_i18n::I18n,
) -> iced::Element<'a, Message> {
    super::chat_panel::message_view::selectable_titled_text(title, body, color, i18n)
}
