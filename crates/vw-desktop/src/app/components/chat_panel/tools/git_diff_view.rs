//! 桌面卡片输入投影与消息映射。
#[allow(unused_imports)]
pub(crate) use desktop_chat_tools::tools::git_diff_view::*;
pub(crate) fn render_git_diff_content<'a>(
    app: &'a crate::app::App,
    title: String,
    path: Option<String>,
    before: String,
    after: String,
) -> iced::Element<'a, crate::app::Message> {
    desktop_chat_tools::tools::git_diff_view::render_git_diff_content(
        crate::app::components::git_panel::embedded_custom_text_diff_view(
            app, title, path, before, after, None,
        ),
    )
}
