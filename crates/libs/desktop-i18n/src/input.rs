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
            .render(I18n::new(config_ui::DesktopLocale::SimplifiedChinese))
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

/// Localized input errors and generated session labels shared across host routes.
#[derive(Default)]
pub struct State {
    /// 错误消息
    pub error_message: Option<String>,
    pub localized_input_errors:
        std::collections::BTreeMap<crate::input::ErrorField, crate::input::Feedback>,
    /// 用于模型聊天时间上下文的 IANA 时区
    pub generated_session_titles: std::collections::HashMap<String, crate::input::Feedback>,
}

impl State {
    pub fn render_error(&self, field: ErrorField, i18n: I18n, raw: &str) -> String {
        self.localized_input_errors
            .get(&field)
            .map(|feedback| feedback.render(i18n, raw))
            .unwrap_or_else(|| raw.to_owned())
    }
}
