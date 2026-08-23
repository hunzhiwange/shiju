//! 验证迷你颜色选择器的通道编辑、格式切换与吸色状态构造。
//! 通道输入包含四舍五入、百分比钳制和非法值拒绝，避免编辑态产生越界颜色。

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use iced::Color;

use super::{
    HslaChannel, RgbaChannel, alpha_percent_text, color_with_alpha_percent_input,
    color_with_hsla_channel_input, color_with_rgba_channel_input, hsla_channel_values,
    render_mini_color_picker, rgba_channel_values,
};
use crate::app::Message;
use crate::app::components::color_picker::ColorFormat;

fn assert_color_close(actual: Color, expected: Color) {
    let epsilon = 0.001;
    assert!((actual.r - expected.r).abs() < epsilon, "red: {actual:?} != {expected:?}");
    assert!((actual.g - expected.g).abs() < epsilon, "green: {actual:?} != {expected:?}");
    assert!((actual.b - expected.b).abs() < epsilon, "blue: {actual:?} != {expected:?}");
    assert!((actual.a - expected.a).abs() < epsilon, "alpha: {actual:?} != {expected:?}");
}

#[test]
fn mini_alpha_helpers_format_parse_and_clamp_percent_values() {
    let color = Color::from_rgba(0.25, 0.5, 0.75, 0.4);

    assert_eq!(alpha_percent_text(color.a), "40");
    assert_color_close(
        color_with_alpha_percent_input(color, "125"),
        Color::from_rgba(0.25, 0.5, 0.75, 1.0),
    );
    assert_color_close(
        color_with_alpha_percent_input(color, "-10"),
        Color::from_rgba(0.25, 0.5, 0.75, 0.0),
    );
    assert_color_close(
        color_with_alpha_percent_input(color, "not a number"),
        Color::from_rgba(0.25, 0.5, 0.75, 1.0),
    );
}

#[test]
fn mini_rgba_channel_helpers_round_channels_and_reject_invalid_input() {
    let color = Color::from_rgba(0.1, 0.5, 0.9, 0.25);

    assert_eq!(rgba_channel_values(color), (26, 128, 230, 64));
    assert_color_close(
        color_with_rgba_channel_input(color, RgbaChannel::R, "255").expect("red update"),
        Color::from_rgba8(255, 128, 230, 0.25),
    );
    assert_color_close(
        color_with_rgba_channel_input(color, RgbaChannel::G, "").expect("empty green update"),
        Color::from_rgba8(26, 0, 230, 0.25),
    );
    assert_color_close(
        color_with_rgba_channel_input(color, RgbaChannel::B, "1").expect("blue update"),
        Color::from_rgba8(26, 128, 1, 0.25),
    );
    assert_color_close(
        color_with_rgba_channel_input(color, RgbaChannel::A, "128").expect("alpha update"),
        Color::from_rgba8(26, 128, 230, 128.0 / 255.0),
    );
    assert!(color_with_rgba_channel_input(color, RgbaChannel::R, "300").is_none());
    assert!(color_with_rgba_channel_input(color, RgbaChannel::R, "abc").is_none());
}

#[test]
fn mini_hsla_channel_helpers_round_channels_and_apply_updates() {
    let color = Color::from_rgba(0.2, 0.4, 0.6, 0.5);

    assert_eq!(hsla_channel_values(color), (210, 50, 40, 128));
    assert_color_close(
        color_with_hsla_channel_input(color, HslaChannel::H, "120").expect("hue update"),
        crate::app::components::color_picker::conversion::hsla_to_rgba(120.0, 0.5, 0.4, 0.5),
    );
    assert_color_close(
        color_with_hsla_channel_input(color, HslaChannel::S, "25").expect("saturation update"),
        crate::app::components::color_picker::conversion::hsla_to_rgba(210.0, 0.25, 0.4, 0.5),
    );
    assert_color_close(
        color_with_hsla_channel_input(color, HslaChannel::L, "80").expect("lightness update"),
        crate::app::components::color_picker::conversion::hsla_to_rgba(210.0, 0.5, 0.8, 0.5),
    );
    assert_color_close(
        color_with_hsla_channel_input(color, HslaChannel::A, "64").expect("alpha update"),
        crate::app::components::color_picker::conversion::hsla_to_rgba(
            210.0,
            0.5,
            0.4,
            64.0 / 255.0,
        ),
    );
    assert!(color_with_hsla_channel_input(color, HslaChannel::H, "abc").is_none());
}

#[test]
fn render_mini_color_picker_builds_every_format_and_wires_format_controls() {
    let selected_formats = Rc::new(RefCell::new(Vec::new()));
    let eyedropper_calls = Rc::new(Cell::new(0));
    let formats = [ColorFormat::Hex, ColorFormat::Rgba, ColorFormat::Hsl, ColorFormat::Css];

    for format in formats {
        let selected_formats_for_callback = selected_formats.clone();
        let eyedropper_calls_for_callback = eyedropper_calls.clone();
        let element = render_mini_color_picker(
            Color::from_rgba(0.9, 0.4, 0.1, 0.65),
            format,
            false,
            |_| Message::None,
            move |selected| {
                selected_formats_for_callback.borrow_mut().push(selected);
                Message::None
            },
            move || {
                eyedropper_calls_for_callback.set(eyedropper_calls_for_callback.get() + 1);
                Message::None
            },
        );
        std::hint::black_box(element);
    }

    assert_eq!(eyedropper_calls.get(), formats.len());
    assert_eq!(selected_formats.borrow().len(), formats.len() * formats.len());
    for format in formats {
        assert_eq!(selected_formats.borrow().iter().filter(|&&value| value == format).count(), 4);
    }
}

#[test]
fn render_mini_color_picker_builds_active_eyedropper_with_alpha_boundaries() {
    for color in [Color::from_rgba(0.0, 1.0, 0.0, 0.0), Color::from_rgba(1.0, 1.0, 1.0, 1.0)] {
        std::hint::black_box(render_mini_color_picker(
            color,
            ColorFormat::Css,
            true,
            |_| Message::None,
            |_| Message::None,
            || Message::None,
        ));
    }
}
