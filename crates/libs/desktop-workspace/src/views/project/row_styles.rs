use iced::{Color, Background, Border, Theme};
use iced::widget::{container, text};
use iced::widget::container::Style as ContainerStyle;
use super::badge_styles::is_dark_theme;
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
pub fn tooltip_bubble<'a, Message: 'a>(tip: String) -> iced::widget::Container<'a, Message> {
    container(text(tip).size(12)).padding([6, 10]).style(tooltip_bubble_style)
}
