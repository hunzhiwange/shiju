//! Git diff 局部渲染辅助。
//!
//! 本模块负责 diff 行、行号、选区、上下文菜单和配色的局部组合。

use iced::widget::{MouseArea, Space, container};
/// 重新导出 use iced::{Background, Border, Color, Element, Length}，让上层模块通过稳定路径访问。
use iced::{Background, Border, Color, Element, Length};

/// 重新导出 use crate::app::{App, Message, message}，让上层模块通过稳定路径访问。
use crate::app::{App, Message, message};

/// 重新导出 use super::{DIFF_SPLIT_DIVIDER_WIDTH, DiffSplitPaneTone, markers}，让上层模块通过稳定路径访问。
use super::{DIFF_SPLIT_DIVIDER_WIDTH, DiffSplitPaneTone, markers};

/// 处理 is dark theme 对应的局部职责。
///
/// # 参数
///
/// 参数沿用调用点中的应用状态、工具输入或渲染上下文，不在这里扩大权限或补造状态。
///
/// # 返回值
///
/// `true` 表示当前输入满足该辅助函数描述的条件。
///
/// # 错误处理
///
/// 本函数不吞掉底层错误；没有显式错误通道时，会用空集合、`None` 或现有 UI 状态表达不可用结果。
pub(super) fn is_dark_theme(theme: &iced::Theme) -> bool {
    theme.palette().background.r + theme.palette().background.g + theme.palette().background.b < 1.5
}

/// 根据主题与语义状态计算 blend color。
///
/// # 参数
///
/// 参数沿用调用点中的应用状态、工具输入或渲染上下文，不在这里扩大权限或补造状态。
///
/// # 返回值
///
/// 返回值保持当前模块的领域语义，供相邻视图或测试继续使用。
///
/// # 错误处理
///
/// 本函数不吞掉底层错误；没有显式错误通道时，会用空集合、`None` 或现有 UI 状态表达不可用结果。
pub(super) fn blend_color(a: Color, b: Color, t: f32) -> Color {
    let t = t.clamp(0.0, 1.0);
    Color::from_rgba(
        a.r * (1.0 - t) + b.r * t,
        a.g * (1.0 - t) + b.g * t,
        a.b * (1.0 - t) + b.b * t,
        a.a * (1.0 - t) + b.a * t,
    )
}

/// 处理 diff pane base background 对应的局部职责。
///
/// # 参数
///
/// 参数沿用调用点中的应用状态、工具输入或渲染上下文，不在这里扩大权限或补造状态。
///
/// # 返回值
///
/// 返回值保持当前模块的领域语义，供相邻视图或测试继续使用。
///
/// # 错误处理
///
/// 本函数不吞掉底层错误；没有显式错误通道时，会用空集合、`None` 或现有 UI 状态表达不可用结果。
pub(super) fn diff_pane_base_background(theme: &iced::Theme, tone: DiffSplitPaneTone) -> Color {
    let ext = theme.extended_palette();
    let is_dark = is_dark_theme(theme);

    match tone {
        // DiffSplitPaneTone 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        DiffSplitPaneTone::Neutral => {
            if is_dark {
                ext.background.base.color.scale_alpha(0.08)
            } else {
                ext.background.base.color.scale_alpha(0.96)
            }
        }
        DiffSplitPaneTone::Empty => {
            if is_dark {
                ext.background.weak.color.scale_alpha(0.08)
            } else {
                ext.background.weak.color.scale_alpha(0.36)
            }
        }
        DiffSplitPaneTone::Add => {
            if is_dark {
                ext.success.base.color.scale_alpha(0.34)
            } else {
                ext.success.base.color.scale_alpha(0.24)
            }
        }
        DiffSplitPaneTone::Delete => {
            if is_dark {
                ext.danger.base.color.scale_alpha(0.20)
            } else {
                ext.danger.base.color.scale_alpha(0.14)
            }
        }
    }
}

/// 处理 diff pane background 对应的局部职责。
///
/// # 参数
///
/// 参数沿用调用点中的应用状态、工具输入或渲染上下文，不在这里扩大权限或补造状态。
///
/// # 返回值
///
/// 返回值保持当前模块的领域语义，供相邻视图或测试继续使用。
///
/// # 错误处理
///
/// 本函数不吞掉底层错误；没有显式错误通道时，会用空集合、`None` 或现有 UI 状态表达不可用结果。
pub(super) fn diff_pane_background(
    theme: &iced::Theme,
    tone: DiffSplitPaneTone,
    emphasized: bool,
) -> Color {
    let ext = theme.extended_palette();
    let is_dark = is_dark_theme(theme);
    let base_bg = diff_pane_base_background(theme, tone);

    if emphasized {
        let hover_tint = ext.background.strong.color.scale_alpha(if is_dark { 0.12 } else { 0.04 });
        blend_color(base_bg, hover_tint, 0.22)
    } else {
        base_bg
    }
}

