use super::*;
use pretty_assertions::assert_eq;

#[test]
fn parses_osc_colors_with_bel_and_st() {
    assert_eq!(
        parse_osc_color(b"\x1B]10;rgb:ffff/8000/0000\x07", /*slot*/ 10),
        Some((255, 127, 0))
    );
    assert_eq!(
        parse_osc_color(b"\x1B]11;rgba:00/80/ff/ff\x1B\\", /*slot*/ 11),
        Some((0, 128, 255))
    );
}

#[test]
fn parses_two_and_four_digit_color_components() {
    assert_eq!(parse_osc_rgb("rgb:00/80/ff"), Some((0, 128, 255)));
    assert_eq!(parse_osc_rgb("rgba:ffff/8000/0000/ffff"), Some((255, 127, 0)));
}

#[test]
fn parses_default_colors_from_one_buffer() {
    assert_eq!(
        parse_default_colors(b"\x1B]10;rgb:eeee/eeee/eeee\x1B\\\x1B]11;rgb:1111/1111/1111\x07"),
        Some(DefaultColors { fg: (238, 238, 238), bg: (17, 17, 17) })
    );
    assert_eq!(
        parse_default_colors(b"\x1B]11;rgb:1111/1111/1111\x07\x1B]10;rgb:eeee/eeee/eeee\x1B\\"),
        Some(DefaultColors { fg: (238, 238, 238), bg: (17, 17, 17) })
    );
    assert_eq!(parse_default_colors(b"\x1B]10;rgb:eeee/eeee/eeee\x1B\\"), None);
}

#[test]
fn ignores_malformed_or_partial_default_color_responses() {
    assert_eq!(
        parse_default_colors(b"\x1B]10;rgb:eeee/eeee/eeee\x1B\\\x1B]11;rgb:nope\x07"),
        None
    );
    assert_eq!(
        parse_default_colors(b"\x1B]10;rgb:eeee/eeee/eeee\x1B\\\x1B]11;rgb:11/11/11/11\x07"),
        None
    );
    assert_eq!(
        parse_default_colors(b"\x1B]10;rgb:eeee/eeee/eeee\x1B\\\x1B]11;rgb:1111/1111/1111"),
        None
    );
}

#[test]
fn parses_default_colors_with_unrelated_bytes() {
    assert_eq!(
        parse_default_colors(
            b"typed\x1B]10;rgb:eeee/eeee/eeee\x1B\\noise\x1B]11;rgb:1111/1111/1111\x07"
        ),
        Some(DefaultColors { fg: (238, 238, 238), bg: (17, 17, 17) })
    );
}
