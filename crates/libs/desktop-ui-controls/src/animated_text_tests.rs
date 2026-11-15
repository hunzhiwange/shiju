//! 验证动画文字的颜色插值、闪烁周期与扫光边界。
//! 时间点 fixture 覆盖周期交界、禁用状态、空文本及单双字符的宽度退化情形。

use iced::{Color, Theme};

use super::{
    LEFT_TO_RIGHT_HIGHLIGHT_CYCLE_MS, animated_gradient_text_color, left_to_right_sweep_mix,
    mix_color, neutral_sweep_text_color,
};

const EPSILON: f32 = 0.000_001;

fn assert_float_close(actual: f32, expected: f32) {
    assert!(
        (actual - expected).abs() <= EPSILON,
        "actual {actual:?} was not close to expected {expected:?}"
    );
}

fn assert_color_close(actual: Color, expected: Color) {
    assert_float_close(actual.r, expected.r);
    assert_float_close(actual.g, expected.g);
    assert_float_close(actual.b, expected.b);
    assert_float_close(actual.a, expected.a);
}

#[test]
fn task_647_test_module_is_wired() {
    let path = std::path::Path::new(file!());
    assert!(path.ends_with("animated_text_tests.rs"));
}

#[test]
fn mix_color_clamps_ratio_and_interpolates_all_channels() {
    let first = Color::from_rgba(0.2, 0.4, 0.6, 0.8);
    let second = Color::from_rgba(0.8, 0.2, 0.0, 0.4);

    assert_color_close(mix_color(first, second, -1.0), first);
    assert_color_close(mix_color(first, second, 2.0), second);
    assert_color_close(mix_color(first, second, 0.25), Color::from_rgba(0.35, 0.35, 0.45, 0.7));
}

#[test]
fn animated_gradient_text_color_returns_static_theme_text_when_inactive() {
    assert_color_close(
        animated_gradient_text_color(&Theme::Dark, 0, false).expect("dark inactive color"),
        Theme::Dark.palette().text,
    );
    assert_color_close(
        animated_gradient_text_color(&Theme::Light, 0, false).expect("light inactive color"),
        Theme::Light.extended_palette().secondary.base.text,
    );
}

#[test]
fn animated_gradient_text_color_blinks_between_dark_theme_endpoints() {
    let even = animated_gradient_text_color(&Theme::Dark, 0, true).expect("even blink color");
    let odd = animated_gradient_text_color(&Theme::Dark, 240, true).expect("odd blink color");

    assert_color_close(
        even,
        mix_color(Theme::Dark.palette().primary.scale_alpha(0.98), Color::WHITE, 0.14),
    );
    assert_color_close(odd, mix_color(Color::WHITE.scale_alpha(0.35), Color::WHITE, 0.14));
}

#[test]
fn animated_gradient_text_color_blinks_between_light_theme_endpoints() {
    let even = animated_gradient_text_color(&Theme::Light, 0, true).expect("even blink color");
    let odd = animated_gradient_text_color(&Theme::Light, 240, true).expect("odd blink color");
    let base = Theme::Light.extended_palette().secondary.base.text;

    assert_color_close(
        even,
        mix_color(Theme::Light.palette().primary.scale_alpha(1.0), Color::WHITE, 0.16),
    );
    assert_color_close(odd, mix_color(base.scale_alpha(0.45), Color::WHITE, 0.16));
}

#[test]
fn animated_gradient_text_color_changes_at_blink_boundaries_and_repeats() {
    let first = animated_gradient_text_color(&Theme::Dark, 0, true).expect("first blink color");
    let first_end =
        animated_gradient_text_color(&Theme::Dark, 239, true).expect("first blink end color");
    let second = animated_gradient_text_color(&Theme::Dark, 240, true).expect("second blink color");
    let second_end =
        animated_gradient_text_color(&Theme::Dark, 479, true).expect("second blink end color");
    let repeated =
        animated_gradient_text_color(&Theme::Dark, 480, true).expect("repeated blink color");

    assert_color_close(first_end, first);
    assert_color_close(second_end, second);
    assert_color_close(repeated, first);
}

#[test]
fn left_to_right_sweep_mix_returns_zero_when_disabled_or_empty() {
    assert_float_close(left_to_right_sweep_mix(0, 0, 3, false), 0.0);
    assert_float_close(left_to_right_sweep_mix(0, 0, 0, true), 0.0);
}

#[test]
fn left_to_right_sweep_mix_peaks_for_single_character_at_cycle_middle() {
    assert_float_close(left_to_right_sweep_mix(0, 0, 1, true), 0.0);
    assert_float_close(
        left_to_right_sweep_mix(LEFT_TO_RIGHT_HIGHLIGHT_CYCLE_MS / 2, 0, 1, true),
        1.0,
    );
}

#[test]
fn left_to_right_sweep_mix_sweeps_across_multiple_characters() {
    let first_peak = left_to_right_sweep_mix(281, 0, 4, true);
    let last_peak = left_to_right_sweep_mix(1_719, 3, 4, true);
    let far_outside = left_to_right_sweep_mix(281, 9, 4, true);

    assert!(first_peak > 0.99, "first peak was {first_peak}");
    assert!(last_peak > 0.99, "last peak was {last_peak}");
    assert_float_close(far_outside, 0.0);
}

#[test]
fn left_to_right_sweep_mix_repeats_each_cycle() {
    for (now_ms, char_idx) in [(0, 0), (281, 0), (1_000, 2), (1_719, 3), (1_999, 3)] {
        assert_float_close(
            left_to_right_sweep_mix(now_ms, char_idx, 4, true),
            left_to_right_sweep_mix(now_ms + LEFT_TO_RIGHT_HIGHLIGHT_CYCLE_MS, char_idx, 4, true),
        );
    }
}

#[test]
fn left_to_right_sweep_mix_clamps_width_for_short_and_long_text() {
    let short_text_first_peak = left_to_right_sweep_mix(359, 0, 2, true);
    let long_text_first_peak = left_to_right_sweep_mix(242, 0, 8, true);

    assert!(short_text_first_peak > 0.99, "short text peak was {short_text_first_peak}");
    assert!(long_text_first_peak > 0.99, "long text peak was {long_text_first_peak}");
}

#[test]
fn neutral_sweep_text_color_returns_base_when_inactive_or_without_characters() {
    let base = Color::from_rgba(0.25, 0.5, 0.75, 0.8);

    assert_color_close(neutral_sweep_text_color(&Theme::Dark, base, 0, 0, 5, false), base);
    assert_color_close(neutral_sweep_text_color(&Theme::Light, base, 0, 0, 0, true), base);
}

#[test]
fn neutral_sweep_text_color_uses_white_highlight_for_dark_theme() {
    let base = Color::from_rgba(0.2, 0.3, 0.4, 0.7);
    let actual = neutral_sweep_text_color(
        &Theme::Dark,
        base,
        LEFT_TO_RIGHT_HIGHLIGHT_CYCLE_MS / 2,
        0,
        1,
        true,
    );

    assert_color_close(actual, mix_color(base, Color::WHITE, 0.42));
}

#[test]
fn neutral_sweep_text_color_uses_black_highlight_for_light_theme() {
    let base = Color::from_rgba(0.8, 0.7, 0.6, 0.9);
    let actual = neutral_sweep_text_color(
        &Theme::Light,
        base,
        LEFT_TO_RIGHT_HIGHLIGHT_CYCLE_MS / 2,
        0,
        1,
        true,
    );

    assert_color_close(actual, mix_color(base, Color::BLACK, 0.18));
}
