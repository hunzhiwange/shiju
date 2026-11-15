use iced::{Background, Color, Theme};
use iced::widget::container;
/// 判断当前主题是否为暗色主题
///
/// 通过计算背景色的平均亮度来判断主题类型。
/// 当 RGB 三通道的平均值小于 0.35 时，认为是暗色主题。
///
/// # 参数
///
/// - `theme`: 当前 Iced 主题的引用
///
/// # 返回值
///
/// 如果是暗色主题返回 `true`，否则返回 `false`
///
/// # 示例
///
/// ```ignore
/// let theme = Theme::Dark;
/// if is_dark_theme(&theme) {
///     // 应用暗色主题样式
/// }
/// ```
pub(super) fn is_dark_theme(theme: &Theme) -> bool {
    let background = theme.palette().background;
    (background.r + background.g + background.b) / 3.0 < 0.35
}

/// 工作区操作按钮样式生成器
///
/// 为工作区的"重置"和"删除"按钮生成统一样式。
/// 样式会根据主题类型和按钮状态动态调整。
///
/// # 参数
///
/// - `theme`: 当前 Iced 主题的引用
/// - `status`: 按钮的当前状态（悬停、按下、禁用等）
///
/// # 返回值
///
/// 返回配置好的按钮样式对象
///
/// # 样式规则
///
/// - 悬停状态：红色背景 (RGB: 220, 38, 38)
/// - 按下状态：深红色背景 (RGB: 185, 28, 28)
/// - 禁用/默认状态：根据主题返回灰色背景
/// - 文字颜色始终为白色
/// - 圆角半径为 999.0（完全圆角）
pub(super) fn worktree_action_button_style(
    theme: &Theme,
    status: iced::widget::button::Status,
) -> iced::widget::button::Style {
    let bg = worktree_action_button_background(theme, status);
    iced::widget::button::Style {
        background: Some(Background::Color(bg)),
        text_color: Color::WHITE,
        border: iced::Border { width: 0.0, color: Color::TRANSPARENT, radius: 999.0.into() },
        ..Default::default()
    }
}

pub(super) fn worktree_action_button_background(theme: &Theme, status: iced::widget::button::Status) -> Color {
    let base = if is_dark_theme(theme) {
        Color::from_rgba(0.34, 0.36, 0.40, 0.92)
    } else {
        Color::from_rgba(0.72, 0.74, 0.78, 0.95)
    };
    match status {
        iced::widget::button::Status::Hovered => Color::from_rgb8(220, 38, 38),
        iced::widget::button::Status::Pressed => Color::from_rgb8(185, 28, 28),
        iced::widget::button::Status::Disabled => base,
        _ => base,
    }
}

pub(super) fn create_worktree_button_style(
    _theme: &Theme,
    status: iced::widget::button::Status,
) -> iced::widget::button::Style {
    let bg = match status {
        iced::widget::button::Status::Hovered => Color::from_rgb8(37, 99, 235),
        iced::widget::button::Status::Pressed => Color::from_rgb8(29, 78, 216),
        _ => Color::from_rgb8(59, 130, 246),
    };
    iced::widget::button::Style {
        background: Some(Background::Color(bg)),
        text_color: Color::WHITE,
        border: iced::Border { radius: 4.0.into(), ..iced::Border::default() },
        ..Default::default()
    }
}

pub(super) fn list_item_button_style(
    theme: &Theme,
    status: iced::widget::button::Status,
) -> iced::widget::button::Style {
    let p = theme.extended_palette();
    let bg = match status {
        iced::widget::button::Status::Hovered => p.background.weak.color,
        iced::widget::button::Status::Pressed => p.background.strong.color,
        _ => Color::TRANSPARENT,
    };
    iced::widget::button::Style {
        background: Some(Background::Color(bg)),
        text_color: theme.palette().text,
        border: iced::Border { radius: 4.0.into(), ..iced::Border::default() },
        ..Default::default()
    }
}

pub(super) fn neutral_button_style(
    theme: &Theme,
    status: iced::widget::button::Status,
) -> iced::widget::button::Style {
    let p = theme.extended_palette();
    let bg = match status {
        iced::widget::button::Status::Hovered => p.background.weak.color,
        iced::widget::button::Status::Pressed => p.background.strong.color,
        _ => p.background.weak.color,
    };
    iced::widget::button::Style {
        background: Some(Background::Color(bg)),
        text_color: theme.palette().text,
        border: iced::Border { width: 0.0, color: Color::TRANSPARENT, radius: 4.0.into() },
        ..Default::default()
    }
}

pub(super) fn close_button_style(
    theme: &Theme,
    status: iced::widget::button::Status,
) -> iced::widget::button::Style {
    let mut style = neutral_button_style(theme, status);
    style.border.radius = 999.0.into();
    style
}

pub(super) fn danger_soft_button_style(
    _theme: &Theme,
    status: iced::widget::button::Status,
) -> iced::widget::button::Style {
    let bg = match status {
        iced::widget::button::Status::Hovered => Color::from_rgb8(220, 38, 38).scale_alpha(0.18),
        iced::widget::button::Status::Pressed => Color::from_rgb8(220, 38, 38).scale_alpha(0.26),
        _ => Color::from_rgb8(220, 38, 38).scale_alpha(0.12),
    };
    iced::widget::button::Style {
        background: Some(Background::Color(bg)),
        text_color: Color::from_rgb8(220, 38, 38),
        border: iced::Border { width: 0.0, color: Color::TRANSPARENT, radius: 4.0.into() },
        ..Default::default()
    }
}

pub(super) fn confirmation_panel_style(theme: &Theme) -> container::Style {
    let p = theme.extended_palette();
    container::Style {
        background: Some(Background::Color(p.background.weak.color)),
        border: iced::Border { width: 1.0, color: p.background.strong.color, radius: 6.0.into() },
        ..Default::default()
    }
}

pub(super) fn picker_panel_style(theme: &Theme) -> container::Style {
    let p = theme.extended_palette();
    container::Style {
        background: Some(p.background.base.color.into()),
        border: iced::Border { width: 1.0, color: p.background.strong.color, radius: 10.0.into() },
        shadow: iced::Shadow {
            color: Color::BLACK.scale_alpha(0.24),
            offset: iced::Vector::new(0.0, 8.0),
            blur_radius: 24.0,
        },
        ..Default::default()
    }
}

pub(super) fn overlay_style(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(Color::from_rgba(0.04, 0.05, 0.07, 0.28))),
        ..Default::default()
    }
}
