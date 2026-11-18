use crate::headings::{HelpRenderer, settings_page_heading_with_help};
use desktop_i18n::I18n;
use iced::widget::column;
use iced::{Element, Length, Theme};
pub fn view<'a, Message: Clone + 'static>(
    locale: config_ui::DesktopLocale,
    theme: &Theme,
    fields: Element<'a, Message>,
    help: HelpRenderer<Message>,
) -> Element<'a, Message> {
    let i18n = I18n::new(locale);
    column![
        settings_page_heading_with_help(
            help,
            i18n,
            theme,
            i18n.__("编辑器配置"),
            i18n.__("配置主题、字号与行高等编辑体验参数。")
        ),
        fields
    ]
    .spacing(16)
    .width(Length::Fill)
    .into()
}
