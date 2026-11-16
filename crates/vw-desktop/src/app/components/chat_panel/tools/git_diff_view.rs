//! 桌面卡片输入投影与消息映射。
#[allow(unused_imports)]
pub(crate) use desktop_chat_tools::tools::git_diff_view::*;
pub(crate) fn render_git_diff_content<'a>(app: &'a crate::app::App, title: String, path: Option<String>, before: String, after: String) -> iced::Element<'a, crate::app::Message> {
 desktop_chat_tools::tools::git_diff_view::render_git_diff_content(
  crate::app::components::git_panel::embedded_custom_text_diff_view(app,title,path,before,after,None))
}

pub fn tool_git_diff_view<'a>(app: &'a crate::app::App, msg_idx: usize, tool_idx: usize, visible: &str) -> Option<iced::Element<'a, crate::app::Message>> {
 let input = desktop_chat_tools::tools::result_input::ResultInput { i18n: crate::app::i18n::I18n::new(app.desktop_locale), hovered: app.chat_tool_hovered_idx, question_request: app.question_modal_request.as_ref(), message_ids: &app.chat_message_ids };
 desktop_chat_tools::tools::git_diff_view::tool_git_diff_view(&input, msg_idx, tool_idx, visible, &|title,path,before,after| render_git_diff_content(app,title,path,before,after)).map(|view| view.map(super::card_adapter::map))
}
