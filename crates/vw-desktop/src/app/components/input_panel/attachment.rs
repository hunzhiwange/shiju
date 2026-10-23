//! 附件按钮组件模块
//!
//! 该模块提供输入面板中的附件功能，允许用户直接选择本地文件或图片附件。
//! 主要组件包括：
//! - 附件按钮：显示一个 "+" 图标按钮
//! - 附件条：显示当前已选择的本地附件
//! - 工具提示：鼠标悬停时显示"附件"提示

use std::borrow::Cow;
use std::path::Path;

use iced::alignment::{Horizontal, Vertical};
use iced::widget::image::{Handle as ImageHandle, Image};
use iced::widget::scrollable::{Direction, Scrollbar};
use iced::widget::tooltip::Position;
use iced::widget::{Space, button, container, rich_text, row, scrollable, span, text, tooltip};
use iced::{Color, ContentFit, Element, Length, Theme};

use crate::app::assets::Icon;
use crate::app::components::input_panel::icons::icon_svg;
use crate::app::components::input_panel::styles::{
    BOTTOM_BAR_ICON_BUTTON_SIZE, BOTTOM_BAR_ICON_SIZE, BOTTOM_BAR_LARGE_ICON_SIZE,
    round_icon_button_style, tooltip_dark_style,
};
use crate::app::{App, Message, message};

const ATTACH_BUTTON_SIZE: f32 = BOTTOM_BAR_ICON_BUTTON_SIZE;
const ATTACH_ICON_SIZE: f32 = BOTTOM_BAR_LARGE_ICON_SIZE;
const ATTACHMENT_NAME_MAX_CHARS: usize = 20;
const ATTACHMENT_NAME_ELLIPSIS: &str = "...";

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum AttachmentDisplayKind {
    Image,
    Document,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AttachmentDisplayItem {
    pub(crate) path: String,
    pub(crate) display_name: Option<String>,
    pub(crate) kind: AttachmentDisplayKind,
}

impl AttachmentDisplayItem {
    pub(crate) fn from_local_path(path: impl Into<String>) -> Self {
        let path = path.into();
        let kind = if is_supported_image_attachment(&path) {
            AttachmentDisplayKind::Image
        } else {
            AttachmentDisplayKind::Document
        };
        Self { path, display_name: None, kind }
    }
}

fn is_supported_image_attachment(path: &str) -> bool {
    Path::new(path)
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| {
            matches!(
                ext.to_ascii_lowercase().as_str(),
                "png" | "jpg" | "jpeg" | "webp" | "gif" | "bmp"
            )
        })
        .unwrap_or(false)
}

fn chip_style(theme: &Theme) -> iced::widget::container::Style {
    let palette = theme.extended_palette();
    iced::widget::container::Style {
        background: Some(iced::Background::Color(palette.background.weak.color.scale_alpha(0.78))),
        border: iced::Border {
            radius: 10.0.into(),
            width: 1.0,
            color: palette.background.strong.color.scale_alpha(0.9),
        },
        text_color: Some(theme.palette().text),
        ..Default::default()
    }
}

fn chip_thumb_style(theme: &Theme) -> iced::widget::container::Style {
    let palette = theme.extended_palette();
    iced::widget::container::Style {
        background: Some(iced::Background::Color(
            palette.background.strong.color.scale_alpha(0.72),
        )),
        border: iced::Border {
            radius: 8.0.into(),
            width: 1.0,
            color: palette.background.strong.color.scale_alpha(0.96),
        },
        ..Default::default()
    }
}

fn chip_close_style(
    theme: &Theme,
    status: iced::widget::button::Status,
) -> iced::widget::button::Style {
    let background = match status {
        iced::widget::button::Status::Hovered => Some(iced::Background::Color(
            theme.extended_palette().background.strong.color.scale_alpha(0.42),
        )),
        _ => None,
    };

    iced::widget::button::Style {
        background,
        border: iced::Border { radius: 999.0.into(), width: 0.0, color: Color::TRANSPARENT },
        text_color: theme.palette().text.scale_alpha(0.78),
        ..Default::default()
    }
}

