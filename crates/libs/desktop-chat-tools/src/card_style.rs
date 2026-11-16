use iced::{Background, Border};
use iced::widget::scrollable::{Direction, Scrollbar};
use iced::{Color, Theme, Length};
use iced::widget::{Svg, svg, button};
use desktop_assets::assets::{self, Icon};
pub fn is_dark_theme(theme: &Theme) -> bool {
    theme.palette().background.r + theme.palette().background.g + theme.palette().background.b < 1.5
}
pub fn chat_secondary_text_color(theme: &Theme) -> Color {
    if is_dark_theme(theme) {
        theme.palette().text.scale_alpha(0.92)
    } else {
        theme.extended_palette().secondary.base.text.scale_alpha(0.90)
    }
}
pub fn chat_secondary_subtle_text_color(theme: &Theme) -> Color {
    if is_dark_theme(theme) {
        theme.palette().text.scale_alpha(0.56)
    } else {
        theme.extended_palette().secondary.base.text.scale_alpha(0.58)
    }
}
pub fn bold_font() -> iced::Font { iced::Font { weight: iced::font::Weight::Bold, ..Default::default() } }
pub fn icon_svg<'a>(icon: Icon) -> Svg<'a> { Svg::new(assets::get_icon(icon)).width(Length::Fixed(14.0)).height(Length::Fixed(14.0)) }
pub use crate::text_layout::truncate_chars;
pub fn chat_secondary_muted_text_color(theme: &Theme) -> Color {
    if is_dark_theme(theme) {
        theme.palette().text.scale_alpha(0.72)
    } else {
        theme.extended_palette().secondary.base.text.scale_alpha(0.72)
    }
}
pub fn chat_scroll_direction() -> Direction {
    // Direction 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    Direction::Vertical(Scrollbar::new().width(4).scroller_width(4))
}
pub fn eye_icon_button_style(
    theme: &Theme,
    // status 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    _status: iced::widget::button::Status,
) -> iced::widget::button::Style {
    iced::widget::button::Style {
        background: None,
        // border 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        border: Border { width: 0.0, color: Color::TRANSPARENT, radius: 999.0.into() },
        // text_color 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        text_color: chat_secondary_text_color(theme),
        ..Default::default()
    }
}
pub fn eye_icon_svg_style(theme: &Theme, _status: svg::Status) -> svg::Style {
    // svg 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    svg::Style {
        // color 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        color: Some(if is_dark_theme(theme) {
            theme.palette().text.scale_alpha(0.92)
        } else {
            theme.extended_palette().secondary.base.text.scale_alpha(0.90)
        }),
    }
}
pub fn simplified_block_style(theme: &Theme) -> iced::widget::container::Style {
    let _ = theme;
    iced::widget::container::Style {
        background: None,
        border: Border { width: 0.0, color: Color::TRANSPARENT, radius: 0.0.into() },
        shadow: iced::Shadow::default(),
        ..Default::default()
    }
}
pub fn simplified_code_block_style(theme: &Theme) -> iced::widget::container::Style {
    let is_dark = is_dark_theme(theme);
    iced::widget::container::Style {
        // background 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        background: Some(Background::Color(if is_dark {
            // Color 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
            Color::from_rgba8(24, 25, 29, 0.90)
        } else {
            // Color 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
            Color::from_rgba8(247, 248, 250, 1.0)
        })),
        // border 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        border: Border {
            // width 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
            width: 1.0,
            // color 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
            color: if is_dark {
                // Color 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
                Color::from_rgba8(45, 48, 54, 0.90)
            } else {
                // Color 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
                Color::from_rgba8(226, 231, 237, 1.0)
            },
            // radius 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
            radius: 14.0.into(),
        },
        ..Default::default()
    }
}

use iced::{Element, Alignment};
use iced::widget::{button as button_widget, text};
pub fn file_button_style(
    theme: &Theme,
    // status 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    status: iced::widget::button::Status,
) -> iced::widget::button::Style {
    let mut style = button_widget::secondary(theme, status);
    style.border.radius = 10.0.into();
    style
}
pub fn additions_pill<'a, Message: 'static>(count: usize) -> Element<'a, Message> {
    let label = format!("+{}", count);
    iced::widget::container(text(label).size(11))
        .padding([2, 8])
        .style(|theme: &Theme| {
            let ext = theme.extended_palette();
            let fg = ext.success.base.color;
            let bg = fg.scale_alpha(0.16);
            iced::widget::container::Style {
                // background 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
                background: Some(Background::Color(bg)),
                // border 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
                border: Border { width: 0.0, color: Color::TRANSPARENT, radius: 999.0.into() },
                // text_color 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
                text_color: Some(fg),
                ..Default::default()
            }
        })
        .into()
}
pub fn deletions_pill<'a, Message: 'static>(count: usize) -> Element<'a, Message> {
    let label = format!("-{}", count);
    iced::widget::container(text(label).size(11))
        .padding([2, 8])
        .style(|theme: &Theme| {
            let ext = theme.extended_palette();
            let fg = ext.danger.base.color;
            let bg = fg.scale_alpha(0.14);
            iced::widget::container::Style {
                // background 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
                background: Some(Background::Color(bg)),
                // border 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
                border: Border { width: 0.0, color: Color::TRANSPARENT, radius: 999.0.into() },
                // text_color 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
                text_color: Some(fg),
                ..Default::default()
            }
        })
        .into()
}
pub fn change_pills<'a, Message: 'static>(adds: usize, dels: usize) -> Element<'a, Message> {
    // iced 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    iced::widget::row![additions_pill(adds), deletions_pill(dels)]
        .spacing(6)
        .align_y(Alignment::Center)
        .into()
}
pub use crate::text_layout::truncate_lines_middle;
pub fn single_line_preview(s: &str, max_chars: usize) -> String { let compact = s.split_whitespace().collect::<Vec<_>>().join(" "); truncate_chars(&compact, max_chars) }
pub fn chat_context_target_key(msg_idx: usize, sub_idx: Option<usize>) -> u64 { match sub_idx { Some(idx) => ((msg_idx as u64) << 32) | (idx as u64 + 1), None => (msg_idx as u64) << 32 } }
