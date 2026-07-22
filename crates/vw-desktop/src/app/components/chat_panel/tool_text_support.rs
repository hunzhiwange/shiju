//! 提供聊天工具文本选择与只读编辑器支持。
//! 该模块统一长文本展示、选择颜色和复制目标，确保暗色主题下仍可读。

use iced::widget::text::Wrapping;
use iced::widget::{container, text, text_editor};
use iced::{Background, Border, Color, Element, Length, Theme, highlighter};

use crate::app::{App, Message, message};

use super::selectable_text::{SelectableTextPolicy, selectable_text_policy};
use super::utils::truncate_lines_middle;

/// ToolTextTarget 描述该模块对外暴露的离散状态。
#[derive(Clone, Copy)]
pub enum ToolTextTarget {
    SpecialMessageText { msg_idx: usize, text_idx: usize },
    ToolCardText { msg_idx: usize, tool_idx: usize, text_idx: usize },
}

/// 执行 tool_text_key 对应的模块功能。
///
/// 参数含义保持与函数签名一致；返回值用于调用方继续组合处理，错误由返回类型显式表达。
pub fn tool_text_key(msg_idx: usize, tool_idx: usize, text_idx: usize) -> u128 {
    ((msg_idx as u128) << 64) | ((tool_idx as u128) << 32) | (text_idx as u128)
}

fn is_dark_theme(theme: &Theme) -> bool {
    theme.palette().background.r + theme.palette().background.g + theme.palette().background.b < 1.5
}

fn selected_text(content: &text_editor::Content) -> Option<String> {
    content.selection().filter(|selection| !selection.is_empty())
}

pub(super) fn message_target_from_context(target: u64) -> (usize, Option<usize>) {
    let msg_idx = (target >> 32) as usize;
    let sub_idx = (target & 0xFFFF_FFFF) as usize;

    if sub_idx == 0 { (msg_idx, None) } else { (msg_idx, Some(sub_idx - 1)) }
}

/// 执行 chat_text_line_height 对应的模块功能。
///
/// 参数含义保持与函数签名一致；返回值用于调用方继续组合处理，错误由返回类型显式表达。
pub fn chat_text_line_height() -> iced::widget::text::LineHeight {
    iced::widget::text::LineHeight::Relative(1.6)
}

/// 执行 chat_text_font_name 对应的模块功能。
///
/// 参数含义保持与函数签名一致；返回值用于调用方继续组合处理，错误由返回类型显式表达。
pub fn chat_text_font_name() -> &'static str {
    #[cfg(target_os = "macos")]
    {
        "PingFang SC"
    }

    #[cfg(not(target_os = "macos"))]
    {
        "Noto Sans CJK SC"
    }
}

/// 执行 chat_text_font 对应的模块功能。
///
/// 参数含义保持与函数签名一致；返回值用于调用方继续组合处理，错误由返回类型显式表达。
pub fn chat_text_font() -> iced::Font {
    iced::Font::with_name(chat_text_font_name())
}

/// 执行 chat_text_selection_color 对应的模块功能。
///
/// 参数含义保持与函数签名一致；返回值用于调用方继续组合处理，错误由返回类型显式表达。
pub fn chat_text_selection_color(theme: &Theme) -> Color {
    if is_dark_theme(theme) {
        Color::from_rgba8(0x8B, 0x93, 0x9C, 0.34)
    } else {
        Color::from_rgba8(0xD9, 0xDE, 0xE5, 0.92)
    }
}

/// 执行 read_only_text_style 对应的模块功能。
///
/// 参数含义保持与函数签名一致；返回值用于调用方继续组合处理，错误由返回类型显式表达。
pub fn read_only_text_style(theme: &Theme, value: Color) -> iced::widget::text_editor::Style {
    iced::widget::text_editor::Style {
        background: Background::Color(Color::TRANSPARENT),
        border: Border { width: 0.0, color: Color::TRANSPARENT, radius: 0.0.into() },
        value,
        selection: chat_text_selection_color(theme),
        placeholder: value.scale_alpha(0.7),
    }
}

/// 执行 selected_chat_text_for_message 对应的模块功能。
///
/// 参数含义保持与函数签名一致；返回值用于调用方继续组合处理，错误由返回类型显式表达。
pub fn selected_chat_text_for_message(app: &App, msg_idx: usize) -> Option<String> {
    app.chat_message_editors
        .get(msg_idx)
        .and_then(selected_text)
        .or_else(|| {
            app.chat_markdown_block_editors
                .iter()
                .filter(|(key, _)| ((**key) >> 64) as usize == msg_idx)
                .find_map(|(_, content)| selected_text(content))
        })
        .or_else(|| {
            app.chat_special_text_editors
                .iter()
                .filter(|(key, _)| ((**key) >> 32) as usize == msg_idx)
                .find_map(|(_, content)| selected_text(content))
        })
        .or_else(|| {
            app.chat_tool_text_editors
                .iter()
                .filter(|(key, _)| ((**key) >> 64) as usize == msg_idx)
                .find_map(|(_, content)| selected_text(content))
        })
        .or_else(|| {
            app.chat_think_editors
                .iter()
                .filter(|(key, _)| ((**key) >> 32) as usize == msg_idx)
                .find_map(|(_, content)| selected_text(content))
        })
}

