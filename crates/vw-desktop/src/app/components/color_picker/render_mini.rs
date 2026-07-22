//! 颜色选择器属性模块，负责颜色解析、格式转换和拾色控件渲染。

use iced::widget::{
    Image, Space, button, canvas, column, container, row, stack, svg, text, text_input,
};
use iced::{Border, Color, ContentFit, Element, Length, Theme};

use super::conversion;
use super::{ColorFormat, Hsv, images, pickers, prop_text_input_style};
use crate::app::Message;
use crate::app::assets::{self, Icon};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum RgbaChannel {
    R,
    G,
    B,
    A,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum HslaChannel {
    H,
    S,
    L,
    A,
}

pub(super) fn alpha_percent_text(alpha: f32) -> String {
    conversion::format_percent(alpha * 100.0)
}

pub(super) fn color_with_alpha_percent_input(color: Color, input: &str) -> Color {
    let val = input.parse::<f32>().unwrap_or(100.0).clamp(0.0, 100.0);
    Color { a: val / 100.0, ..color }
}

pub(super) fn rgba_channel_values(color: Color) -> (u8, u8, u8, u8) {
    (
        (color.r * 255.0).round() as u8,
        (color.g * 255.0).round() as u8,
        (color.b * 255.0).round() as u8,
        (color.a * 255.0).round() as u8,
    )
}

pub(super) fn color_with_rgba_channel_input(
    color: Color,
    channel: RgbaChannel,
    input: &str,
) -> Option<Color> {
    let value = if input.is_empty() { Some(0) } else { input.parse::<u8>().ok() }?;
    let (r, g, b, _a) = rgba_channel_values(color);
    Some(match channel {
        RgbaChannel::R => Color::from_rgba8(value, g, b, color.a),
        RgbaChannel::G => Color::from_rgba8(r, value, b, color.a),
        RgbaChannel::B => Color::from_rgba8(r, g, value, color.a),
        RgbaChannel::A => Color::from_rgba8(r, g, b, value as f32 / 255.0),
    })
}

pub(super) fn hsla_channel_values(color: Color) -> (u16, u8, u8, u8) {
    let (h, s, l, a) = conversion::rgba_to_hsla(color);
    (
        h.round() as u16,
        (s * 100.0).round() as u8,
        (l * 100.0).round() as u8,
        (a * 255.0).round() as u8,
    )
}

pub(super) fn color_with_hsla_channel_input(
    color: Color,
    channel: HslaChannel,
    input: &str,
) -> Option<Color> {
    let value = input.parse::<u16>().ok()?;
    let (h, s, l, a) = conversion::rgba_to_hsla(color);
    Some(match channel {
        HslaChannel::H => conversion::hsla_to_rgba(value as f32, s, l, a),
        HslaChannel::S => conversion::hsla_to_rgba(h, value as f32 / 100.0, l, a),
        HslaChannel::L => conversion::hsla_to_rgba(h, s, value as f32 / 100.0, a),
        HslaChannel::A => conversion::hsla_to_rgba(h, s, l, value as f32 / 255.0),
    })
}

/// 渲染对应的设计界面片段。
///
/// 返回 Iced 元素；输入为空或不支持时由调用方保留现有界面兜底。
pub fn render_mini_color_picker<'a, F, F2>(
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
            .height(90),
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
        .height(90)
    ]
    .width(Length::Fill)
    .height(90);

    let hue_picker = stack![
        Image::new(images::hue_image_handle())
            .content_fit(ContentFit::Fill)
            .width(Length::Fill)
            .height(12),
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
        .height(12)
    ]
    .width(Length::Fill)
    .height(12);

    let alpha_percent = alpha_percent_text(a);
    let alpha_bar = stack![
        Image::new(images::alpha_image_handle(Color { a: 1.0, ..color }))
            .content_fit(ContentFit::Fill)
            .width(Length::Fill)
            .height(12),
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
        .height(12)
    ]
    .width(Length::Fill)
    .height(12);

    let a_slider = row![
        text("A").size(11).width(10),
        alpha_bar,
        text_input("100", &alpha_percent)
            .on_input({
                let on_change = on_change.clone();
                move |s| on_change(color_with_alpha_percent_input(color, &s))
            })
            .style(prop_text_input_style)
            .width(Length::Fixed(40.0))
            .size(11)
    ]
    .spacing(5)
    .align_y(iced::Alignment::Center);

    let formats = [ColorFormat::Hex, ColorFormat::Rgba, ColorFormat::Hsl, ColorFormat::Css];
    let mut format_switcher =
        row![].spacing(4).align_y(iced::Alignment::Center).width(Length::Fill);
    for f in formats {
        let active = f == format;
        let btn = button(
            text(f.to_string())
                .size(11)
                .width(Length::Fill)
                .align_x(iced::alignment::Horizontal::Center),
        )
        .style(move |theme: &Theme, _status| {
            let palette = theme.extended_palette();
            let (bg, txt) = if active {
                (palette.primary.base.color, Color::WHITE)
            } else {
                (palette.background.weak.color, palette.background.strong.text)
            };
            button::Style {
                background: Some(bg.into()),
                text_color: txt,
                border: Border { radius: 20.0.into(), width: 0.0, color: Color::TRANSPARENT },
                ..button::Style::default()
            }
        })
        .padding([2, 0])
        .width(Length::FillPortion(1))
        .on_press(on_format_change(f));
        format_switcher = format_switcher.push(btn);
    }

    let inputs: Element<'a, Message> = match format {
        ColorFormat::Hex => {
            let hex_str = conversion::format_rgba_to_hex(color.r, color.g, color.b, color.a);
            text_input::<Message, Theme, iced::Renderer>("Hex", &hex_str)
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
                .size(11)
                .into()
        }
        _ => match format {
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
                    .size(11)
                    .into()
            }
            ColorFormat::Rgba => {
                let (r, g, b, a_val) = rgba_channel_values(color);

                let input_field = |label: &str,
                                   val: u8,
                                   _max: u8,
                                   cb: Box<dyn Fn(u8) -> Message>|
                 -> iced::widget::Row<Message> {
                    row![
                        text(label.to_string()).size(9),
                        text_input("", &val.to_string())
                            .on_input(move |s| {
                                if let Some(v) =
                                    if s.is_empty() { Some(0) } else { s.parse::<u8>().ok() }
                                {
                                    cb(v)
                                } else {
                                    Message::None
                                }
                            })
                            .style(prop_text_input_style)
                            .width(Length::Fill)
                            .size(10)
                    ]
                    .spacing(1)
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
                            move |v| {
                                color_with_rgba_channel_input(color, RgbaChannel::R, &v.to_string())
                                    .map_or(Message::None, &on_change)
                            }
                        })
                    ),
                    input_field(
                        "G",
                        g,
                        255,
                        Box::new({
                            let on_change = on_change.clone();
                            move |v| {
                                color_with_rgba_channel_input(color, RgbaChannel::G, &v.to_string())
                                    .map_or(Message::None, &on_change)
                            }
                        })
                    ),
                    input_field(
                        "B",
                        b,
                        255,
                        Box::new({
                            let on_change = on_change.clone();
                            move |v| {
                                color_with_rgba_channel_input(color, RgbaChannel::B, &v.to_string())
                                    .map_or(Message::None, &on_change)
                            }
                        })
                    ),
                    input_field(
                        "A",
                        a_val,
                        255,
                        Box::new({
                            let on_change = on_change.clone();
                            move |v| {
                                color_with_rgba_channel_input(color, RgbaChannel::A, &v.to_string())
                                    .map_or(Message::None, &on_change)
                            }
                        })
                    ),
                ]
                .spacing(2)
                .into()
            }
            ColorFormat::Hsl => {
                let (h_int, s_int, l_int, a_int) = hsla_channel_values(color);

                let input_field = |label: &str,
                                   val: u16,
                                   _max: u16,
                                   cb: Box<dyn Fn(u16) -> Message>|
                 -> iced::widget::Row<Message> {
                    row![
                        text(label.to_string()).size(9),
                        text_input("", &val.to_string())
                            .on_input(move |s| if let Ok(v) = s.parse::<u16>() {
                                cb(v)
                            } else {
                                Message::None
                            })
                            .style(prop_text_input_style)
                            .width(Length::Fill)
                            .size(10)
                    ]
                    .spacing(1)
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
                            move |v| {
                                color_with_hsla_channel_input(color, HslaChannel::H, &v.to_string())
                                    .map_or(Message::None, &on_change)
                            }
                        })
                    ),
                    input_field(
                        "S",
                        s_int as u16,
                        100,
                        Box::new({
                            let on_change = on_change.clone();
                            move |v| {
                                color_with_hsla_channel_input(color, HslaChannel::S, &v.to_string())
                                    .map_or(Message::None, &on_change)
                            }
                        })
                    ),
                    input_field(
                        "L",
                        l_int as u16,
                        100,
                        Box::new({
                            let on_change = on_change.clone();
                            move |v| {
                                color_with_hsla_channel_input(color, HslaChannel::L, &v.to_string())
                                    .map_or(Message::None, &on_change)
                            }
                        })
                    ),
                    input_field(
                        "A",
                        a_int as u16,
                        255,
                        Box::new({
                            let on_change = on_change.clone();
                            move |v| {
                                color_with_hsla_channel_input(color, HslaChannel::A, &v.to_string())
                                    .map_or(Message::None, &on_change)
                            }
                        })
                    ),
                ]
                .spacing(2)
                .into()
            }
            _ => text("").into(),
        },
    };

    let preview =
        container(Space::new().width(20).height(20)).style(move |_: &Theme| container::Style {
            background: Some(color.into()),
            border: iced::Border {
                color: Color::from_rgb(0.8, 0.8, 0.8),
                width: 1.0,
                radius: 3.0.into(),
            },
            ..Default::default()
        });

    let eyedropper = button(svg(assets::get_icon(Icon::Pen)).width(10).height(10))
        .style(if picking { button::primary } else { button::secondary })
        .padding(4)
        .on_press((on_eyedropper)());

    let controls_row =
        row![preview, format_switcher, eyedropper].spacing(5).align_y(iced::Alignment::Center);

    column![sv_picker, hue_picker, a_slider, controls_row, inputs].spacing(5).into()
}

#[cfg(test)]
#[path = "render_mini_tests.rs"]
mod render_mini_tests;
