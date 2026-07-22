//! 项目工作区样式模块，负责根据主题生成面板、按钮和提示气泡样式。
//!
//! 本模块只负责视图组合与样式适配，不持有业务状态，也不扩大外部能力边界。

use iced::border::{Border, Radius};
use iced::widget::container::Style as ContainerStyle;
use iced::widget::{container, text};
use iced::{Background, Color, Theme};

use crate::app::Message;

fn blend_color(from: Color, to: Color, amount: f32) -> Color {
    let amount = amount.clamp(0.0, 1.0);
    Color {
        r: from.r + (to.r - from.r) * amount,
        g: from.g + (to.g - from.g) * amount,
        b: from.b + (to.b - from.b) * amount,
        a: from.a + (to.a - from.a) * amount,
    }
}

fn elevated_shadow(theme: &Theme, dark_alpha: f32, light_alpha: f32) -> iced::Shadow {
    iced::Shadow {
        color: Color::BLACK.scale_alpha(if is_dark_theme(theme) {
            dark_alpha
        } else {
            light_alpha
        }),
        offset: iced::Vector::new(0.0, 12.0),
        blur_radius: 28.0,
    }
}

/// 执行本模块的界面辅助逻辑。
///
/// # 参数
/// - `theme`: 当前视图构建所需的状态、配置或消息。
///
/// # 返回
/// 返回判断结果，供调用方选择分支或样式。
///
/// # 错误
/// 此函数不返回 `Result`；不可用状态会通过空视图、禁用控件或回退文案表达。
pub fn is_dark_theme(theme: &Theme) -> bool {
    let palette = theme.palette();
    palette.background.r + palette.background.g + palette.background.b < 1.5
}

/// 生成主题相关样式。
///
/// # 参数
/// - `theme`: 当前视图构建所需的状态、配置或消息。
///
/// # 返回
/// 返回值遵循函数签名约定，调用方据此继续组装界面或更新状态。
///
/// # 错误
/// 此函数不返回 `Result`；不可用状态会通过空视图、禁用控件或回退文案表达。
pub fn workspace_background_style(theme: &Theme) -> ContainerStyle {
    let palette = theme.extended_palette();
    let background = if is_dark_theme(theme) {
        blend_color(palette.background.base.color, Color::BLACK, 0.24)
    } else {
        blend_color(palette.background.base.color, Color::from_rgb8(244, 246, 250), 0.72)
    };

    ContainerStyle { background: Some(Background::Color(background)), ..Default::default() }
}

/// 生成主题相关样式。
///
/// # 参数
/// - `theme`: 当前视图构建所需的状态、配置或消息。
///
/// # 返回
/// 返回值遵循函数签名约定，调用方据此继续组装界面或更新状态。
///
/// # 错误
/// 此函数不返回 `Result`；不可用状态会通过空视图、禁用控件或回退文案表达。
pub fn content_panel_style(theme: &Theme) -> ContainerStyle {
    let palette = theme.extended_palette();
    let is_dark = is_dark_theme(theme);
    let background = if is_dark {
        Color::from_rgba8(17, 18, 22, 0.98)
    } else {
        Color::from_rgba8(255, 255, 255, 0.985)
    };
    let border_color = if is_dark {
        palette.background.strong.color.scale_alpha(0.70)
    } else {
        Color::from_rgba8(225, 229, 236, 0.98)
    };

    ContainerStyle {
        background: Some(Background::Color(background)),
        border: Border { color: border_color, width: 1.0, radius: 18.0.into() },
        shadow: elevated_shadow(theme, 0.14, 0.04),
        ..Default::default()
    }
}

/// 生成主题相关样式。
///
/// # 参数
/// - `_theme`: 当前视图构建所需的状态、配置或消息。
///
/// # 返回
/// 返回值遵循函数签名约定，调用方据此继续组装界面或更新状态。
///
/// # 错误
/// 此函数不返回 `Result`；不可用状态会通过空视图、禁用控件或回退文案表达。
pub fn left_rail_style(_theme: &Theme) -> ContainerStyle {
    ContainerStyle {
        background: None,
        border: Border { color: Color::TRANSPARENT, width: 0.0, radius: Radius::from(0.0) },
        shadow: iced::Shadow::default(),
        ..Default::default()
    }
}