/// 执行 selected_chat_text_for_tool 对应的模块功能。
///
/// 参数含义保持与函数签名一致；返回值用于调用方继续组合处理，错误由返回类型显式表达。
pub fn selected_chat_text_for_tool(app: &App, msg_idx: usize, tool_idx: usize) -> Option<String> {
    app.chat_tool_text_editors
        .iter()
        .filter(|(key, _)| {
            ((**key) >> 64) as usize == msg_idx && (((**key) >> 32) as u32) as usize == tool_idx
        })
        .find_map(|(_, content)| selected_text(content))
}

/// 执行 selected_chat_text_for_target 对应的模块功能。
///
/// 参数含义保持与函数签名一致；返回值用于调用方继续组合处理，错误由返回类型显式表达。
pub fn selected_chat_text_for_target(app: &App, target: u64) -> Option<String> {
    let (msg_idx, sub_idx) = message_target_from_context(target);

    match sub_idx {
        Some(tool_idx) => selected_chat_text_for_tool(app, msg_idx, tool_idx),
        None => selected_chat_text_for_message(app, msg_idx),
    }
}

/// 执行 tool_text_style 对应的模块功能。
///
/// 参数含义保持与函数签名一致；返回值用于调用方继续组合处理，错误由返回类型显式表达。
pub fn tool_text_style(
    theme: &Theme,
    _status: text_editor::Status,
) -> iced::widget::text_editor::Style {
    tool_text_style_with_danger(theme, _status, false)
}

/// 执行 tool_text_style_with_danger 对应的模块功能。
///
/// 参数含义保持与函数签名一致；返回值用于调用方继续组合处理，错误由返回类型显式表达。
pub fn tool_text_style_with_danger(
    theme: &Theme,
    _status: text_editor::Status,
    danger: bool,
) -> iced::widget::text_editor::Style {
    let value = theme.palette().text;
    let value = if danger { theme.extended_palette().danger.base.color } else { value };
    read_only_text_style(theme, value)
}

const SAFE_EDITOR_MAX_CHARS: usize = 20_000;
const SAFE_EDITOR_MAX_LINE_CHARS: usize = 2_000;
const PREVIEW_MAX_LINES: usize = 120;
const PREVIEW_MAX_LINE_CHARS: usize = 500;

/// 执行 is_safe_for_text_editor 对应的模块功能。
///
/// 参数含义保持与函数签名一致；返回值用于调用方继续组合处理，错误由返回类型显式表达。
pub fn is_safe_for_text_editor(content: &str) -> bool {
    if content.len() > SAFE_EDITOR_MAX_CHARS {
        return false;
    }

    content.lines().map(|line| line.chars().count()).max().unwrap_or(0)
        <= SAFE_EDITOR_MAX_LINE_CHARS
}

fn preview_text<'a>(
    content: &str,
    font_name: &'static str,
    size: f32,
    color: Color,
) -> Element<'a, Message> {
    let preview = truncate_lines_middle(content, PREVIEW_MAX_LINES, PREVIEW_MAX_LINE_CHARS);

    container(
        text(preview)
            .size(size)
            .font(iced::Font::with_name(font_name))
            .line_height(chat_text_line_height())
            .wrapping(Wrapping::Word)
            .style(move |_theme| iced::widget::text::Style { color: Some(color) }),
    )
    .width(Length::Fill)
    .into()
}