pub(super) fn merge_diff_row_style(
    theme: &iced::Theme,
    tone: DiffSplitPaneTone,
    emphasized: bool,
) -> iced::widget::container::Style {
    iced::widget::container::Style {
        background: Some(Background::Color(diff_pane_background(theme, tone, emphasized))),
        border: Border::default(),
        ..Default::default()
    }
}

pub(super) fn merge_diff_row_background_style(
    theme: &iced::Theme,
    background: Color,
    emphasized: bool,
) -> iced::widget::container::Style {
    let ext = theme.extended_palette();
    let is_dark = is_dark_theme(theme);
    let row_background = if emphasized {
        let hover_tint = ext.background.strong.color.scale_alpha(if is_dark { 0.12 } else { 0.04 });
        blend_color(background, hover_tint, 0.22)
    } else {
        background
    };

    iced::widget::container::Style {
        background: Some(Background::Color(row_background)),
        border: Border::default(),
        ..Default::default()
    }
}

pub(super) fn diff_split_pane_style(
    theme: &iced::Theme,
    tone: DiffSplitPaneTone,
    emphasized: bool,
) -> iced::widget::container::Style {
    let ext = theme.extended_palette();
    let background = diff_pane_background(theme, tone, emphasized);
    let accent = match tone {
        DiffSplitPaneTone::Add => Some(ext.success.base.color),
        DiffSplitPaneTone::Delete => Some(ext.danger.base.color),
        DiffSplitPaneTone::Neutral | DiffSplitPaneTone::Empty => None,
    };
    let border_color = accent.unwrap_or(Color::TRANSPARENT).scale_alpha(0.0);

    iced::widget::container::Style {
        background: Some(Background::Color(background)),
        border: Border { width: 0.0, color: border_color, radius: 0.0.into() },
        ..Default::default()
    }
}

pub(super) fn diff_split_pane_background_style(
    theme: &iced::Theme,
    background: Color,
    emphasized: bool,
) -> iced::widget::container::Style {
    let ext = theme.extended_palette();
    let is_dark = is_dark_theme(theme);
    let pane_background = if emphasized {
        let hover_tint = ext.background.strong.color.scale_alpha(if is_dark { 0.12 } else { 0.04 });
        blend_color(background, hover_tint, 0.22)
    } else {
        background
    };

    iced::widget::container::Style {
        background: Some(Background::Color(pane_background)),
        border: Border::default(),
        ..Default::default()
    }
}

pub(super) fn diff_split_divider_style(theme: &iced::Theme) -> iced::widget::container::Style {
    let ext = theme.extended_palette();
    let is_dark = is_dark_theme(theme);
    iced::widget::container::Style {
        background: Some(Background::Color(if is_dark {
            ext.background.strong.color.scale_alpha(0.20)
        } else {
            ext.background.strong.color.scale_alpha(0.12)
        })),
        ..Default::default()
    }
}

/// 处理 merge diff row 对应的局部职责。
///
/// # 参数
///
/// 参数沿用调用点中的应用状态、工具输入或渲染上下文，不在这里扩大权限或补造状态。
///
/// # 返回值
///
/// 无返回值时，函数通过发布消息或更新局部状态完成交互。
///
/// # 错误处理
///
/// 本函数不吞掉底层错误；没有显式错误通道时，会用空集合、`None` 或现有 UI 状态表达不可用结果。
pub(in crate::app::components::git_panel) fn merge_diff_row<'a>(
    content: Element<'a, Message>,
    // tone 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    tone: DiffSplitPaneTone,
    // emphasized 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    emphasized: bool,
) -> Element<'a, Message> {
    container(content)
        .width(Length::Fill)
        .style(move |theme: &iced::Theme| merge_diff_row_style(theme, tone, emphasized))
        .into()
}

/// 处理 merge diff row with background 对应的局部职责。
///
/// # 参数
///
/// 参数沿用调用点中的应用状态、工具输入或渲染上下文，不在这里扩大权限或补造状态。
///
/// # 返回值
///
/// 无返回值时，函数通过发布消息或更新局部状态完成交互。
///
/// # 错误处理
///
/// 本函数不吞掉底层错误；没有显式错误通道时，会用空集合、`None` 或现有 UI 状态表达不可用结果。
pub(in crate::app::components::git_panel) fn merge_diff_row_with_background<'a>(
    content: Element<'a, Message>,
    // background 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    background: Color,
    // emphasized 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    emphasized: bool,
) -> Element<'a, Message> {
    container(content)
        .width(Length::Fill)
        .style(move |theme: &iced::Theme| {
            merge_diff_row_background_style(theme, background, emphasized)
        })
        .into()
}

