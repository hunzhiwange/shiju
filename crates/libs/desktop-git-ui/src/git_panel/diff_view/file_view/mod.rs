//! File content rendering; desktop composes the file header.
use iced::widget::{Space, button, container, text};
use iced::{Border, Element, Length};
use super::DiffRenderCtx;
use crate::git_panel::utils::{FileStatus, Lang, lang_for_file};
pub mod added_deleted;
pub mod body;
pub mod gaps;
pub mod hunk_ops;
pub mod standard;
pub mod start_end;
const DIFF_LINE_SELECT_WIDTH: f32 = 18.0;

pub fn diff_line_select_button<Message: Clone + 'static>(
    selected: bool,
    on: Message,
    read_only: bool,
) -> Element<'static, Message> {
    if read_only {
        // 分栏 diff 的行布局依赖控件的固有高度；隐藏暂存框时仍保留行高。
        return container(Space::new().height(20)).width(DIFF_LINE_SELECT_WIDTH).into();
    }
    let glyph = if selected { "✓" } else { "" };
    button(text(glyph).size(12))
        .on_press(on)
        .padding([2, 4])
        .width(Length::Fixed(DIFF_LINE_SELECT_WIDTH))
        .style(|theme: &iced::Theme, _status| iced::widget::button::Style {
            background: None,
            border: Border::default(),
            text_color: theme.extended_palette().background.strong.text,
            ..Default::default()
        })
        .into()
}

pub fn diff_line_select_spacer<Message: 'static>() -> Element<'static, Message> {
    container(Space::new().width(Length::Fill).height(Length::Shrink))
        .width(Length::Fixed(DIFF_LINE_SELECT_WIDTH))
        .into()
}


#[cfg(test)]
mod added_deleted_tests;

#[cfg(test)]
mod gaps_tests;

#[cfg(test)]
mod standard_tests;

#[cfg(test)]
mod start_end_tests;
