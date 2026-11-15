//! 视图实际使用的主题、语言及壳提供的可选择帮助文本渲染入口。
use std::borrow::Cow;
use config_ui::DesktopLocale;
use desktop_i18n::I18n;
use desktop_ui_controls::system_settings_common::{help, settings_field_label, settings_flexible_row_with_label};
use iced::{Color, Element, Length, Theme, widget::{self, text}};
use crate::view_message::Message;

pub type HelpText = for<'a> fn(&'a str, Cow<'a, str>, Option<Color>, I18n) -> Element<'a, Message>;
#[derive(Clone, Copy)]
pub struct ViewStyle<'a> {
    pub locale: DesktopLocale,
    pub theme: &'a Theme,
    pub help_text: HelpText,
}
impl ViewStyle<'_> {
    pub fn help_label<'a>(self, title: &'a str, label: widget::Text<'a>, description: impl Into<Cow<'a,str>>) -> widget::Row<'a, Message> {
        help::settings_hover_help_label(label, (self.help_text)(title, description.into(), Some(self.theme.palette().text), I18n::new(self.locale)))
    }
    pub fn page_heading<'a>(self, title: &'a str, description: impl Into<Cow<'a,str>>) -> Element<'a,Message> {
        self.help_label(title, text(title).size(28), description).wrap().into()
    }
    pub fn group_heading<'a>(self, title: &'a str, description: impl Into<Cow<'a,str>>) -> Element<'a,Message> {
        self.help_label(title, text(title).size(17), description).wrap().into()
    }
    pub fn field<'a>(self, title: &'a str, description: impl Into<Cow<'a,str>>, width: f32, control: impl Into<Element<'a,Message>>) -> widget::Container<'a,Message> {
        let label = self.help_label(title, settings_field_label(title), description).width(Length::Fill).wrap();
        settings_flexible_row_with_label(label, width, control)
    }
}
