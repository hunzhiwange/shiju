//! 桌面文件卡片投影与差异渲染装配。
pub fn tool_files_view<'a>(app: &'a crate::app::App, msg_idx: usize, tool_idx: usize, visible: &str) -> Option<iced::Element<'a, crate::app::Message>> {
 let input = desktop_chat_tools::tools::files_input::FilesInput {
  card: super::card_adapter::input(app), filter: &app.tool_files_filter,
  file_hovered: app.chat_tool_file_hovered.as_deref(),
 };
 desktop_chat_tools::tools::files_view::tool_files_view(&input, msg_idx, tool_idx, visible,
  &|title,file,before,after| super::git_diff_view::render_git_diff_content(app,title,file,before,after))
 .map(|view| view.map(super::card_adapter::map))
}
