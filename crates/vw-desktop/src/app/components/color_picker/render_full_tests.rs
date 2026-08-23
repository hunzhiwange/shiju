//! 验证完整颜色选择器的格式切换、吸色状态、输入边界与样式构造。
//! 每种颜色格式都走真实控件树，透明度端点和激活吸色器用于覆盖条件分支。

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use iced::Color;

use super::{
    eyedropper_button_style, format_button_style, hex_input_container_style,
    parse_alpha_percent_input, parse_hsla_channel_input, parse_rgba_channel_input, preview_style,
    render_color_picker, rounded_track_style,
};
use crate::app::Message;
use crate::app::components::color_picker::ColorFormat;

#[test]
fn render_color_picker_builds_every_format_and_wires_format_controls() {
    let selected_formats = Rc::new(RefCell::new(Vec::new()));
    let eyedropper_calls = Rc::new(Cell::new(0));
    let formats = [ColorFormat::Hex, ColorFormat::Rgba, ColorFormat::Hsl, ColorFormat::Css];

    for format in formats {
        let selected_formats_for_callback = selected_formats.clone();
        let eyedropper_calls_for_callback = eyedropper_calls.clone();
        let element = render_color_picker(
            Color::from_rgba(0.25, 0.5, 0.75, 0.4),
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
fn render_color_picker_builds_active_eyedropper_with_alpha_boundaries() {
    for color in [Color::from_rgba(1.0, 0.0, 0.0, 0.0), Color::from_rgba(0.0, 0.0, 1.0, 1.0)] {
        std::hint::black_box(render_color_picker(
            color,
            ColorFormat::Hex,
            true,
            |_| Message::None,
            |_| Message::None,
            || Message::None,
        ));
    }
}

#[test]
fn color_picker_private_helpers_cover_input_boundaries_and_styles() {
    assert_eq!(parse_alpha_percent_input("-4"), 0.0);
    assert_eq!(parse_alpha_percent_input("25"), 0.25);
    assert_eq!(parse_alpha_percent_input("140"), 1.0);
    assert_eq!(parse_alpha_percent_input("not-a-percent"), 1.0);

    assert_eq!(parse_rgba_channel_input(""), Some(0));
    assert_eq!(parse_rgba_channel_input("255"), Some(255));
    assert_eq!(parse_rgba_channel_input("256"), None);
    assert_eq!(parse_rgba_channel_input("red"), None);

    assert_eq!(parse_hsla_channel_input("360"), Some(360));
    assert_eq!(parse_hsla_channel_input("65536"), None);
    assert_eq!(parse_hsla_channel_input("blue"), None);

    let active_format = format_button_style(true, &iced::Theme::Dark);
    let inactive_format = format_button_style(false, &iced::Theme::Dark);
    assert_eq!(active_format.text_color, Color::WHITE);
    assert_ne!(active_format.background, inactive_format.background);

    let picking = eyedropper_button_style(true, false);
    let idle = eyedropper_button_style(false, false);
    let hovered = eyedropper_button_style(false, true);
    assert_eq!(picking.text_color, Color::WHITE);
    assert_ne!(idle.background, hovered.background);

    let track = rounded_track_style(&iced::Theme::Dark);
    assert_eq!(track.border.width, 1.0);

    let preview = preview_style(Color::from_rgba(0.1, 0.2, 0.3, 0.4));
    assert!(preview.background.is_some());
    assert_eq!(preview.border.radius, 4.0.into());

    let hex = hex_input_container_style(&iced::Theme::Dark);
    assert!(hex.background.is_some());
    assert_eq!(hex.border.radius, 10.0.into());
}
