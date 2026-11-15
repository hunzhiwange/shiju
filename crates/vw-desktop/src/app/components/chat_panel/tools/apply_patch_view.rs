use crate::app::{App, Message};
pub fn tool_apply_patch_view<'a>(app: &'a App, m: usize, t: usize, visible: &str) -> Option<iced::Element<'a, Message>> {
 desktop_chat_tools::tools::apply_patch_view::tool_apply_patch_view(&super::card_adapter::input(app),m,t,visible, |title,path,before,after| super::git_diff_view::render_git_diff_content(app,title,path,before,after)).map(|e| e.map(super::card_adapter::map))
}
pub(super) use desktop_chat_tools::tools::apply_patch_view::apply_patch_file_expand_key;
