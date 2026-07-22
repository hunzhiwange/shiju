//! 颜色选择器属性模块，负责颜色解析、格式转换和拾色控件渲染。

use iced::widget::{
    Image, Space, button, canvas, column, container, row, stack, svg, text, text_input,
};
use iced::{Border, Color, ContentFit, Element, Length, Theme};

use super::conversion;
use super::{ColorFormat, Hsv, images, pickers, prop_text_input_style};
use crate::app::Message;
use crate::app::assets::{self, Icon};

fn rounded_track_style(_theme: &Theme) -> container::Style {
    container::Style {
        border: Border {
            radius: 999.0.into(),
            width: 1.0,
            color: Color::from_rgba8(255, 255, 255, 0.22),
        },
        ..Default::default()
    }
}

fn format_button_style(active: bool, theme: &Theme) -> button::Style {
    let palette = theme.extended_palette();
    let (bg, txt) = if active {
        (palette.primary.base.color, Color::WHITE)
    } else {
        (palette.background.weak.color, palette.background.weak.text)
    };
    button::Style {
        background: Some(bg.into()),
        text_color: txt,
        border: Border { radius: 20.0.into(), width: 0.0, color: Color::TRANSPARENT },
        ..button::Style::default()
    }
}

fn preview_style(color: Color) -> container::Style {
    container::Style {
        background: Some(color.into()),
        border: iced::Border {
            color: Color::from_rgb(0.8, 0.8, 0.8),
            width: 1.0,
            radius: 4.0.into(),
        },
        ..Default::default()
    }
}

fn eyedropper_button_style(picking: bool, hovered: bool) -> button::Style {
    if picking {
        button::Style {
            background: Some(Color::from_rgba8(117, 94, 255, 0.95).into()),
            text_color: Color::WHITE,
            border: Border {
                radius: 10.0.into(),
                width: 1.0,
                color: Color::from_rgba8(165, 148, 255, 0.95),
            },
            ..Default::default()
        }
    } else {
        button::Style {
            background: Some(
                (if hovered {
                    Color::from_rgba8(255, 255, 255, 0.14)
                } else {
                    Color::from_rgba8(255, 255, 255, 0.08)
                })
                .into(),
            ),
            text_color: Color::WHITE,
            border: Border {
                radius: 10.0.into(),
                width: 1.0,
                color: Color::from_rgba8(255, 255, 255, 0.20),
            },
            ..Default::default()
        }
    }
}

fn hex_input_container_style(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(Color::from_rgba8(255, 255, 255, 0.06).into()),
        border: Border {
            radius: 10.0.into(),
            width: 1.0,
            color: Color::from_rgba8(255, 255, 255, 0.18),
        },
        ..Default::default()
    }
}

fn parse_alpha_percent_input(input: &str) -> f32 {
    input.parse::<f32>().unwrap_or(100.0).clamp(0.0, 100.0) / 100.0
}

fn parse_rgba_channel_input(input: &str) -> Option<u8> {
    if input.is_empty() { Some(0) } else { input.parse::<u8>().ok() }
}

fn parse_hsla_channel_input(input: &str) -> Option<u16> {
    input.parse::<u16>().ok()
}

