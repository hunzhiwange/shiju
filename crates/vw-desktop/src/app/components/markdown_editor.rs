//! Markdown 编辑器组件
//!
//! 本模块提供用于编辑和预览 Markdown 内容的 UI 组件与布局函数。
//!
//! 主要功能包括：
//! - 支持三种查看模式：编辑模式、预览模式和分屏模式
//! - 提供模式切换控件
//! - 集成代码高亮与滚动预览
//!
//! # 示例
//!
//! ```ignore
//! use crate::app::components::markdown_editor::{MarkdownViewMode, view, mode_switch};
//!
//! // 在应用中根据模式渲染视图
//! let element = view(&editor_content, &preview_content, &theme, &viewer, MarkdownViewMode::Edit, on_action);
//! let switch = mode_switch(current_mode, on_change);
//! ```

use iced::highlighter;
use iced::widget::scrollable::Direction;
use iced::widget::{button, container, markdown, row, scrollable, text, text_editor};
use iced::{Background, Border, Color, Element, Length, Shadow, Theme, Vector};

/// Markdown 查看模式
///
/// 定义了编辑器内容的三种展示方式：
/// - `Edit`: 纯编辑模式，仅显示文本编辑器
/// - `Preview`: 纯预览模式，仅显示渲染后的 Markdown
/// - `Split`: 分屏模式，同时显示编辑器和预览
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MarkdownViewMode {
    /// 编辑模式：仅显示文本编辑器
    Edit,
    /// 预览模式：仅显示渲染后的 Markdown
    Preview,
    /// 分屏模式：同时显示编辑器和预览
    Split,
}

fn is_dark_theme(theme: &Theme) -> bool {
    theme.palette().background.r + theme.palette().background.g + theme.palette().background.b < 1.5
}

fn mode_button_style(
    theme: &Theme,
    status: iced::widget::button::Status,
    selected: bool,
) -> iced::widget::button::Style {
    let palette = theme.extended_palette();
    let is_dark = is_dark_theme(theme);

    let background = if selected {
        match status {
            iced::widget::button::Status::Hovered => {
                Some(Background::Color(theme.palette().primary.scale_alpha(if is_dark {
                    0.24
                } else {
                    0.16
                })))
            }
            iced::widget::button::Status::Pressed => {
                Some(Background::Color(theme.palette().primary.scale_alpha(if is_dark {
                    0.30
                } else {
                    0.20
                })))
            }
            _ => Some(Background::Color(theme.palette().primary.scale_alpha(if is_dark {
                0.18
            } else {
                0.12
            }))),
        }
    } else {
        match status {
            iced::widget::button::Status::Hovered => Some(Background::Color(if is_dark {
                palette.background.weak.color.scale_alpha(0.82)
            } else {
                Color::WHITE.scale_alpha(0.92)
            })),
            iced::widget::button::Status::Pressed => Some(Background::Color(if is_dark {
                palette.background.strong.color.scale_alpha(0.88)
            } else {
                palette.background.weak.color.scale_alpha(0.94)
            })),
            _ => Some(Background::Color(Color::TRANSPARENT)),
        }
    };

    iced::widget::button::Style {
        background,
        text_color: if selected {
            theme.palette().primary
        } else {
            theme.palette().text.scale_alpha(0.86)
        },
        border: Border {
            radius: 999.0.into(),
            width: 1.0,
            color: if selected {
                theme.palette().primary.scale_alpha(if is_dark { 0.44 } else { 0.28 })
            } else if matches!(status, iced::widget::button::Status::Hovered) {
                palette.background.strong.color.scale_alpha(if is_dark { 0.72 } else { 0.16 })
            } else {
                Color::TRANSPARENT
            },
        },
        shadow: if selected {
            Shadow {
                color: theme.palette().primary.scale_alpha(if is_dark { 0.16 } else { 0.08 }),
                offset: Vector::new(0.0, 6.0),
                blur_radius: 16.0,
            }
        } else {
            Shadow::default()
        },
        ..Default::default()
    }
}

