//! 桌面消息与既有标题栏按钮的组合适配。
pub use crate::git_panel::diff_view::header::wrap_diff_header;
use crate::git_panel::ui::header_plain_glyph_button;
use crate::git_panel::ui::square_icon_button_tiny;
use crate::page::Message;
use desktop_assets::assets::Icon;
use iced::Element;
pub fn build_diff_header(
    title: String,
    insertions: usize,
    deletions: usize,
    close_message: Option<Message>,
    fullscreen_message: Option<Message>,
    fullscreen_tip: Option<String>,
    fullscreen_icon: Option<Icon>,
) -> Element<'static, Message> {
    let close =
        close_message.map(|message| header_plain_glyph_button("✕", "关闭".to_string(), message));
    let fullscreen = fullscreen_message
        .zip(fullscreen_tip)
        .zip(fullscreen_icon)
        .map(|((message, tip), icon)| square_icon_button_tiny(icon, tip, message));
    crate::git_panel::diff_view::header::build_diff_header(
        title, insertions, deletions, close, fullscreen,
    )
}
