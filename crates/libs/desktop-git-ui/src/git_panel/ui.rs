use iced::{Element, Length, Border, Background, Color};
use iced::widget::{Space, button, container, text, svg, Svg, Tooltip};
use iced::widget::tooltip::Position as TooltipPosition;
use desktop_assets::assets::{Icon, get_icon};
use super::diff_view::context_menu::is_dark_theme;
pub fn icon_svg_sized(icon: Icon, size: f32) -> Svg<'static> {
    // Svg 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    Svg::new(get_icon(icon)).width(Length::Fixed(size)).height(Length::Fixed(size)).style(
        |theme: &iced::Theme, _status| svg::Style {
            // color 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
            color: Some(theme.palette().text),
        },
    )
}
pub fn with_tooltip<'a, Message: 'a>(
    content: impl Into<Element<'a, Message>>,
    // tip 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    tip: String,
    // compact 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    compact: bool,
    // gap 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    gap: f32,
) -> Element<'a, Message> {
    let tip_text = if compact { text(tip).size(12) } else { text(tip) };
    let radius = if compact { 4.0 } else { 8.0 };
    let padding = if compact { [4, 8] } else { [6, 10] };
    let tip_content = container(tip_text).padding(padding).style(move |theme: &iced::Theme| {
        let palette = theme.extended_palette();
        if is_dark_theme(theme) {
            iced::widget::container::Style {
                // text_color 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
                text_color: Some(palette.background.strong.text),
                // background 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
                background: Some(Background::Color(palette.background.base.color)),
                // border 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
                border: Border {
                    // width 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
                    width: 1.0,
                    // color 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
                    color: palette.background.strong.color,
                    // radius 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
                    radius: radius.into(),
                },
                // shadow 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
                shadow: iced::Shadow::default(),
                // snap 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
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
    });
    Tooltip::new(content, tip_content, TooltipPosition::Bottom).gap(gap).into()
}
pub fn small_plain_icon_button_style(
    theme: &iced::Theme,
    // status 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    status: button::Status,
) -> button::Style {
    let palette = theme.extended_palette();
    let background = match status {
        // button 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        button::Status::Pressed => Some(palette.background.strong.color.scale_alpha(0.18).into()),
        _ => None,
    };
    button::Style {
        background,
        // border 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        border: Border { radius: 6.0.into(), ..Default::default() },
        // text_color 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        text_color: palette.background.strong.text,
        ..Default::default()
    }
}
pub fn small_plain_icon_button<Message: Clone + 'static>(
    icon: Option<Icon>,
    // tip 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    tip: String,
    // on 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    on: Message,
) -> Element<'static, Message> {
    let content: Element<'static, Message> = if let Some(icon) = icon {
        icon_svg_sized(icon, 12.0).into()
    } else {
        // Space 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        Space::new().width(Length::Fixed(12.0)).height(Length::Fixed(12.0)).into()
    };
    let button = button(content)
        .on_press(on)
        .padding([2, 4])
        .width(Length::Fixed(20.0))
        .style(small_plain_icon_button_style);
    with_tooltip(button, tip, true, 4.0)
}
