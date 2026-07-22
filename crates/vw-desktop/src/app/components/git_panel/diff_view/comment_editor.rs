//! Git diff 局部渲染辅助。
//!
//! 本模块负责 diff 行、行号、选区、上下文菜单和配色的局部组合。

use iced::widget::{Space, button, column, container, row, text};
/// 重新导出 use iced::{Background, Border, Color, Element, Length}，让上层模块通过稳定路径访问。
use iced::{Background, Border, Color, Element, Length};

/// 重新导出 use crate::app::{App, Message, message}，让上层模块通过稳定路径访问。
use crate::app::{App, Message, message};

pub(super) fn diff_comment_file_label(file: &str) -> String {
    std::path::Path::new(file)
        .file_name()
        .and_then(|name| name.to_str())
        .map(|name| {
            let truncated: String = name.chars().take(28).collect();
            if name.chars().count() > 28 { format!("{}…", truncated) } else { truncated }
        })
        .unwrap_or_else(|| file.to_string())
}

pub(super) fn diff_comment_range_label(start: usize, end: usize, is_old: bool) -> String {
    let start = start + 1;
    let end = end + 1;
    let side = if is_old { "旧行" } else { "新行" };
    if start == end {
        format!("正在评论 {} {}", side, start)
    } else {
        format!("正在评论 {} {}-{}", side, start, end)
    }
}

pub(super) fn diff_comment_editor_action(action: iced::widget::text_editor::Action) -> Message {
    Message::Git(message::GitMessage::DiffCommentEditorAction(action))
}

pub(super) fn diff_comment_cancel_button_style(
    theme: &iced::Theme,
    status: iced::widget::button::Status,
) -> iced::widget::button::Style {
    let palette = theme.extended_palette();
    let background = match status {
        iced::widget::button::Status::Hovered => palette.background.weak.color,
        iced::widget::button::Status::Pressed => palette.background.strong.color,
        _ => palette.background.base.color,
    };
    iced::widget::button::Style {
        background: Some(Background::Color(background)),
        text_color: theme.palette().text,
        border: Border { width: 1.0, color: palette.background.strong.color, radius: 10.0.into() },
        ..Default::default()
    }
}

pub(super) fn diff_comment_submit_button_style(
    theme: &iced::Theme,
    status: iced::widget::button::Status,
) -> iced::widget::button::Style {
    let palette = theme.palette();
    let background = match status {
        iced::widget::button::Status::Hovered => palette.primary.scale_alpha(0.92),
        iced::widget::button::Status::Pressed => palette.primary.scale_alpha(0.86),
        _ => palette.primary,
    };
    iced::widget::button::Style {
        background: Some(Background::Color(background)),
        text_color: Color::WHITE,
        border: Border { width: 0.0, color: Color::TRANSPARENT, radius: 10.0.into() },
        ..Default::default()
    }
}

pub(super) fn diff_comment_editor_card_style(
    theme: &iced::Theme,
) -> iced::widget::container::Style {
    iced::widget::container::Style {
        background: Some(theme.extended_palette().background.base.color.into()),
        border: Border {
            width: 1.0,
            color: theme.extended_palette().background.strong.color,
            radius: 12.0.into(),
        },
        shadow: iced::Shadow {
            color: Color::BLACK.scale_alpha(0.25),
            offset: iced::Vector::new(0.0, 8.0),
            blur_radius: 18.0,
        },
        ..Default::default()
    }
}

/// 处理 diff comment editor 对应的局部职责。
///
/// # 参数
///
/// 参数沿用调用点中的应用状态、工具输入或渲染上下文，不在这里扩大权限或补造状态。
///
/// # 返回值
///
/// `None` 表示输入缺少必要字段、当前状态不匹配，或该视图片段不需要展示。
///
/// # 错误处理
///
/// 本函数不吞掉底层错误；没有显式错误通道时，会用空集合、`None` 或现有 UI 状态表达不可用结果。
pub(crate) fn diff_comment_editor<'a>(app: &'a App) -> Option<Element<'a, Message>> {
    let draft = app.git_diff_comment_draft.as_ref()?;
    let file_label = diff_comment_file_label(&draft.range.file);
    let range_label =
        diff_comment_range_label(draft.range.start, draft.range.end, draft.range.is_old);

    let header = iced::widget::row![
        text("添加评论").size(15),
        // Space 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        Space::new().width(Length::Fill),
        text(file_label).size(12),
        text(range_label).size(12),
    ]
    .align_y(iced::Alignment::Center)
    .spacing(10);

    let editor = iced::widget::text_editor(&draft.editor)
        .placeholder("输入评论…")
        .on_action(diff_comment_editor_action)
        .height(Length::Fixed(92.0))
        .padding(10);

    let actions = iced::widget::row![
        Space::new().width(Length::Fill),
        button(text("取消"))
            .on_press(Message::Git(message::GitMessage::DiffCommentCancel))
            .style(diff_comment_cancel_button_style)
            .padding([6, 10]),
        button(text("评论"))
            .on_press(Message::Git(message::GitMessage::DiffCommentSubmit))
            .style(diff_comment_submit_button_style)
            .padding([6, 10]),
    ]
    .align_y(iced::Alignment::Center)
    .spacing(10);

    let editor_card = container(column![header, editor, actions].spacing(10))
        .padding(12)
        .width(Length::Fixed(520.0))
        .style(diff_comment_editor_card_style);

    Some(
        row![Space::new().width(Length::Fill), editor_card, Space::new().width(Length::Fill)]
            .width(Length::Fill)
            .into(),
    )
}
