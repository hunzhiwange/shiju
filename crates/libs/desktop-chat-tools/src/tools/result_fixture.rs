//! 最小结果卡片测试存储，不构造桌面应用。
#[derive(Default)]
pub struct TestApp {
 pub chat_tool_hovered_idx: Option<u64>,
 pub question_modal_request: Option<question_types::Request>,
 pub chat_message_ids: Vec<Option<String>>,
}
impl TestApp {
 pub fn new() -> (Self, ()) { (Self::default(), ()) }
 pub fn input(&self) -> super::result_input::ResultInput<'_> {
  super::result_input::ResultInput { i18n: desktop_i18n::I18n::new(config_ui::DesktopLocale::SimplifiedChinese), hovered: self.chat_tool_hovered_idx, question_request: self.question_modal_request.as_ref(), message_ids: &self.chat_message_ids }
 }
}