/// 渲染对应的设计界面片段。
///
/// 返回 Iced 元素；输入为空或不支持时由调用方保留现有界面兜底。
pub fn render_color_picker<'a, F, F2>(
    color: Color,
    format: ColorFormat,
    picking: bool,
    on_change: F,
    on_format_change: F2,
    on_eyedropper: impl Fn() -> Message + Clone + 'static,
) -> Element<'a, Message>
where
    F: Fn(Color) -> Message + Clone + 'static,
    F2: Fn(ColorFormat) -> Message + 'static,
{
    let hsv = Hsv::from_color(color);
    let a = color.a;

    let sv_picker = stack![
        Image::new(images::sv_image_handle(hsv.h))
            .content_fit(ContentFit::Fill)
            .width(Length::Fill)
            .height(150),
        canvas(pickers::SaturationValuePicker {
            hsv,
            on_change: Box::new({
                let on_change = on_change.clone();
                move |new_hsv| {
                    let new_color = new_hsv.to_color();
                    let final_color = Color { a, ..new_color };
                    on_change(final_color)
                }
            }),
        })
        .width(Length::Fill)
        .height(150)
    ]
    .width(Length::Fill)
    .height(150);

    let hue_picker = container(stack![
        Image::new(images::hue_image_handle())
            .content_fit(ContentFit::Fill)
            .width(Length::Fill)
            .height(16),
        canvas(pickers::HuePicker {
            hsv,
            on_change: Box::new({
                let on_change = on_change.clone();
                move |new_hsv| {
                    let new_color = new_hsv.to_color();
                    let final_color = Color { a, ..new_color };
                    on_change(final_color)
                }
            }),
        })
        .width(Length::Fill)
        .height(16)
    ])
    .width(Length::Fill)
    .height(16)
    .padding(1)
    .style(rounded_track_style);

    let alpha_percent = conversion::format_percent(a * 100.0);
    let alpha_bar = container(stack![
        Image::new(images::alpha_image_handle(Color { a: 1.0, ..color }))
            .content_fit(ContentFit::Fill)
            .width(Length::Fill)
            .height(16),
        canvas(pickers::AlphaPicker {
            rgb: Color { a: 1.0, ..color },
            alpha: a,
            on_change: Box::new({
                let on_change = on_change.clone();
                move |v| {
                    let new_color = Color { a: v, ..color };
                    on_change(new_color)
                }
            }),
        })
        .width(Length::Fill)
        .height(16)
    ])
    .width(Length::Fill)
    .height(16)
    .padding(1)
    .style(rounded_track_style);

    let a_slider = row![
        text("A").size(12).width(15),
        alpha_bar,
        text_input("100", &alpha_percent)
            .on_input({
                let on_change = on_change.clone();
                move |s| {
                    let new_color = Color { a: parse_alpha_percent_input(&s), ..color };
                    on_change(new_color)
                }
            })
            .style(prop_text_input_style)
            .width(Length::Fixed(52.0))
    ]
    .spacing(5)
    .align_y(iced::Alignment::Center);

    let formats = [ColorFormat::Hex, ColorFormat::Rgba, ColorFormat::Hsl, ColorFormat::Css];
    let mut format_switcher =
        row![].spacing(6).align_y(iced::Alignment::Center).width(Length::Fill);
    for f in formats {
        let active = f == format;
        let btn = button(
            text(f.to_string())
                .size(12)
                .width(Length::Fill)
                .align_x(iced::alignment::Horizontal::Center),
        )
        .style(move |theme: &Theme, _status| format_button_style(active, theme))
        .padding([6, 10])
        .width(Length::FillPortion(1))
        .on_press(on_format_change(f));
        format_switcher = format_switcher.push(btn);
    }

    let inputs: Element<'a, Message> = match format {
        ColorFormat::Hex => Space::new().into(),
        ColorFormat::Css => {
            let css_str = conversion::format_rgba_to_css(color.r, color.g, color.b, color.a);
            text_input::<Message, Theme, iced::Renderer>("CSS", &css_str)
                .on_input({
                    let on_change = on_change.clone();
                    move |s| {
                        if let Some(c) = conversion::parse_css_color(&s) {
                            on_change(c)
                        } else {
                            Message::None
                        }
                    }
                })
                .style(prop_text_input_style)
                .width(Length::Fill)
                .into()
        }
        ColorFormat::Rgba => {
            let r = (color.r * 255.0).round() as u8;
            let g = (color.g * 255.0).round() as u8;
            let b = (color.b * 255.0).round() as u8;
            let a_val = (color.a * 255.0).round() as u8;

            let input_field = |label: &str,
                               val: u8,
                               _max: u8,
                               cb: Box<dyn Fn(u8) -> Message>|
             -> iced::widget::Row<Message> {
                row![
                    text(label.to_string()).size(10),
                    text_input("", &val.to_string())
                        .on_input(move |s| {
                            parse_rgba_channel_input(&s).map_or(Message::None, &cb)
                        })
                        .style(prop_text_input_style)
                        .width(Length::Fill)
                ]
                .spacing(2)
                .align_y(iced::Alignment::Center)
            };

            let on_change = on_change.clone();

            row![
                input_field(
                    "R",
                    r,
                    255,
                    Box::new({
                        let on_change = on_change.clone();
                        move |v| on_change(Color::from_rgba8(v, g, b, color.a))
                    })
                ),
                input_field(
                    "G",
                    g,
                    255,
                    Box::new({
                        let on_change = on_change.clone();
                        move |v| on_change(Color::from_rgba8(r, v, b, color.a))
                    })
                ),
                input_field(
                    "B",
                    b,
                    255,
                    Box::new({
                        let on_change = on_change.clone();
                        move |v| on_change(Color::from_rgba8(r, g, v, color.a))
                    })
                ),
                input_field(
                    "A",
                    a_val,
                    255,
                    Box::new({
                        let on_change = on_change.clone();
                        move |v| on_change(Color::from_rgba8(r, g, b, v as f32 / 255.0))
                    })
                ),
            ]
            .spacing(5)
            .into()
        }
        ColorFormat::Hsl => {
            let (h, s, l, a_val) = conversion::rgba_to_hsla(color);
            let h_int = h.round() as u16;
            let s_int = (s * 100.0).round() as u8;
            let l_int = (l * 100.0).round() as u8;
            let a_int = (a_val * 255.0).round() as u8;

            let input_field = |label: &str,
                               val: u16,
                               _max: u16,
                               cb: Box<dyn Fn(u16) -> Message>|
             -> iced::widget::Row<Message> {
                row![
                    text(label.to_string()).size(10),
                    text_input("", &val.to_string())
                        .on_input(move |s| {
                            parse_hsla_channel_input(&s).map_or(Message::None, &cb)
                        })
                        .style(prop_text_input_style)
                        .width(Length::Fill)
                ]
                .spacing(2)
                .align_y(iced::Alignment::Center)
            };

            let on_change = on_change.clone();

            row![
                input_field(
                    "H",
                    h_int,
                    360,
                    Box::new({
                        let on_change = on_change.clone();
                        move |v| on_change(conversion::hsla_to_rgba(v as f32, s, l, a_val))
                    })
                ),
                input_field(
                    "S",
                    s_int as u16,
                    100,
                    Box::new({
                        let on_change = on_change.clone();
                        move |v| on_change(conversion::hsla_to_rgba(h, v as f32 / 100.0, l, a_val))
                    })
                ),
                input_field(
                    "L",
                    l_int as u16,
                    100,
                    Box::new({
                        let on_change = on_change.clone();
                        move |v| on_change(conversion::hsla_to_rgba(h, s, v as f32 / 100.0, a_val))
                    })
                ),
                input_field(
                    "A",
                    a_int as u16,
                    255,
                    Box::new({
                        let on_change = on_change.clone();
                        move |v| on_change(conversion::hsla_to_rgba(h, s, l, v as f32 / 255.0))
                    })
                ),
            ]
            .spacing(5)
            .into()
        }
    };

    let preview =
        container(Space::new().width(32).height(32)).style(move |_: &Theme| preview_style(color));

    let eyedropper = button(svg(assets::get_icon(Icon::Pen)).width(14).height(14))
        .style(move |_theme: &Theme, status| {
            let hovered = matches!(status, button::Status::Hovered | button::Status::Pressed);
            eyedropper_button_style(picking, hovered)
        })
        .padding(8)
        .on_press((on_eyedropper)());

    let controls_row = if format == ColorFormat::Hex {
        let hex_str = conversion::format_rgba_to_hex(color.r, color.g, color.b, color.a);
        row![
            preview,
            container(
                text_input::<Message, Theme, iced::Renderer>("HEX", &hex_str)
                    .on_input({
                        let on_change = on_change.clone();
                        move |s| {
                            if let Some(c) = conversion::parse_color(&s) {
                                on_change(c)
                            } else {
                                Message::None
                            }
                        }
                    })
                    .style(prop_text_input_style)
                    .width(Length::Fill)
            )
            .padding([2, 8])
            .style(hex_input_container_style)
            .width(Length::Fill),
            eyedropper
        ]
        .spacing(10)
        .align_y(iced::Alignment::Center)
    } else {
        row![preview, format_switcher, eyedropper].spacing(10).align_y(iced::Alignment::Center)
    };

    column![sv_picker, hue_picker, a_slider, controls_row, inputs].spacing(10).into()
}

#[cfg(test)]
#[path = "render_full_tests.rs"]
mod render_full_tests;
