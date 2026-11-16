//! 文本差异的桌面状态和消息适配。
use crate::DiffTheme;
use crate::page::Message;
use crate::page::ViewContext;
use iced::Color;
use iced::Element;
pub fn view_custom_text_diff<'a>(
    app: &ViewContext<'a>,
    title: String,
    file: Option<String>,
    old_content: String,
    new_content: String,
    close_message: Option<Message>,
    effective_theme: DiffTheme,
    bg_default: Color,
    add_line_bg: Color,
    add_word_bg: Color,
    del_line_bg: Color,
    del_word_bg: Color,
) -> Element<'a, Message> {
    use crate::git_panel::diff_view::text_diff::{self, TextDiffMessage};
    let close_control = close_message.as_ref().map(|_| {
        crate::git_panel::ui::header_plain_glyph_button("✕", "关闭".to_string(), Message::None)
            .map(|_| TextDiffMessage::Close)
    });
    text_diff::view_custom_text_diff(
        &super::diff_view_state(app),
        &super::diff_render_ctx(app),
        title,
        file,
        old_content,
        new_content,
        close_control,
        effective_theme,
        bg_default,
        add_line_bg,
        add_word_bg,
        del_line_bg,
        del_word_bg,
    )
    .map(move |event| match event {
        TextDiffMessage::Diff(event) => super::map_diff_message(event),
        TextDiffMessage::Close => close_message.clone().unwrap_or(Message::None),
    })
}
