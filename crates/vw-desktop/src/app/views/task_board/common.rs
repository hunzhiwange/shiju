//! 任务看板通用组件和样式
//!
//! 本模块提供任务看板视图中使用的通用工具函数、样式定义和常量。
//! 主要包括：
//! - UI 组件样式函数（按钮样式）
//! - 图标和资源加载工具
//! - 布局相关常量
//!
//! # 用途
//!
//! 这些通用组件被任务看板的各个子模块共享使用，确保整个任务看板界面
//! 的视觉一致性和代码复用。

use iced::{Background, Border, Color, Theme};

use crate::app::assets::{self, Icon};
use crate::app::components::system_settings_common::{
    danger_action_btn_style, primary_action_btn_style, rounded_action_btn_style,
};

/// 子任务徽章的默认尺寸（以像素为单位）
///
/// 该常量定义了任务卡片上显示子任务数量徽章的标准尺寸，
/// 确保所有徽章在视觉上保持一致。
pub const SUBTASK_BADGE_SIZE: f32 = 18.0;

/// 获取指定提供商的 logo SVG 句柄
///
/// 根据提供商 ID 返回对应的 logo 图标句柄，用于在任务卡片等位置
/// 显示任务所使用的 AI 提供商标识。
///
/// # 参数
///
/// - `provider_id` - 提供商的唯一标识符（例如 "openai"、"anthropic" 等）
///
/// # 返回值
///
/// 返回一个 `iced::widget::svg::Handle`，可用于创建 SVG 图标组件。
///
/// # 示例
///
/// ```ignore
/// let handle = provider_logo_handle("openai");
/// let icon = iced::widget::svg(handle);
/// ```
pub fn provider_logo_handle(provider_id: &str) -> iced::widget::svg::Handle {
    crate::app::assets::get_provider_icon(provider_id)
}

/// 获取自动选择图标（星形图标）
///
/// 返回用于表示"自动选择"功能的星形图标句柄。
/// 通常用于标识系统自动选择的任务或推荐项。
///
/// # 返回值
///
/// 返回一个 `iced::widget::svg::Handle`，表示星形图标。
///
/// # 示例
///
/// ```ignore
/// let icon_handle = auto_icon();
/// let icon_widget = iced::widget::svg(icon_handle);
/// ```
pub fn auto_icon() -> iced::widget::svg::Handle {
    assets::get_icon(Icon::Star)
}

/// 主要按钮样式
///
/// 定义任务看板中主要操作按钮的视觉样式。
/// 使用蓝色主题，并在悬停和按下状态下提供视觉反馈。
///
/// # 参数
///
/// - `_theme` - 当前 iced 主题（此样式不使用主题变量）
/// - `status` - 按钮的当前状态（Active、Hovered、Pressed、Disabled）
///
/// # 返回值
///
/// 返回一个 `iced::widget::button::Style` 结构体，定义按钮的背景色、
/// 文字颜色和边框样式。
///
/// # 颜色方案
///
/// - 默认状态：亮蓝色 (RGB: 59, 130, 246)
/// - 悬停状态：中蓝色 (RGB: 37, 99, 235)
/// - 按下状态：深蓝色 (RGB: 29, 78, 216)
/// - 边框圆角：6.0 像素
///
/// # 示例
///
/// ```ignore
/// use iced::widget::button;
///
/// let btn = button("保存")
///     .style(button_style_primary);
/// ```
pub fn button_style_primary(
    theme: &Theme,
    status: iced::widget::button::Status,
) -> iced::widget::button::Style {
    let mut style = primary_action_btn_style(theme, status);
    style.border.radius = 12.0.into();
    style.shadow = iced::Shadow {
        color: theme.palette().primary.scale_alpha(0.20),
        offset: iced::Vector::new(0.0, 10.0),
        blur_radius: 22.0,
    };
    style
}

