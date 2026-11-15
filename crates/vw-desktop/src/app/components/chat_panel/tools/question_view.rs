//! 桌面卡片输入投影与消息映射。
#[allow(unused_imports)]
pub(crate) use desktop_chat_tools::tools::question_view::*;
pub fn tool_question_view<'a>(app: &'a crate::app::App, msg_idx: usize, tool_idx: usize, visible: &str) -> Option<iced::Element<'a, crate::app::Message>> {
 let input = desktop_chat_tools::tools::result_input::ResultInput { i18n: crate::app::i18n::I18n::new(app.desktop_locale), hovered: app.chat_tool_hovered_idx, question_request: app.question_modal_request.as_ref(), message_ids: &app.chat_message_ids };
 desktop_chat_tools::tools::question_view::tool_question_view(&input, msg_idx, tool_idx, visible).map(|view| view.map(super::card_adapter::map_card))
}
