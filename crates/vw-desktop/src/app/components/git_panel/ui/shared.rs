//! Git 面板通用按钮组件。
//!
//! 本模块封装 Git 面板中复用的图标、字形按钮、禁用态和提示样式。

use iced::Element;
/// 重新导出 use iced::widget::svg::{self, Svg}，让上层模块通过稳定路径访问。
use iced::widget::svg::{self, Svg};
/// 重新导出 use iced::widget::tooltip::{Position as TooltipPosition, Tooltip}，让上层模块通过稳定路径访问。
/// 重新导出 use iced::widget::{button, text}，让上层模块通过稳定路径访问。
use iced::widget::{button, text};
/// 重新导出 use iced::{Background, Border, Color, Length}，让上层模块通过稳定路径访问。
use iced::{Background, Border, Color, Length};

/// 重新导出 use crate::app::Message，让上层模块通过稳定路径访问。
use crate::app::Message;
/// 重新导出 use crate::app::assets::{Icon, get_icon}，让上层模块通过稳定路径访问。
use crate::app::assets::{Icon, get_icon};

/// 处理 icon svg 对应的局部职责。
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
pub(super) fn icon_svg(icon: Icon) -> Svg<'static> {
    icon_svg_sized(icon, 18.0)
}

/// 处理 icon svg sized 对应的局部职责。
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
pub(super) fn icon_svg_sized(icon: Icon, size: f32) -> Svg<'static> { desktop_git_ui::git_panel::ui::icon_svg_sized(icon, size) }

/// 处理 disabled icon svg 对应的局部职责。
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
pub(super) fn disabled_icon_svg(icon: Icon, size: f32) -> Svg<'static> {
    // Svg 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    Svg::new(get_icon(icon)).width(Length::Fixed(size)).height(Length::Fixed(size)).style(
        |theme: &iced::Theme, _status| svg::Style {
            // color 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
            color: Some(theme.extended_palette().background.weak.text),
        },
    )
}

/// 处理 themed icon svg 对应的局部职责。
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
pub(super) fn themed_icon_svg(icon: Icon, size: f32) -> Svg<'static> {
    icon_svg_sized(icon, size).style(|theme: &iced::Theme, _status| svg::Style {
        // color 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        color: Some(theme.extended_palette().background.strong.text),
    })
}

/// 处理 with tooltip 对应的局部职责。
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
pub(super) fn with_tooltip<'a>(
    content: impl Into<Element<'a, Message>>,
    // tip 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    tip: String,
    // compact 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    compact: bool,
    // gap 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    gap: f32,
) -> Element<'a, Message> { desktop_git_ui::git_panel::ui::with_tooltip(content, tip, compact, gap) }

/// 构建 outlined button style 控件，并绑定既有消息或样式。
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
pub(super) fn outlined_button_style(
    theme: &iced::Theme,
    // status 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    status: button::Status,
    // radius 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    radius: f32,
) -> button::Style {
    let palette = theme.extended_palette();
    let background = match status {
        // button 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        button::Status::Hovered => Some(palette.background.weak.color.into()),
        // button 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        button::Status::Pressed => Some(palette.background.strong.color.into()),
        _ => None,
    };
    button::Style {
        background,
        // border 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        border: Border {
            // width 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
            width: 1.0,
            // color 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
            color: palette.background.strong.color,
            // radius 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
            radius: radius.into(),
        },
        ..Default::default()
    }
}

