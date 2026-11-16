use crate::headings::{HelpRenderer, settings_page_heading_with_help, settings_group_heading_with_help};
use desktop_i18n::I18n;
use iced::{Element, Length, Theme};
use iced::widget::column;
pub fn content<'a, Message: Clone + 'static>(locale: config_ui::DesktopLocale, theme: &Theme, fields: Element<'a, Message>, help: HelpRenderer<Message>) -> Element<'a, Message> {
 let i18n = I18n::new(locale);
 column![settings_group_heading_with_help(help, i18n, theme, i18n.__("主题与显示"), i18n.__("统一编辑器主题来源及排版参数。")), fields].spacing(16).width(Length::Fill).into()
}
pub fn view<'a, Message: Clone + 'static>(locale: config_ui::DesktopLocale, theme: &Theme, fields: Element<'a, Message>, help: HelpRenderer<Message>) -> Element<'a, Message> {
 let i18n = I18n::new(locale);
 column![settings_page_heading_with_help(help, i18n, theme, i18n.__("编辑器配置"), i18n.__("配置主题、字号与行高等编辑体验参数。")), content(locale, theme, fields, help)].spacing(16).into()
}