/// 创建附件按钮组件
///
/// 该函数构建一个直接打开本地文件选择器的附件按钮。
///
/// # 参数
///
/// - `app` - 应用状态引用，用于访问：
///   - `app.files` - 当前已选择的附件列表
///   - `app.multimodal_settings` - 图片附件数量与大小限制
///
/// # 返回值
///
/// 返回一个带工具提示的附件按钮。
///
/// # 示例
///
/// ```rust,ignore
/// let attachment_btn = attachment_button(&app);
/// // 在 UI 布局中使用该按钮
/// column![attachment_btn, /* 其他组件 */]
/// ```
pub fn attachment_button(app: &App) -> Element<'_, Message> {
    let image_count = app.files.iter().filter(|path| is_supported_image_attachment(path)).count();
    let max_images = app.multimodal_settings.max_images.clamp(1, 16);

    let attach_btn = button(
        container(icon_svg(Icon::Plus, ATTACH_ICON_SIZE))
            .width(Length::Fixed(ATTACH_BUTTON_SIZE))
            .height(Length::Fixed(ATTACH_BUTTON_SIZE))
            .align_x(Horizontal::Center)
            .align_y(Vertical::Center),
    )
    .padding(0)
    .style(|theme: &Theme, status| round_icon_button_style(theme, status, true))
    .on_press(Message::Project(message::ProjectMessage::AttachmentFilesPick));

    let attach_tip = container(
        iced::widget::column![
            text("附件")
                .size(12)
                .style(|_theme: &Theme| iced::widget::text::Style { color: Some(Color::WHITE) }),
            text(format!(
                "本地文件或截图粘贴，已选 {} 个，图片 {}/{}，单张最多 {} MB",
                app.files.len(),
                image_count,
                max_images,
                app.multimodal_settings.max_image_size_mb.clamp(1, 20)
            ))
            .size(11)
            .style(|_theme: &Theme| iced::widget::text::Style {
                color: Some(Color::WHITE.scale_alpha(0.78)),
            }),
        ]
        .spacing(2),
    )
    .style(tooltip_dark_style)
    .padding([6, 8]);

    tooltip(attach_btn, attach_tip, Position::Top).into()
}

fn attachment_thumbnail(path: String, kind: AttachmentDisplayKind) -> Element<'static, Message> {
    match kind {
        AttachmentDisplayKind::Image if Path::new(&path).exists() => container(
            Image::new(ImageHandle::from_path(path))
                .width(Length::Fixed(28.0))
                .height(Length::Fixed(28.0))
                .content_fit(ContentFit::Cover),
        )
        .width(Length::Fixed(28.0))
        .height(Length::Fixed(28.0))
        .style(chip_thumb_style)
        .into(),
        _ => {
            let icon = match kind {
                AttachmentDisplayKind::Image => Icon::Image,
                AttachmentDisplayKind::Document => Icon::FileText,
            };
            container(icon_svg(icon, BOTTOM_BAR_ICON_SIZE))
                .width(Length::Fixed(28.0))
                .height(Length::Fixed(28.0))
                .align_x(Horizontal::Center)
                .align_y(Vertical::Center)
                .style(chip_thumb_style)
                .into()
        }
    }
}

fn split_attachment_name_extension(name: &str) -> Option<(&str, &str)> {
    let dot_index = name.rfind('.')?;
    if dot_index == 0 || dot_index + 1 >= name.len() {
        return None;
    }

    Some((&name[..dot_index], &name[dot_index..]))
}

