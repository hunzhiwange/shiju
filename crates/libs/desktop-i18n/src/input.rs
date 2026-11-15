//! Semantic feedback retained alongside the legacy raw error fields.
use super::{
    I18n,
    format::{Argument, Message},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ErrorField {
    General,
    WorktreeDelete,
    WorktreeReset,
}

#[derive(Debug, Clone)]
pub struct Feedback {
    pub fallback: String,
    message: Message,
}
impl Feedback {
    pub fn new(source: &str, arguments: Vec<Argument>) -> Self {
        let message = Message::new(source, arguments);
        let fallback = message
            .render(I18n::new(vw_config_types::ui::DesktopLocale::SimplifiedChinese))
            .expect("valid feedback template");
        Self { fallback, message }
    }
    pub fn render(&self, i18n: I18n, raw: &str) -> String {
        if self.fallback == raw {
            self.message.render(i18n).expect("valid feedback template")
        } else {
            raw.to_owned()
        }
    }
}