/// 构建 compact outlined button style 控件，并绑定既有消息或样式。
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
pub(super) fn compact_outlined_button_style(
    theme: &iced::Theme,
    // status 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    status: button::Status,
    // radius 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    radius: f32,
) -> button::Style {
    let palette = theme.extended_palette();
    let is_dark = is_dark_theme(theme);
    let (background, text_color) = match status {
        // button 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        button::Status::Hovered => {
            if is_dark {
                (
                    Some(Background::Color(Color::from_rgba8(0xFF, 0xFF, 0xFF, 0.12))),
                    palette.background.strong.text,
                )
            } else {
                (
                    Some(palette.background.weak.color.scale_alpha(0.5).into()),
                    palette.background.strong.text,
                )
            }
        }
        button::Status::Pressed => {
            if is_dark {
                (
                    Some(Background::Color(Color::from_rgba8(0xFF, 0xFF, 0xFF, 0.20))),
                    palette.background.strong.text,
                )
            } else {
                (
                    Some(palette.background.strong.color.scale_alpha(0.72).into()),
                    palette.background.strong.text,
                )
            }
        }
        _ => {
            if is_dark {
                (
                    Some(Background::Color(Color::from_rgba8(0xFF, 0xFF, 0xFF, 0.06))),
                    palette.background.strong.text,
                )
            } else {
                (None, palette.background.strong.text)
            }
        }
    };
    let border_color = if is_dark {
        // Color 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        Color::from_rgba8(0xA8, 0xB0, 0xBA, 0.28)
    } else {
        palette.background.strong.color
    };
    button::Style {
        background,
        // border 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        border: Border { width: 1.0, color: border_color, radius: radius.into() },
        text_color,
        ..Default::default()
    }
}

/// 构建 disabled button style 控件，并绑定既有消息或样式。
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
pub(super) fn disabled_button_style(theme: &iced::Theme, radius: f32) -> button::Style {
    let palette = theme.extended_palette();
    button::Style {
        // background 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        background: Some(palette.background.weak.color.into()),
        // border 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        border: Border {
            // width 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
            width: 1.0,
            // color 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
            color: palette.background.strong.color,
            // radius 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
            radius: radius.into(),
        },
        // text_color 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        text_color: palette.background.weak.text,
        ..Default::default()
    }
}

/// 构建 subtle button style 控件，并绑定既有消息或样式。
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
pub(super) fn subtle_button_style(status: button::Status, radius: f32) -> button::Style {
    let background = match status {
        // button 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        button::Status::Hovered => Color::from_rgba8(235, 235, 235, 1.0),
        // button 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        button::Status::Pressed => Color::from_rgba8(225, 225, 225, 1.0),
        _ => Color::TRANSPARENT,
    };
    button::Style {
        // background 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        background: Some(Background::Color(background)),
        // border 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        border: Border { radius: radius.into(), ..Default::default() },
        ..Default::default()
    }
}

/// 构建 small plain icon button style 控件，并绑定既有消息或样式。
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
pub(super) fn small_plain_icon_button_style(
    theme: &iced::Theme,
    // status 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    status: button::Status,
) -> button::Style { desktop_git_ui::git_panel::ui::small_plain_icon_button_style(theme, status, button) }

/// 构建 header plain glyph button style 控件，并绑定既有消息或样式。
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
pub(super) fn header_plain_glyph_button_style(
    theme: &iced::Theme,
    // status 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    status: button::Status,
) -> button::Style {
    let palette = theme.extended_palette();
    let is_dark = is_dark_theme(theme);
    let (background, text_color) = match status {
        // button 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        button::Status::Hovered => {
            if is_dark {
                (
                    Some(Background::Color(Color::from_rgba8(0xFF, 0xFF, 0xFF, 0.05))),
                    palette.background.strong.text,
                )
            } else {
                (
                    Some(Background::Color(palette.background.weak.color)),
                    palette.secondary.base.text,
                )
            }
        }
        button::Status::Pressed => {
            if is_dark {
                (
                    Some(Background::Color(Color::from_rgba8(0xFF, 0xFF, 0xFF, 0.09))),
                    palette.background.strong.text,
                )
            } else {
                (
                    Some(Background::Color(palette.background.strong.color)),
                    palette.secondary.base.text,
                )
            }
        }
        _ => (None, palette.background.strong.text),
    };
    button::Style {
        background,
        // border 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        border: Border { radius: 8.0.into(), ..Default::default() },
        text_color,
        ..Default::default()
    }
}

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
fn is_dark_theme(theme: &iced::Theme) -> bool {
    let palette = theme.palette();
    palette.background.r + palette.background.g + palette.background.b < 1.5
}