fn truncate_attachment_name_middle(name: &str, max_chars: usize) -> String {
    let total_chars = name.chars().count();
    if total_chars <= max_chars {
        return name.to_string();
    }

    let ellipsis_chars = ATTACHMENT_NAME_ELLIPSIS.chars().count();
    if max_chars <= ellipsis_chars {
        return name.chars().take(max_chars).collect();
    }

    let available_chars = max_chars - ellipsis_chars;
    let max_tail_chars = available_chars.saturating_sub(2);
    let tail_chars = split_attachment_name_extension(name)
        .map(|(stem, extension)| {
            let extension_chars = extension.chars().count();
            let desired_tail_chars = extension_chars + stem.chars().count().min(3);
            desired_tail_chars.max(extension_chars).min(max_tail_chars)
        })
        .filter(|tail_chars| *tail_chars > 0)
        .unwrap_or(available_chars / 2);
    let head_chars = available_chars.saturating_sub(tail_chars);

    let head = name.chars().take(head_chars).collect::<String>();
    let tail = name.chars().skip(total_chars.saturating_sub(tail_chars)).collect::<String>();

    format!("{head}{ATTACHMENT_NAME_ELLIPSIS}{tail}")
}

fn open_attachment_message(path: String) -> Message {
    Message::Preview(message::PreviewMessage::Open(path))
}

fn attachment_chip(
    item: AttachmentDisplayItem,
    remove_message: Option<Message>,
) -> Element<'static, Message> {
    let AttachmentDisplayItem { path, display_name, kind } = item;
    let name = display_name.unwrap_or_else(|| {
        Path::new(&path)
            .file_name()
            .and_then(|file| file.to_str())
            .map(ToOwned::to_owned)
            .unwrap_or_else(|| path.clone())
    });
    let truncated_name = truncate_attachment_name_middle(&name, ATTACHMENT_NAME_MAX_CHARS);
    let label = match kind {
        AttachmentDisplayKind::Image => "图片",
        AttachmentDisplayKind::Document => "文件",
    };
    let open_path = path.clone();
    let name_view: Element<'static, Message> =
        rich_text([span(truncated_name).underline(true).link(open_path)])
            .size(12)
            .style(|theme: &Theme| iced::widget::text::Style {
                color: Some(theme.extended_palette().primary.base.color.scale_alpha(0.92)),
            })
            .on_link_click(open_attachment_message)
            .into();

    let mut chip_row = row![
        attachment_thumbnail(path.clone(), kind.clone()),
        text(label).size(11).style(|theme: &Theme| iced::widget::text::Style {
            color: Some(theme.palette().primary.scale_alpha(0.92)),
        }),
        name_view,
    ]
    .spacing(8)
    .align_y(iced::Alignment::Center);

    if let Some(remove_message) = remove_message {
        let remove = button(
            container(icon_svg(Icon::X, BOTTOM_BAR_ICON_SIZE))
                .width(Length::Fixed(16.0))
                .height(Length::Fixed(16.0))
                .align_x(Horizontal::Center)
                .align_y(Vertical::Center),
        )
        .padding(0)
        .style(chip_close_style)
        .on_press(remove_message);
        chip_row = chip_row.push(remove);
    }

    let content = container(chip_row).padding([6, 10]).style(chip_style);

    let tip = container(
        text(path)
            .size(12)
            .style(|_theme: &Theme| iced::widget::text::Style { color: Some(Color::WHITE) }),
    )
    .style(tooltip_dark_style)
    .padding([6, 8]);

    tooltip(content, tip, Position::Top).into()
}

pub(crate) fn attachment_preview_strip(
    items: Vec<AttachmentDisplayItem>,
) -> Element<'static, Message> {
    if items.is_empty() {
        return Space::new().height(Length::Fixed(0.0)).into();
    }

    let mut chips = row![].spacing(8).align_y(iced::Alignment::Center);
    for item in items {
        chips = chips.push(attachment_chip(item, None));
    }

    scrollable(chips)
        .direction(Direction::Horizontal(Scrollbar::new().width(4).scroller_width(4)))
        .height(Length::Shrink)
        .into()
}

#[cfg(test)]
#[path = "attachment_tests.rs"]
mod tests;