/// 计算颜色表现。
///
/// # 参数
/// - `theme`: 当前视图构建所需的状态、配置或消息。
///
/// # 返回
/// 返回根据输入和主题计算出的颜色。
///
/// # 错误
/// 此函数不返回 `Result`；不可用状态会通过空视图、禁用控件或回退文案表达。
pub fn divider_line_color(theme: &Theme) -> Color {
    let is_dark = is_dark_theme(theme);
    if is_dark {
        Color::from_rgba8(50, 54, 61, 0.96)
    } else {
        Color::from_rgba8(221, 225, 232, 1.0)
    }
}

/// 计算颜色表现。
///
/// # 参数
/// - `theme`: 当前视图构建所需的状态、配置或消息。
///
/// # 返回
/// 返回根据输入和主题计算出的颜色。
///
/// # 错误
/// 此函数不返回 `Result`；不可用状态会通过空视图、禁用控件或回退文案表达。
pub fn session_border_color(theme: &Theme) -> Color {
    let base = divider_line_color(theme);
    if is_dark_theme(theme) { base.scale_alpha(0.96) } else { base }
}

/// 生成主题相关样式。
///
/// # 参数
/// - `theme`: 当前视图构建所需的状态、配置或消息。
/// - `corner_radius`: 当前视图构建所需的状态、配置或消息。
///
/// # 返回
/// 返回值遵循函数签名约定，调用方据此继续组装界面或更新状态。
///
/// # 错误
/// 此函数不返回 `Result`；不可用状态会通过空视图、禁用控件或回退文案表达。
pub fn session_panel_style(theme: &Theme, corner_radius: f32) -> ContainerStyle {
    let palette = theme.extended_palette();
    let is_dark = is_dark_theme(theme);
    let background = if is_dark {
        blend_color(palette.background.base.color, Color::from_rgb8(13, 15, 19), 0.42)
    } else {
        Color::from_rgba8(252, 253, 255, 0.99)
    };
    let border_color = if is_dark {
        palette.background.strong.color.scale_alpha(0.78)
    } else {
        Color::from_rgba8(222, 226, 233, 0.98)
    };
    let radius = corner_radius + 4.0;

    ContainerStyle {
        background: Some(Background::Color(background)),
        border: Border { color: border_color, width: 1.0, radius: radius.into() },
        shadow: elevated_shadow(theme, 0.18, 0.05),
        ..Default::default()
    }
}

/// 生成主题相关样式。
///
/// # 参数
/// - `theme`: 当前视图构建所需的状态、配置或消息。
/// - `corner_radius`: 当前视图构建所需的状态、配置或消息。
///
/// # 返回
/// 返回值遵循函数签名约定，调用方据此继续组装界面或更新状态。
///
/// # 错误
/// 此函数不返回 `Result`；不可用状态会通过空视图、禁用控件或回退文案表达。
pub fn right_column_inner_style(theme: &Theme, corner_radius: f32) -> ContainerStyle {
    let palette = theme.extended_palette();
    let background = if is_dark_theme(theme) {
        blend_color(palette.background.base.color, Color::from_rgb8(12, 13, 17), 0.34)
    } else {
        Color::from_rgba8(254, 254, 255, 0.99)
    };

    ContainerStyle {
        background: Some(Background::Color(background)),
        border: Border {
            color: Color::TRANSPARENT,
            width: 0.0,
            radius: (corner_radius + 4.0).into(),
        },
        ..Default::default()
    }
}

/// 生成主题相关样式。
///
/// # 参数
/// - `theme`: 当前视图构建所需的状态、配置或消息。
/// - `corner_radius`: 当前视图构建所需的状态、配置或消息。
///
/// # 返回
/// 返回值遵循函数签名约定，调用方据此继续组装界面或更新状态。
///
/// # 错误
/// 此函数不返回 `Result`；不可用状态会通过空视图、禁用控件或回退文案表达。
pub fn right_column_outer_style(theme: &Theme, corner_radius: f32) -> ContainerStyle {
    ContainerStyle {
        border: Border {
            color: session_border_color(theme),
            width: 1.0,
            radius: (corner_radius + 4.0).into(),
        },
        shadow: iced::Shadow::default(),
        ..Default::default()
    }
}

/// 生成主题相关样式。
///
/// # 参数
/// - `_theme`: 当前视图构建所需的状态、配置或消息。
///
/// # 返回
/// 返回值遵循函数签名约定，调用方据此继续组装界面或更新状态。
///
/// # 错误
/// 此函数不返回 `Result`；不可用状态会通过空视图、禁用控件或回退文案表达。
pub fn panel_style_no_border(_theme: &Theme) -> ContainerStyle {
    ContainerStyle {
        border: Border { color: Color::TRANSPARENT, width: 0.0, radius: Radius::from(0.0) },
        ..Default::default()
    }
}

