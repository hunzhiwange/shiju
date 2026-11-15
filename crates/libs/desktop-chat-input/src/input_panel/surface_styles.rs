//! 输入面板样式定义模块
//!
//! 本模块提供输入面板组件的各种视觉样式函数，用于定义按钮、容器、编辑器等 UI 元素的外观。
//! 所有样式函数均基于 iced 框架的主题系统，支持响应式主题切换和状态变化。

use iced::widget::{button, container, text_editor};
use iced::{Background, Border, Color, Theme};

use desktop_theme::theme_catalog::ChromeStyleOverrides;

pub const BOTTOM_BAR_ICON_BUTTON_SIZE: f32 = 24.0;
pub const BOTTOM_BAR_ICON_SIZE: f32 = 12.0;
pub const BOTTOM_BAR_LARGE_ICON_SIZE: f32 = 14.0;
pub const BOTTOM_BAR_CHEVRON_ICON_SIZE: f32 = 11.0;
pub const BOTTOM_BAR_LABEL_SIZE: f32 = 12.0;

fn is_dark_theme(theme: &Theme) -> bool {
    theme.palette().background.r + theme.palette().background.g + theme.palette().background.b < 1.5
}

pub use super::quick_styles::*;

pub use desktop_ui_controls::tooltip::tooltip_dark_style;

/// 生成方形图标按钮样式
///
/// 根据按钮状态和启用状态返回相应的样式配置。
/// 悬停时显示弱背景色，按下时显示强背景色，支持禁用状态视觉反馈。
///
/// # 参数
///
/// * `theme` - iced 主题引用，用于获取调色板颜色
/// * `status` - 按钮当前状态（Active、Hovered、Pressed、Disabled）
/// * `enabled` - 按钮是否启用，影响文字颜色
///
/// # 返回值
///
/// 返回配置好的 `button::Style` 实例：
/// - 悬停状态：弱背景色（35% 透明度）
/// - 按下状态：强背景色（45% 透明度）
/// - 其他状态：透明背景
/// - 6px 圆角，无边框
/// - 启用时使用主题文字颜色，禁用时使用弱背景文字颜色
pub use super::styles::square_icon_button_style;

/// 生成无边框圆形图标按钮样式
///
/// 提供与发送按钮一致的圆形轮廓，但保持中性背景，不使用描边。
pub use desktop_chat_view::header_style::round_icon_button_style;

pub use super::styles::selector_label_font;

pub use super::styles::selector_text_color;

pub use super::styles::selector_chevron_color;

pub use super::styles::selector_pill_button_style;

pub use super::styles::selectable_list_button_style;

/// 生成弹出框容器样式
///
/// 返回适用于弹出菜单、下拉面板等浮动组件的容器样式。
/// 样式包含高不透明度背景和细边框，与主题颜色协调。
///
/// # 参数
///
/// * `theme` - iced 主题引用，用于获取调色板颜色
///
/// # 返回值
///
/// 返回配置好的 `container::Style` 实例：
/// - 背景：主题背景色（98% 不透明度）
/// - 1px 强背景色边框
/// - 8px 圆角
pub use super::styles::popover_style;

/// 生成输入卡片容器样式
///
/// 返回输入面板卡片的样式，支持文件拖拽悬停状态的视觉反馈。
/// 拖拽悬停时显示主题色边框和淡色背景。
///
/// # 参数
///
/// * `theme` - iced 主题引用，用于获取调色板颜色
/// * `drop_hovered` - 是否处于文件拖拽悬停状态
/// * `chrome` - 可选的活动主题控件皮肤覆盖
///
/// # 返回值
///
/// 返回配置好的 `container::Style` 实例：
/// - 普通状态：无背景，强背景色边框
/// - 拖拽悬停状态：主色边框，主色淡背景（8% 透明度）
/// - 14px 圆角，无阴影
pub fn input_card_style(
    theme: &Theme,
    drop_hovered: bool,
    chrome: Option<&ChromeStyleOverrides>,
) -> container::Style {
    let is_dark = is_dark_theme(theme);
    let default_bg =
        desktop_theme::theme_catalog::input_surface_color(theme, if is_dark { 0.96 } else { 1.0 });
    let chrome_bg = chrome.and_then(|c| c.input_bg.or(c.panel_bg));
    let base_bg = chrome_bg.unwrap_or(default_bg);
    let palette = theme.extended_palette();
    let default_border = if is_dark {
        palette.background.strong.color.scale_alpha(0.72)
    } else {
        palette.background.strong.color.scale_alpha(0.48)
    };
    let chrome_border = chrome.and_then(|c| c.input_border.or(c.panel_border));
    let border_color = if drop_hovered {
        theme.palette().primary.scale_alpha(0.82)
    } else {
        chrome_border.unwrap_or(default_border)
    };
    let background = if drop_hovered {
        Color::from_rgba(
            base_bg.r * 0.94 + theme.palette().primary.r * 0.06,
            base_bg.g * 0.94 + theme.palette().primary.g * 0.06,
            base_bg.b * 0.94 + theme.palette().primary.b * 0.06,
            base_bg.a,
        )
    } else {
        base_bg
    };
    let radius = chrome.and_then(|c| c.radius_px).unwrap_or(16.0);
    container::Style {
        background: Some(Background::Color(background)),
        border: Border { width: 1.0, color: border_color, radius: radius.into() },
        shadow: iced::Shadow {
            color: Color::BLACK.scale_alpha(if is_dark { 0.10 } else { 0.035 }),
            offset: iced::Vector::new(0.0, 4.0),
            blur_radius: 14.0,
        },
        ..Default::default()
    }
}