// Most rendered messages have no attachments or whitespace to normalize. Borrow
// their text instead of allocating several full transcript copies per frame.
fn normalize_attachment_text(content: &str) -> Cow<'_, str> {
    let trimmed = content.trim();
    if !trimmed.contains("\r\n") && trimmed.lines().all(|line| line.len() == line.trim_end().len())
    {
        return Cow::Borrowed(trimmed);
    }
    let mut normalized = String::with_capacity(trimmed.len());
    for (index, line) in trimmed.lines().enumerate() {
        if index != 0 {
            normalized.push('\n');
        }
        normalized.push_str(line.trim_end());
    }
    Cow::Owned(normalized)
}

pub(crate) fn parse_attachment_markers(
    content: &str,
) -> (Cow<'_, str>, Vec<AttachmentDisplayItem>) {
    if !content.contains('[')
        || !(content.contains("IMAGE:")
            || content.contains("DOCUMENT:")
            || content.contains("Document:"))
    {
        return (normalize_attachment_text(content), Vec::new());
    }
    let mut attachments = Vec::new();
    let mut cleaned = String::with_capacity(content.len());
    let mut cursor = 0usize;

    while cursor < content.len() {
        let remaining = &content[cursor..];
        let Some(rel_start) = remaining.find('[') else {
            cleaned.push_str(remaining);
            break;
        };

        let start = cursor + rel_start;
        cleaned.push_str(&content[cursor..start]);

        let marker_start = start + 1;
        let Some(rel_end) = content[marker_start..].find(']') else {
            cleaned.push_str(&content[start..]);
            break;
        };

        let marker_end = marker_start + rel_end;
        let marker = content[marker_start..marker_end].trim();

        if let Some(path) = marker.strip_prefix("IMAGE:").map(str::trim) {
            if !path.is_empty() {
                attachments.push(AttachmentDisplayItem {
                    path: path.to_string(),
                    display_name: None,
                    kind: AttachmentDisplayKind::Image,
                });
                cursor = marker_end + 1;
                continue;
            }
        }

        if let Some(path) = marker.strip_prefix("DOCUMENT:").map(str::trim) {
            if !path.is_empty() {
                attachments.push(AttachmentDisplayItem {
                    path: path.to_string(),
                    display_name: None,
                    kind: AttachmentDisplayKind::Document,
                });
                cursor = marker_end + 1;
                continue;
            }
        }

        if let Some(name) = marker.strip_prefix("Document:").map(str::trim) {
            let mut path_cursor = marker_end + 1;
            while path_cursor < content.len() {
                let ch = content[path_cursor..].chars().next().unwrap_or('\n');
                if !ch.is_whitespace() || ch == '\n' || ch == '\r' {
                    break;
                }
                path_cursor += ch.len_utf8();
            }

            let path_start = path_cursor;
            while path_cursor < content.len() {
                let ch = content[path_cursor..].chars().next().unwrap_or('\n');
                if ch == '\n' || ch == '\r' {
                    break;
                }
                path_cursor += ch.len_utf8();
            }

            let path = content[path_start..path_cursor].trim();
            if !path.is_empty() {
                attachments.push(AttachmentDisplayItem {
                    path: path.to_string(),
                    display_name: (!name.is_empty()).then(|| name.to_string()),
                    kind: AttachmentDisplayKind::Document,
                });
                cursor = path_cursor;
                continue;
            }
        }

        cleaned.push_str(&content[start..=marker_end]);
        cursor = marker_end + 1;
    }

    if attachments.is_empty() {
        return (normalize_attachment_text(content), attachments);
    }
    (Cow::Owned(normalize_attachment_text(&cleaned).into_owned()), attachments)
}

pub fn attachment_strip(app: &App) -> Element<'_, Message> {
    if app.files.is_empty() {
        return Space::new().height(Length::Fixed(0.0)).into();
    }

    let mut chips = row![].spacing(8).align_y(iced::Alignment::Center);
    for path in &app.files {
        let item = AttachmentDisplayItem::from_local_path(path.clone());
        chips = chips.push(attachment_chip(
            item,
            Some(Message::Project(message::ProjectMessage::RemoveAttachedFile(path.to_string()))),
        ));
    }

    scrollable(chips)
        .direction(Direction::Horizontal(Scrollbar::new().width(4).scroller_width(4)))
        .height(Length::Shrink)
        .into()
}
