//! 验证颜色字符串解析、RGBA 与 HSLA 转换以及百分比格式化边界。
//! 输入覆盖裁剪空白、通道越界、灰阶和各主色分支，非法格式必须明确拒绝。

use super::*;

fn assert_close(actual: f32, expected: f32) {
    assert!((actual - expected).abs() < 0.0001, "expected {expected}, got {actual}");
}

fn assert_color_close(actual: Color, expected: Color) {
    assert_close(actual.r, expected.r);
    assert_close(actual.g, expected.g);
    assert_close(actual.b, expected.b);
    assert_close(actual.a, expected.a);
}

#[test]
fn rgba_formatters_round_channels_and_preserve_alpha_representation() {
    assert_eq!(format_rgba_to_hex(0.0, 0.5, 1.0, 0.25), "#0080FF40");
    assert_eq!(format_rgba_to_css(0.0, 0.5, 1.0, 0.256), "rgba(0, 128, 255, 0.26)");
}

#[test]
fn parse_css_color_accepts_trimmed_rgb_and_rgba_inputs() {
    assert_color_close(
        parse_css_color("  rgb(12, 34, 56)  ").expect("valid rgb"),
        Color::from_rgb8(12, 34, 56),
    );
    assert_color_close(
        parse_css_color("rgba(12, 34, 56, 0.25)").expect("valid rgba"),
        Color::from_rgba8(12, 34, 56, 0.25),
    );
}

#[test]
fn parse_css_color_rejects_unsupported_malformed_and_out_of_range_inputs() {
    for input in [
        "#0C2238",
        "rgb(12, 34)",
        "rgba(12, 34, 56)",
        "rgb(256, 34, 56)",
        "rgba(12, nope, 56, 1)",
        "rgb(12, 34, 56",
    ] {
        assert!(parse_css_color(input).is_none(), "unexpectedly parsed {input}");
    }
}

#[test]
fn parse_color_supports_rgb_and_rgba_hex_and_rejects_invalid_values() {
    assert_color_close(
        parse_color("#0C2238").expect("valid rgb hex"),
        Color::from_rgba8(12, 34, 56, 1.0),
    );
    assert_color_close(
        parse_color("#0C223880").expect("valid rgba hex"),
        Color::from_rgba8(12, 34, 56, 128.0 / 255.0),
    );

    for input in ["0C2238", "#12345", "#GG2238", "#0C2238GG"] {
        assert!(parse_color(input).is_none(), "unexpectedly parsed {input}");
    }
}

#[test]
fn rgba_to_hsla_covers_gray_and_each_dominant_channel() {
    let cases = [
        (Color::from_rgba(0.5, 0.5, 0.5, 0.4), (0.0, 0.0, 0.5, 0.4)),
        (Color::from_rgb(1.0, 0.0, 0.5), (330.0, 1.0, 0.5, 1.0)),
        (Color::from_rgb(0.0, 1.0, 0.0), (120.0, 1.0, 0.5, 1.0)),
        (Color::from_rgb(0.0, 0.0, 1.0), (240.0, 1.0, 0.5, 1.0)),
        (Color::from_rgb(0.8, 0.6, 0.6), (0.0, 1.0 / 3.0, 0.7, 1.0)),
    ];

    for (color, expected) in cases {
        let actual = rgba_to_hsla(color);
        assert_close(actual.0, expected.0);
        assert_close(actual.1, expected.1);
        assert_close(actual.2, expected.2);
        assert_close(actual.3, expected.3);
    }
}

#[test]
fn hsla_to_rgba_maps_all_hue_sectors_and_alpha() {
    let cases = [
        (0.0, Color::from_rgba(1.0, 0.0, 0.0, 0.3)),
        (60.0, Color::from_rgba(1.0, 1.0, 0.0, 0.3)),
        (120.0, Color::from_rgba(0.0, 1.0, 0.0, 0.3)),
        (180.0, Color::from_rgba(0.0, 1.0, 1.0, 0.3)),
        (240.0, Color::from_rgba(0.0, 0.0, 1.0, 0.3)),
        (300.0, Color::from_rgba(1.0, 0.0, 1.0, 0.3)),
    ];

    for (hue, expected) in cases {
        assert_color_close(hsla_to_rgba(hue, 1.0, 0.5, 0.3), expected);
    }
}

#[test]
fn format_percent_removes_only_redundant_fractional_zeroes() {
    assert_eq!(format_percent(12.0), "12");
    assert_eq!(format_percent(12.5), "12.5");
    assert_eq!(format_percent(12.345), "12.35");
}
