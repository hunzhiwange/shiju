use desktop_i18n::I18n;
use desktop_ui_controls::system_settings_common::{settings_field_label,help::settings_hover_help_label};
use iced::{Element,Theme,widget::text};
use crate::Message;
use std::borrow::Cow;
pub type HelpRenderer = for<'a> fn(&'a str, Cow<'a,str>, Option<iced::Color>, I18n) -> Element<'a,Message>;
#[derive(Clone,Copy)]
pub struct Help<'a> { pub i18n: I18n, pub theme: &'a Theme, pub render: HelpRenderer }
impl Help<'_> {
    pub fn field(self, title: &'static str, description: &'static str) -> Element<'static,Message> {
        let title=self.i18n.__(title);
        settings_hover_help_label(settings_field_label(title), (self.render)(title,self.i18n.__(description).into(),Some(self.theme.palette().text),self.i18n)).wrap().into()
    }
    pub fn heading(self, title: &'static str, description: &'static str,size:u32) -> Element<'static,Message> {
        let title=self.i18n.__(title);
        settings_hover_help_label(text(title).size(size), (self.render)(title,self.i18n.__(description).into(),Some(self.theme.palette().text),self.i18n)).wrap().into()
    }
    pub fn group(self,title: &'static str,description: &'static str)->Element<'static,Message>{ self.heading(title,description,17) }
}