/// 构建模式切换控件
///
/// 创建一个水平排列的按钮组，用于在编辑、预览和分屏三种模式间切换。
///
/// # 类型参数
/// - `Message`: 应用程序消息类型，必须实现 `Clone` 和 `'a`
///
/// # 参数
/// - `mode`: 当前激活的视图模式
/// - `on_change`: 回调函数，用于将新的模式转换为应用程序消息
///
/// # 返回值
/// 返回包含三个模式切换按钮的 UI 元素
///
/// # 示例
/// ```ignore
/// let switch = mode_switch(MarkdownViewMode::Edit, |m| Message::ChangeMode(m));
/// ```
pub fn mode_switch<'a, Message: Clone + 'a>(
    mode: MarkdownViewMode,
    on_change: fn(MarkdownViewMode) -> Message,
) -> Element<'a, Message> {
    // 内部辅助函数，用于创建单个模式切换按钮
    let btn = |label: &'static str, target: MarkdownViewMode| {
        let selected = mode == target;
        let b = button(text(label).size(12)).padding([6, 12]);
        if selected {
            // 当前选中的模式使用主要样式，不可点击
            b.style(move |theme, status| mode_button_style(theme, status, selected))
        } else {
            // 非选中模式使用次要样式，点击触发模式切换
            b.style(move |theme, status| mode_button_style(theme, status, selected))
                .on_press(on_change(target))
        }
    };

    row![
        btn("编辑", MarkdownViewMode::Edit),
        btn("预览", MarkdownViewMode::Preview),
        btn("分屏", MarkdownViewMode::Split)
    ]
    .spacing(4)
    .into()
}

/// 渲染 Markdown 编辑器与预览视图
///
/// 根据指定的查看模式构建相应的 UI 布局：
/// - 编辑模式：仅显示代码编辑器
/// - 预览模式：仅显示渲染后的 Markdown 内容
/// - 分屏模式：左侧编辑器，右侧预览
///
/// # 类型参数
/// - `Message`: 应用程序消息类型
/// - `Viewer`: Markdown 渲染器类型，用于自定义渲染逻辑
///
/// # 参数
/// - `editor_content`: 编辑器内容引用
/// - `preview_content`: 预览内容引用
/// - `theme`: 当前主题引用
/// - `viewer`: Markdown 渲染器实例
/// - `mode`: 当前视图模式
/// - `on_editor_action`: 编辑器动作回调，用于将编辑器操作转换为应用程序消息
///
/// # 返回值
/// 返回对应模式的 UI 元素
///
/// # 示例
/// ```ignore
/// let element = view(
///     &editor_content,
///     &preview_content,
///     &theme,
///     &viewer,
///     MarkdownViewMode::Split,
///     |action| Message::EditorAction(action),
/// );
/// ```
pub fn view<'a, Message, Viewer>(
    editor_content: &'a text_editor::Content,
    preview_content: &'a markdown::Content,
    theme: &Theme,
    viewer: &'a Viewer,
    mode: MarkdownViewMode,
    on_editor_action: fn(text_editor::Action) -> Message,
) -> Element<'a, Message>
where
    Message: Clone + 'a,
    Viewer: markdown::Viewer<'a, Message> + 'a,
{
    // 创建文本编辑器组件，配置占位符、动作回调、高度、字体和语法高亮
    let is_dark =
        theme.palette().background.r + theme.palette().background.g + theme.palette().background.b
            < 1.5;
    let highlight_theme =
        if is_dark { highlighter::Theme::Base16Ocean } else { highlighter::Theme::InspiredGitHub };

    let editor = text_editor(editor_content)
        .on_action(on_editor_action)
        .height(Length::Fill)
        .padding(10)
        .font(iced::Font::with_name("Noto Sans CJK SC"))
        .highlight("markdown", highlight_theme);

    // 创建 Markdown 预览区域
    let preview = markdown::view_with(preview_content.items(), theme, viewer);
    let preview = scrollable(preview)
        .direction(Direction::Vertical(scrollable::Scrollbar::new().width(4).scroller_width(4)))
        .spacing(10)
        .width(Length::Fill)
        .height(Length::Fill);

    // 定义面板样式：背景色、边框宽度和颜色、圆角
    let panel_style = |theme: &Theme| iced::widget::container::Style {
        background: Some(theme.extended_palette().background.base.color.into()),
        border: Border {
            width: 1.0,
            color: theme.extended_palette().background.strong.color,
            radius: 10.0.into(),
        },
        ..Default::default()
    };

    match mode {
        // 编辑模式：仅包含编辑器
        MarkdownViewMode::Edit => {
            container(editor).width(Length::Fill).height(Length::Fill).style(panel_style).into()
        }
        // 预览模式：仅包含预览区域
        MarkdownViewMode::Preview => container(preview)
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(10)
            .style(panel_style)
            .into(),
        // 分屏模式：左侧编辑器，右侧预览
        MarkdownViewMode::Split => row![
            container(editor).width(Length::Fill).height(Length::Fill).style(panel_style),
            container(preview)
                .width(Length::Fill)
                .height(Length::Fill)
                .padding(10)
                .style(panel_style),
        ]
        .spacing(10)
        .height(Length::Fill)
        .into(),
    }
}

#[cfg(test)]
#[path = "markdown_editor_tests.rs"]
mod markdown_editor_tests;
