//! 桌面消息与既有标题栏按钮的组合适配。
use crate::app::{Message, assets::Icon};
use iced::Element;
use super::super::ui::{header_plain_glyph_button, square_icon_button_tiny};
pub use desktop_git_ui::git_panel::diff_view::header::wrap_diff_header;
pub fn build_diff_header(
    title: String, insertions: usize, deletions: usize,
    close_message: Option<Message>, fullscreen_message: Option<Message>,
    fullscreen_tip: Option<String>, fullscreen_icon: Option<Icon>,
) -> Element<'static, Message> {
    let close = close_message.map(|message| header_plain_glyph_button("✕", "关闭".to_string(), message));
    let fullscreen = fullscreen_message.zip(fullscreen_tip).zip(fullscreen_icon)
        .map(|((message, tip), icon)| square_icon_button_tiny(icon, tip, message));
    desktop_git_ui::git_panel::diff_view::header::build_diff_header(title, insertions, deletions, close, fullscreen)
}
