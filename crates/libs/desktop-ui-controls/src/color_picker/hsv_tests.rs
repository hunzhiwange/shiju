//! 验证 HSV 与 RGB 互转覆盖灰阶、负色相和全部色相分区。
//! 浮点断言允许极小误差，但黑色零饱和度和色相环绕的语义保持精确。

use super::Hsv;
use iced::Color;

fn assert_close(actual: f32, expected: f32) {
    assert!((actual - expected).abs() < 1e-6, "expected {expected}, got {actual}");
}

fn assert_color_close(color: Color, r: f32, g: f32, b: f32) {
    assert_close(color.r, r);
    assert_close(color.g, g);
    assert_close(color.b, b);
    assert_close(color.a, 1.0);
}

#[test]
fn from_color_wraps_negative_red_sector_hue() {
    let hsv = Hsv::from_color(Color::from_rgb(1.0, 0.0, 1.0));

    assert_close(hsv.h, 300.0);
    assert_close(hsv.s, 1.0);
    assert_close(hsv.v, 1.0);
}

#[test]
fn from_color_treats_black_as_zero_saturation() {
    let hsv = Hsv::from_color(Color::BLACK);

    assert_close(hsv.h, 0.0);
    assert_close(hsv.s, 0.0);
    assert_close(hsv.v, 0.0);
}

#[test]
fn from_color_covers_green_blue_and_gray_branches() {
    let green = Hsv::from_color(Color::from_rgb(0.25, 1.0, 0.5));
    assert_close(green.h, 140.0);
    assert_close(green.s, 0.75);
    assert_close(green.v, 1.0);

    let blue = Hsv::from_color(Color::from_rgb(0.25, 0.5, 1.0));
    assert_close(blue.h, 220.0);
    assert_close(blue.s, 0.75);
    assert_close(blue.v, 1.0);

    let gray = Hsv::from_color(Color::from_rgb(0.42, 0.42, 0.42));
    assert_close(gray.h, 0.0);
    assert_close(gray.s, 0.0);
    assert_close(gray.v, 0.42);
}

#[test]
fn to_color_maps_every_hue_sector() {
    for (hsv, expected) in [
        (Hsv { h: 30.0, s: 1.0, v: 0.8 }, (0.8, 0.4, 0.0)),
        (Hsv { h: 90.0, s: 1.0, v: 0.8 }, (0.4, 0.8, 0.0)),
        (Hsv { h: 150.0, s: 1.0, v: 0.8 }, (0.0, 0.8, 0.4)),
        (Hsv { h: 210.0, s: 1.0, v: 0.8 }, (0.0, 0.4, 0.8)),
        (Hsv { h: 270.0, s: 1.0, v: 0.8 }, (0.4, 0.0, 0.8)),
        (Hsv { h: 330.0, s: 1.0, v: 0.8 }, (0.8, 0.0, 0.4)),
    ] {
        let (r, g, b) = expected;
        assert_color_close(hsv.to_color(), r, g, b);
    }
}

#[test]
fn to_color_uses_value_as_gray_when_saturation_is_zero() {
    assert_color_close(Hsv { h: 240.0, s: 0.0, v: 0.35 }.to_color(), 0.35, 0.35, 0.35);
}
