//! 项目工作区样式模块，负责根据主题生成面板、按钮和提示气泡样式。
//!
//! 本模块只负责视图组合与样式适配，不持有业务状态，也不扩大外部能力边界。

use iced::border::{Border, Radius};
use iced::widget::container::Style as ContainerStyle;
use iced::{Background, Color, Theme};


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
pub use desktop_workspace::views::project::badge_styles::is_dark_theme;

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

/// 工作模式侧栏与主区域共用的主题底色。
pub(super) fn work_surface_style(theme: &Theme) -> ContainerStyle {
    let palette = theme.extended_palette();
    let background = if palette.is_dark {
        palette.background.weak.color
    } else {
        crate::app::theme_catalog::panel_surface_color(theme, 1.0)
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
pub use desktop_workspace::views::project::badge_styles::content_panel_style;

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
        blend_color(palette.background.base.color, palette.background.strong.color, 0.28)
            .scale_alpha(0.99)
    } else {
        crate::app::theme_catalog::panel_surface_color(theme, 0.99)
    };
    let border_color = if is_dark {
        palette.background.strong.color.scale_alpha(0.78)
    } else {
        palette.background.strong.color.scale_alpha(0.55)
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
        blend_color(palette.background.base.color, palette.background.strong.color, 0.22)
            .scale_alpha(0.99)
    } else {
        crate::app::theme_catalog::panel_surface_color(theme, 0.99)
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
/// - `_theme`: 当前视图构建所需的状态、配置或消息。
///
/// # 返回
/// 返回值遵循函数签名约定，调用方据此继续组装界面或更新状态。
///
/// # 错误
/// 此函数不返回 `Result`；不可用状态会通过空视图、禁用控件或回退文案表达。
pub use desktop_workspace::views::project::badge_styles::panel_style_no_border;

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
pub use desktop_workspace::views::project::row_styles::{session_row_highlight_color, tooltip_bubble};

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
pub use desktop_workspace::views::project::badge_styles::project_item_button_style;

#[cfg(test)]
#[path = "styles_tests.rs"]
mod styles_tests;