/// 计算颜色表现。
///
/// # 参数
/// - `theme`: 当前视图构建所需的状态、配置或消息。
///
/// # 返回
/// 返回根据输入和主题计算出的颜色。
///
/// # 错误
/// 此函数不返回 `Result`；不可用状态会通过空视图、禁用控件或回退文案表达。
pub fn session_row_highlight_color(theme: &Theme) -> Color {
    let is_dark = is_dark_theme(theme);
    if is_dark {
        Color::from_rgba8(37, 41, 48, 0.96)
    } else {
        Color::from_rgba8(238, 241, 246, 1.0)
    }
}

fn tooltip_bubble_style(_theme: &Theme) -> ContainerStyle {
    iced::widget::container::Style {
        text_color: Some(Color::WHITE),
        background: Some(Background::Color(Color::from_rgb8(28, 28, 30))),
        border: Border { width: 0.0, color: Color::TRANSPARENT, radius: 12.0.into() },
        shadow: iced::Shadow {
            color: Color::BLACK.scale_alpha(0.32),
            offset: iced::Vector::new(0.0, 4.0),
            blur_radius: 16.0,
        },
        snap: false,
    }
}

/// 执行本模块的界面辅助逻辑。
///
/// # 参数
/// - `tip`: 当前视图构建所需的状态、配置或消息。
///
/// # 返回
/// 返回可直接嵌入父级视图的 Iced 控件或样式闭包。
///
/// # 错误
/// 此函数不返回 `Result`；不可用状态会通过空视图、禁用控件或回退文案表达。
pub fn tooltip_bubble<'a>(tip: String) -> iced::widget::Container<'a, Message> {
    container(text(tip).size(12)).padding([6, 10]).style(tooltip_bubble_style)
}

/// 生成主题相关样式。
///
/// # 参数
/// - `theme`: 当前视图构建所需的状态、配置或消息。
/// - `selected`: 当前视图构建所需的状态、配置或消息。
/// - `accent`: 当前视图构建所需的状态、配置或消息。
/// - `status`: 当前视图构建所需的状态、配置或消息。
///
/// # 返回
/// 返回值遵循函数签名约定，调用方据此继续组装界面或更新状态。
///
/// # 错误
/// 此函数不返回 `Result`；不可用状态会通过空视图、禁用控件或回退文案表达。
pub fn project_item_button_style(
    theme: &Theme,
    selected: bool,
    accent: Color,
    status: iced::widget::button::Status,
) -> iced::widget::button::Style {
    let palette = theme.extended_palette();
    let is_dark = is_dark_theme(theme);
    let is_hovered = matches!(status, iced::widget::button::Status::Hovered);
    let is_pressed = matches!(status, iced::widget::button::Status::Pressed);
    let is_active = selected || is_hovered || is_pressed;

    let background = if is_pressed {
        blend_color(palette.background.base.color, accent, if is_dark { 0.26 } else { 0.18 })
    } else if is_hovered {
        blend_color(palette.background.base.color, accent, if is_dark { 0.16 } else { 0.11 })
    } else if selected {
        blend_color(palette.background.base.color, accent, if is_dark { 0.11 } else { 0.07 })
    } else {
        Color::TRANSPARENT
    };
    let border_color = if is_pressed {
        blend_color(accent, palette.background.base.color, 0.18)
    } else if is_hovered {
        blend_color(accent, palette.background.base.color, 0.28)
    } else if selected {
        blend_color(accent, palette.background.base.color, 0.36)
    } else {
        Color::TRANSPARENT
    };

    iced::widget::button::Style {
        background: Some(Background::Color(background)),
        text_color: theme.palette().text,
        border: iced::Border {
            radius: 18.0.into(),
            width: if is_active { 1.0 } else { 0.0 },
            color: border_color,
        },
        shadow: if is_active {
            iced::Shadow {
                color: accent.scale_alpha(if is_pressed {
                    if is_dark { 0.18 } else { 0.10 }
                } else if is_hovered {
                    if is_dark { 0.14 } else { 0.07 }
                } else if is_dark {
                    0.10
                } else {
                    0.05
                }),
                offset: iced::Vector::new(0.0, if is_pressed { 4.0 } else { 8.0 }),
                blur_radius: if is_pressed { 10.0 } else { 18.0 },
            }
        } else {
            iced::Shadow::default()
        },
        ..Default::default()
    }
}

#[cfg(test)]
#[path = "styles_tests.rs"]
mod styles_tests;
