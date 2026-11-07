//! 系统设置和配置弹窗的内容切换标签。

use super::rounded_action_btn_style;
use iced::widget::{Button, Row, button, row, text};
use iced::{Alignment, Border, Element, Theme};

/// 使用常规设置的中性色调绘制标签。
pub fn settings_tab_style(theme: &Theme, status: button::Status, active: bool) -> button::Style {
    let mut style = rounded_action_btn_style(theme, status);
    let palette = theme.extended_palette();
    style.border = Border { radius: 10.0.into(), ..Default::default() };
    style.background = if active || matches!(status, button::Status::Pressed) {
        Some(palette.background.strong.color.scale_alpha(0.5).into())
    } else if matches!(status, button::Status::Hovered) {
        Some(palette.background.weak.color.into())
    } else {
        None
    };
    style.text_color = if active { theme.palette().text } else { palette.background.strong.text };
    style
}

/// 创建可按页面需要设置宽度的标签按钮。
pub fn settings_tab_button<'a, Message: Clone + 'a>(
    label: impl text::IntoFragment<'a>,
    selected: bool,
    on_press: Option<Message>,
) -> Button<'a, Message> {
    button(text(label).size(15))
        .padding([8, 14])
        .style(move |theme, status| settings_tab_style(theme, status, selected))
        .on_press_maybe(on_press)
}

/// 排列内容切换标签。调用方可按布局需要启用换行。
pub fn settings_tab_row<'a, Message>(
    tabs: impl IntoIterator<Item = Element<'a, Message>>,
) -> Row<'a, Message> {
    row(tabs).spacing(8).align_y(Alignment::Center)
}