pub fn manual_context_card_style(theme: &Theme) -> container::Style {
    let is_dark = is_dark_theme(theme);
    let primary = theme.palette().primary;
    let background = if is_dark {
        Color::from_rgba(
            primary.r * 0.20 + 18.0 / 255.0 * 0.80,
            primary.g * 0.20 + 19.0 / 255.0 * 0.80,
            primary.b * 0.20 + 22.0 / 255.0 * 0.80,
            0.96,
        )
    } else {
        Color::from_rgba(
            primary.r * 0.08 + 250.0 / 255.0 * 0.92,
            primary.g * 0.08 + 251.0 / 255.0 * 0.92,
            primary.b * 0.08 + 253.0 / 255.0 * 0.92,
            1.0,
        )
    };
    let border = if is_dark { primary.scale_alpha(0.34) } else { primary.scale_alpha(0.24) };
    container::Style {
        background: Some(Background::Color(background)),
        border: Border { width: 1.0, color: border, radius: 12.0.into() },
        ..Default::default()
    }
}

pub fn manual_context_card_button_style(theme: &Theme, status: button::Status) -> button::Style {
    let mut style = button::Style {
        background: Some(Background::Color(Color::TRANSPARENT)),
        border: Border { width: 0.0, color: Color::TRANSPARENT, radius: 12.0.into() },
        text_color: selector_text_color(theme, true),
        ..Default::default()
    };

    if matches!(status, button::Status::Hovered | button::Status::Pressed) {
        style.background =
            Some(Background::Color(theme.palette().primary.scale_alpha(if is_dark_theme(theme) {
                0.10
            } else {
                0.06
            })));
    }

    style
}

/// 生成文本编辑器样式
///
/// 返回主输入编辑器的样式配置，支持请求中状态的视觉反馈。
/// 根据主题亮度自动调整占位符文字颜色。
///
/// # 参数
///
/// * `theme` - iced 主题引用，用于获取调色板颜色
/// * `_status` - 编辑器状态（当前未使用）
/// * `requesting` - 是否正在发送请求，影响文字颜色
/// * `chrome` - 可选的活动主题控件皮肤覆盖
///
/// # 返回值
///
/// 返回配置好的 `text_editor::Style` 实例：
/// - 透明背景，无边框
/// - 普通状态：主题文字颜色
/// - 请求中状态：次要色文字颜色
/// - 选中文本：主色背景（30% 透明度）
/// - 占位符：根据主题亮度自动调整（深色主题 60% 透明度，浅色主题 80% 透明度）
pub fn editor_style(
    theme: &Theme,
    _status: text_editor::Status,
    requesting: bool,
    chrome: Option<&ChromeStyleOverrides>,
) -> text_editor::Style {
    let p = theme.extended_palette();
    let palette = theme.palette();
    let is_dark = is_dark_theme(theme);
    let chrome_text = chrome.and_then(|c| c.input_text);
    let value = if let Some(text) = chrome_text {
        if requesting { text.scale_alpha(0.92) } else { text }
    } else if requesting {
        palette.text.scale_alpha(0.92)
    } else {
        palette.text.scale_alpha(if is_dark { 0.96 } else { 0.94 })
    };
    let placeholder = if is_dark {
        chrome_text.unwrap_or(palette.text).scale_alpha(0.48)
    } else {
        p.secondary.base.text.scale_alpha(0.56)
    };
    text_editor::Style {
        background: Background::Color(Color::TRANSPARENT),
        border: Border { width: 0.0, color: Color::TRANSPARENT, radius: 0.0.into() },
        value,
        selection: theme.palette().primary.scale_alpha(if is_dark { 0.26 } else { 0.18 }),
        placeholder,
    }
}