/// 次要按钮样式
///
/// 定义任务看板中次要操作按钮的视觉样式。
/// 使用主题背景色，适合用于不那么突出的操作按钮。
///
/// # 参数
///
/// - `theme` - 当前 iced 主题，用于获取配色方案
/// - `status` - 按钮的当前状态（Active、Hovered、Pressed、Disabled）
///
/// # 返回值
///
/// 返回一个 `iced::widget::button::Style` 结构体，定义按钮的背景色、
/// 文字颜色和边框样式。
///
/// # 颜色方案
///
/// - 默认状态：主题基础背景色
/// - 悬停状态：主题弱背景色
/// - 按下状态：主题强背景色
/// - 边框：1.0 像素宽度，使用强背景色，圆角 6.0 像素
///
/// # 示例
///
/// ```ignore
/// use iced::widget::button;
///
/// let btn = button("取消")
///     .style(button_style_secondary);
/// ```
pub fn button_style_secondary(
    theme: &Theme,
    status: iced::widget::button::Status,
) -> iced::widget::button::Style {
    let mut style = rounded_action_btn_style(theme, status);
    style.border.radius = 12.0.into();
    style
}

/// 危险按钮样式
///
/// 定义任务看板中危险操作按钮的视觉样式（如删除、取消等）。
/// 使用红色主题，提醒用户该操作具有风险或不可逆。
///
/// # 参数
///
/// - `_theme` - 当前 iced 主题（此样式不使用主题变量）
/// - `status` - 按钮的当前状态（Active、Hovered、Pressed、Disabled）
///
/// # 返回值
///
/// 返回一个 `iced::widget::button::Style` 结构体，定义按钮的背景色、
/// 文字颜色和边框样式。
///
/// # 颜色方案
///
/// - 默认状态：红色 (RGB: 220, 38, 38)
/// - 悬停状态：深红色 (RGB: 185, 28, 28)
/// - 按下状态：更深红色 (RGB: 127, 29, 29)
/// - 边框圆角：6.0 像素
///
/// # 示例
///
/// ```ignore
/// use iced::widget::button;
///
/// let btn = button("删除")
///     .style(button_style_danger);
/// ```
pub fn button_style_danger(
    theme: &Theme,
    status: iced::widget::button::Status,
) -> iced::widget::button::Style {
    let mut style = danger_action_btn_style(theme, status);
    style.border.radius = 12.0.into();
    style
}

/// 成功按钮样式
///
/// 定义任务看板中成功/确认操作按钮的视觉样式。
/// 使用绿色主题，表示积极的、确认性的操作。
///
/// # 参数
///
/// - `_theme` - 当前 iced 主题（此样式不使用主题变量）
/// - `status` - 按钮的当前状态（Active、Hovered、Pressed、Disabled）
///
/// # 返回值
///
/// 返回一个 `iced::widget::button::Style` 结构体，定义按钮的背景色、
/// 文字颜色和边框样式。
///
/// # 颜色方案
///
/// - 默认状态：绿色 (RGB: 34, 197, 94)
/// - 悬停状态：深绿色 (RGB: 22, 101, 52)
/// - 按下状态：更深绿色 (RGB: 20, 83, 45)
/// - 边框圆角：6.0 像素
///
/// # 示例
///
/// ```ignore
/// use iced::widget::button;
///
/// let btn = button("确认")
///     .style(button_style_success);
/// ```
pub fn button_style_success(
    theme: &Theme,
    status: iced::widget::button::Status,
) -> iced::widget::button::Style {
    let bg = match status {
        iced::widget::button::Status::Hovered => Color::from_rgb8(22, 163, 74).scale_alpha(0.92),
        iced::widget::button::Status::Pressed => Color::from_rgb8(21, 128, 61).scale_alpha(0.94),
        _ => Color::from_rgb8(34, 197, 94).scale_alpha(0.90),
    };
    iced::widget::button::Style {
        background: Some(Background::Color(bg)),
        text_color: Color::WHITE,
        border: Border {
            radius: 12.0.into(),
            width: 1.0,
            color: Color::from_rgb8(22, 163, 74).scale_alpha(0.42),
        },
        shadow: iced::Shadow {
            color: theme.extended_palette().success.base.color.scale_alpha(0.16),
            offset: iced::Vector::new(0.0, 10.0),
            blur_radius: 22.0,
        },
        ..Default::default()
    }
}

#[cfg(test)]
#[path = "common_tests.rs"]
mod common_tests;

#[cfg(test)]
#[path = "common_ui_refresh_34_tests.rs"]
mod ui_refresh_34_tests;
