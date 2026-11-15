use iced::{Background, Border, Color, Element, Length, Theme};
use iced::widget::{button, container, text, svg};
use iced::widget::tooltip::{Tooltip, Position as TooltipPosition};
pub fn overlay_icon_button_style(theme: &Theme, status: button::Status) -> button::Style {
    let palette = theme.extended_palette();
    let primary = palette.primary.base.color;
    let bg = match status {
        // button 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        button::Status::Hovered => {
            Some(Background::Color(Color::from_rgba(primary.r, primary.g, primary.b, 0.12)))
        }
        button::Status::Pressed => {
            Some(Background::Color(Color::from_rgba(primary.r, primary.g, primary.b, 0.20)))
        }
        _ => None,
    };
    let border_color = match status {
        // button 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        button::Status::Hovered => Color::from_rgba(primary.r, primary.g, primary.b, 0.28),
        // button 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        button::Status::Pressed => Color::from_rgba(primary.r, primary.g, primary.b, 0.36),
        _ => Color::TRANSPARENT,
    };

    button::Style {
        // background 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        background: bg,
        // text_color 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        text_color: theme.palette().text,
        // border 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        border: Border { width: 1.0, color: border_color, radius: 6.0.into() },
        // shadow 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        shadow: iced::Shadow::default(),
        ..Default::default()
    }
}

/// 构建或定位 fullscreen button overlay，用于把浮层稳定附着到目标控件。
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
pub fn preview_control_tooltip<'a, Message: Clone + 'a>(
    content: Element<'a, Message>,
    label: &'a str,
) -> Element<'a, Message> {
    let tip_content =
        container(text(label).size(12)).padding([4, 8]).style(fullscreen_tooltip_style);

    Tooltip::new(content, tip_content, TooltipPosition::Bottom).gap(6.0).into()
}

pub fn fullscreen_tooltip_style(theme: &Theme) -> iced::widget::container::Style {
    let palette = theme.extended_palette();
    let is_dark =
        theme.palette().background.r + theme.palette().background.g + theme.palette().background.b
            < 1.5;

    if is_dark {
        iced::widget::container::Style {
            text_color: Some(palette.background.strong.text),
            background: Some(Background::Color(palette.background.base.color)),
            border: Border {
                width: 1.0,
                color: palette.background.strong.color,
                radius: 4.0.into(),
            },
            shadow: iced::Shadow::default(),
            snap: false,
        }
    } else {
        iced::widget::container::Style {
            text_color: Some(Color::WHITE),
            background: Some(Background::Color(Color::from_rgba8(12, 13, 15, 0.97))),
            border: Border {
                width: 1.0,
                color: Color::from_rgba8(255, 255, 255, 0.08),
                radius: 10.0.into(),
            },
            shadow: iced::Shadow {
                color: Color::BLACK.scale_alpha(0.32),
                offset: iced::Vector::new(0.0, 6.0),
                blur_radius: 20.0,
            },
            snap: false,
        }
    }
}

pub fn fullscreen_icon_style(theme: &Theme) -> svg::Style {
    let bg = theme.palette().background;
    let is_dark = bg.r + bg.g + bg.b < 1.5;
    let palette = theme.extended_palette();
    svg::Style {
        color: Some(if is_dark {
            palette.background.strong.text.scale_alpha(0.88)
        } else {
            Color::from_rgba8(30, 41, 59, 0.78)
        }),
    }
}