/// 处理 diff highlight enabled 对应的局部职责。
///
/// # 参数
///
/// 参数沿用调用点中的应用状态、工具输入或渲染上下文，不在这里扩大权限或补造状态。
///
/// # 返回值
///
/// `true` 表示当前输入满足该辅助函数描述的条件。
///
/// # 错误处理
///
/// 本函数不吞掉底层错误；没有显式错误通道时，会用空集合、`None` 或现有 UI 状态表达不可用结果。
pub(in crate::app::components::git_panel) fn diff_highlight_enabled(_app: &App) -> bool {
    true
}

/// 处理 diff split pane 对应的局部职责。
///
/// # 参数
///
/// 参数沿用调用点中的应用状态、工具输入或渲染上下文，不在这里扩大权限或补造状态。
///
/// # 返回值
///
/// 无返回值时，函数通过发布消息或更新局部状态完成交互。
///
/// # 错误处理
///
/// 本函数不吞掉底层错误；没有显式错误通道时，会用空集合、`None` 或现有 UI 状态表达不可用结果。
pub(in crate::app::components::git_panel) fn diff_split_pane<'a>(
    content: Element<'a, Message>,
    // tone 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    tone: DiffSplitPaneTone,
    // emphasized 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    emphasized: bool,
) -> Element<'a, Message> {
    container(content)
        .width(Length::FillPortion(1))
        .padding([0, 3])
        .style(move |theme: &iced::Theme| diff_split_pane_style(theme, tone, emphasized))
        .into()
}

/// 处理 diff split pane with background 对应的局部职责。
///
/// # 参数
///
/// 参数沿用调用点中的应用状态、工具输入或渲染上下文，不在这里扩大权限或补造状态。
///
/// # 返回值
///
/// 无返回值时，函数通过发布消息或更新局部状态完成交互。
///
/// # 错误处理
///
/// 本函数不吞掉底层错误；没有显式错误通道时，会用空集合、`None` 或现有 UI 状态表达不可用结果。
pub(in crate::app::components::git_panel) fn diff_split_pane_with_background<'a>(
    content: Element<'a, Message>,
    // background 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    background: Color,
    // emphasized 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    emphasized: bool,
) -> Element<'a, Message> {
    container(content)
        .width(Length::FillPortion(1))
        .style(move |theme: &iced::Theme| {
            diff_split_pane_background_style(theme, background, emphasized)
        })
        .into()
}

/// 处理 diff line number with background 对应的局部职责。
///
/// # 参数
///
/// 参数沿用调用点中的应用状态、工具输入或渲染上下文，不在这里扩大权限或补造状态。
///
/// # 返回值
///
/// 无返回值时，函数通过发布消息或更新局部状态完成交互。
///
/// # 错误处理
///
/// 本函数不吞掉底层错误；没有显式错误通道时，会用空集合、`None` 或现有 UI 状态表达不可用结果。
pub(in crate::app::components::git_panel) fn diff_line_number_with_background<'a>(
    content: Element<'a, Message>,
    // _background 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    _background: Color,
) -> Element<'a, Message> {
    content
}

/// 处理 diff split divider 对应的局部职责。
///
/// # 参数
///
/// 参数沿用调用点中的应用状态、工具输入或渲染上下文，不在这里扩大权限或补造状态。
///
/// # 返回值
///
/// 返回值是 Iced `Element`，调用方继续组合到当前界面树中。
///
/// # 错误处理
///
/// 本函数不吞掉底层错误；没有显式错误通道时，会用空集合、`None` 或现有 UI 状态表达不可用结果。
pub(in crate::app::components::git_panel) fn diff_split_divider<'a>() -> Element<'a, Message> {
    container(Space::new().width(Length::Fill).height(Length::Fill))
        .width(Length::Fixed(DIFF_SPLIT_DIVIDER_WIDTH))
        .height(Length::Fill)
        .style(diff_split_divider_style)
        .into()
}

/// 处理 split line number area 对应的局部职责。
///
/// # 参数
///
/// 参数沿用调用点中的应用状态、工具输入或渲染上下文，不在这里扩大权限或补造状态。
///
/// # 返回值
///
/// 无返回值时，函数通过发布消息或更新局部状态完成交互。
///
/// # 错误处理
///
/// 本函数不吞掉底层错误；没有显式错误通道时，会用空集合、`None` 或现有 UI 状态表达不可用结果。
pub(in crate::app::components::git_panel) fn split_line_number_area(
    file: &str,
    // line_info 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    line_info: Option<(usize, bool)>,
    // content 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    content: &str,
    // tone 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    tone: markers::LineNumberTone,
) -> Element<'static, Message> {
    match line_info {
        Some((line, is_old)) => {
            // MouseArea 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
            MouseArea::new(markers::line_number_cell_with_tone((line + 1).to_string(), tone))
                .on_press(Message::Git(message::GitMessage::DiffDragSelectStart(
                    file.to_string(),
                    line,
                    is_old,
                    content.to_string(),
                )))
                .on_enter(Message::Git(message::GitMessage::DiffDragSelectHover(
                    file.to_string(),
                    line,
                    is_old,
                )))
                .into()
        }
        None => markers::empty_line_number_cell(),
    }
}
