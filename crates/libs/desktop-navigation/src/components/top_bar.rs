//! 顶部菜单栏组件。

pub mod actions;
pub use actions::{Message, Menu};
pub mod types;
pub mod controls;
pub mod drag;
pub mod external;
pub mod gateway;

pub mod widgets;

use iced::widget::{Row, container};
use iced::{Element, Length, Theme};
use widgets::color_with_alpha;

/// 顶部菜单栏的高度（像素）
///
/// 此常量定义了顶部菜单栏的固定高度，用于布局计算和样式设置。
pub const TOP_BAR_HEIGHT: f32 = 28.0;

pub fn view<'a, M: 'a>(items: Vec<Element<'a, M>>) -> Element<'a, M> {
    let bar = Row::with_children(items);
    container(bar.spacing(2).align_y(iced::Alignment::Center))
        .padding([0, 6])
        .height(Length::Fixed(TOP_BAR_HEIGHT))
        .style(top_bar_style)
        .into()
}

pub fn top_bar_style(theme: &Theme) -> container::Style {
    let palette = theme.extended_palette();
    container::Style {
        // 半透明背景，使用弱背景色
        background: Some(iced::Background::Color(color_with_alpha(
            palette.background.weak.color,
            0.88,
        ))),
        // 底部边框
        border: iced::Border {
            width: 1.0,
            color: palette.background.strong.color.scale_alpha(0.60),
            radius: 0.0.into(),
        },
        ..Default::default()
    }
}

#[cfg(test)] mod drag_tests;
#[cfg(test)] mod external_tests;

#[cfg(test)] mod controls_tests;

pub mod menus;
pub mod workspace_mode;

#[cfg(test)] mod menus_tests;

#[cfg(test)] mod workspace_mode_tests;

#[cfg(test)] mod widgets_tests;