/// 执行 tool_text_editor 对应的模块功能。
///
/// 参数含义保持与函数签名一致；返回值用于调用方继续组合处理，错误由返回类型显式表达。
pub fn tool_text_editor<'a>(
    app: &'a App,
    target: ToolTextTarget,
    font_name: &'static str,
    size: f32,
    enable_highlight: bool,
    danger: bool,
) -> Option<Element<'a, Message>> {
    match target {
        ToolTextTarget::SpecialMessageText { msg_idx, text_idx } => {
            let editor_content = app
                .chat_special_text_editors
                .get(&(((msg_idx as u64) << 32) | (text_idx as u64)))?;
            let content_text = editor_content.text();

            if selectable_text_policy(&content_text, false)
                == SelectableTextPolicy::PreviewWithCopyFallback
            {
                let theme = app.theme();
                let color = if danger {
                    theme.extended_palette().danger.base.color
                } else {
                    theme.palette().text
                };
                return Some(preview_text(&content_text, font_name, size, color));
            }

            let editor = text_editor(editor_content)
                .on_action(move |a| {
                    Message::Chat(message::ChatMessage::SpecialTextEditorAction(
                        msg_idx, text_idx, a,
                    ))
                })
                .size(size)
                .line_height(chat_text_line_height())
                .padding(0)
                .height(Length::Shrink)
                .font(iced::Font::with_name(font_name))
                .style(move |theme, status| tool_text_style_with_danger(theme, status, danger));

            Some(if enable_highlight {
                let highlight_theme = if is_dark_theme(&app.theme()) {
                    highlighter::Theme::Base16Ocean
                } else {
                    highlighter::Theme::InspiredGitHub
                };
                container(editor.highlight("markdown", highlight_theme)).width(Length::Fill).into()
            } else {
                container(editor).width(Length::Fill).into()
            })
        }
        ToolTextTarget::ToolCardText { msg_idx, tool_idx, text_idx } => {
            let editor_content =
                app.chat_tool_text_editors.get(&tool_text_key(msg_idx, tool_idx, text_idx))?;
            let content_text = editor_content.text();

            if selectable_text_policy(&content_text, false)
                == SelectableTextPolicy::PreviewWithCopyFallback
            {
                let theme = app.theme();
                let color = if danger {
                    theme.extended_palette().danger.base.color
                } else {
                    theme.palette().text
                };
                return Some(preview_text(&content_text, font_name, size, color));
            }

            let editor = text_editor(editor_content)
                .on_action(move |a| {
                    Message::Chat(message::ChatMessage::ToolTextEditorAction(
                        msg_idx, tool_idx, text_idx, a,
                    ))
                })
                .size(size)
                .line_height(chat_text_line_height())
                .padding(0)
                .height(Length::Shrink)
                .font(iced::Font::with_name(font_name))
                .style(move |theme, status| tool_text_style_with_danger(theme, status, danger));

            Some(if enable_highlight {
                let highlight_theme = if is_dark_theme(&app.theme()) {
                    highlighter::Theme::Base16Ocean
                } else {
                    highlighter::Theme::InspiredGitHub
                };
                container(editor.highlight("markdown", highlight_theme)).width(Length::Fill).into()
            } else {
                container(editor).width(Length::Fill).into()
            })
        }
    }
}

/// 执行 tool_inline_text_editor 对应的模块功能。
///
/// 参数含义保持与函数签名一致；返回值用于调用方继续组合处理，错误由返回类型显式表达。
pub fn tool_inline_text_editor<'a, F>(
    app: &'a App,
    target: ToolTextTarget,
    font_name: &'static str,
    size: f32,
    color_fn: F,
) -> Option<Element<'a, Message>>
where
    F: Fn(&Theme) -> Color + Copy + 'a,
{
    match target {
        ToolTextTarget::SpecialMessageText { msg_idx, text_idx } => {
            let editor_content = app
                .chat_special_text_editors
                .get(&(((msg_idx as u64) << 32) | (text_idx as u64)))?;
            let content_text = editor_content.text();

            if selectable_text_policy(&content_text, false)
                == SelectableTextPolicy::PreviewWithCopyFallback
            {
                let theme = app.theme();
                return Some(preview_text(&content_text, font_name, size, color_fn(&theme)));
            }

            let font = iced::Font::with_name(font_name);
            let inline_text = text(content_text)
                .size(size)
                .font(font)
                .wrapping(Wrapping::None)
                .style(move |theme| iced::widget::text::Style { color: Some(color_fn(theme)) });

            Some(container(inline_text).width(Length::Shrink).into())
        }
        ToolTextTarget::ToolCardText { msg_idx, tool_idx, text_idx } => {
            let editor_content =
                app.chat_tool_text_editors.get(&tool_text_key(msg_idx, tool_idx, text_idx))?;
            let content_text = editor_content.text();

            if selectable_text_policy(&content_text, false)
                == SelectableTextPolicy::PreviewWithCopyFallback
            {
                let theme = app.theme();
                return Some(preview_text(&content_text, font_name, size, color_fn(&theme)));
            }

            let font = iced::Font::with_name(font_name);
            let inline_text = text(content_text)
                .size(size)
                .font(font)
                .wrapping(Wrapping::None)
                .style(move |theme| iced::widget::text::Style { color: Some(color_fn(theme)) });

            Some(container(inline_text).width(Length::Shrink).into())
        }
    }
}
