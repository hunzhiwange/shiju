use super::*;
use pretty_assertions::assert_eq;
use ratatui::style::Modifier;

#[test]
// 验证浅色终端背景上的状态色仍有足够对比度。
fn status_colors_preserve_light_terminal_themes() {
    for level in [StdoutColorLevel::TrueColor, StdoutColorLevel::Ansi256] {
        for bg in [(255, 255, 255), (130, 130, 130), (220, 210, 180)] {
            for (tone, color) in [
                (StatusTone::Success, Color::Green),
                (StatusTone::Attention, Color::Reset),
                (StatusTone::Failure, Color::Red),
            ] {
                assert_eq!(
                    status_style_for(tone, Some(bg), level),
                    Style::default().fg(color).bold(),
                );
            }
        }
        for bg in [(0, 0, 0), (0, 218, 0)] {
            assert_eq!(
                status_style_for(StatusTone::Attention, Some(bg), level),
                Style::default().fg(Color::Yellow).bold(),
            );
        }
        assert_eq!(
            status_style_for(StatusTone::Attention, /*terminal_bg*/ None, level),
            Style::default().fg(Color::Reset).bold(),
        );
    }
}

#[test]
// 验证 ANSI16 与无颜色能力回退不依赖真彩色。
fn status_colors_preserve_ansi16_and_no_color_fallbacks() {
    for (tone, light, dark) in [
        (StatusTone::Success, Color::Green, Color::Green),
        (StatusTone::Attention, Color::Reset, Color::Yellow),
        (StatusTone::Failure, Color::Red, Color::Red),
    ] {
        for (bg, expected) in [((255, 255, 255), light), ((0, 0, 0), dark)] {
            assert_eq!(
                status_style_for(tone, Some(bg), StdoutColorLevel::Ansi16),
                Style::default().fg(expected).bold()
            );
            assert_eq!(
                status_style_for(tone, Some(bg), StdoutColorLevel::Unknown),
                Style::default().fg(Color::Reset).bold()
            );
        }
    }
}

#[test]
// 验证浅色背景使用较深青色作为强调色。
fn accent_style_uses_darker_cyan_on_light_backgrounds() {
    let style = accent_style_for(Some((255, 255, 255)));

    assert_eq!(style.fg, Some(best_color(LIGHT_BG_ACCENT_RGB)));
    assert!(style.add_modifier.contains(Modifier::BOLD));
}

#[test]
// 验证暗色或未知背景沿用标准青色强调。
fn accent_style_uses_cyan_on_dark_or_unknown_backgrounds() {
    let expected = Style::default().fg(Color::Cyan).bold();

    assert_eq!(accent_style_for(Some((0, 0, 0))), expected);
    assert_eq!(accent_style_for(/*terminal_bg*/ None), expected);
}

#[test]
// 验证表格分隔色朝暗色背景混合而不过分抢眼。
fn table_separator_blends_toward_dark_background() {
    let style = table_separator_style_for(
        Some((255, 255, 255)),
        Some((0, 0, 0)),
        StdoutColorLevel::TrueColor,
    );

    assert_eq!(style.fg, Some(rgb_color((51, 51, 51))));
}

#[test]
// 验证表格分隔色在浅色背景上仍可见。
fn table_separator_blends_toward_light_background() {
    let style = table_separator_style_for(
        Some((0, 0, 0)),
        Some((255, 255, 255)),
        StdoutColorLevel::TrueColor,
    );

    assert_eq!(style.fg, Some(rgb_color((204, 204, 204))));
}

#[test]
// 验证无法感知调色板时以 dim 属性安全回退。
fn table_separator_dims_when_palette_aware_color_is_unavailable() {
    let expected = Style::default().dim();

    assert_eq!(
        table_separator_style_for(
            Some((255, 255, 255)),
            Some((0, 0, 0)),
            StdoutColorLevel::Ansi16,
        ),
        expected
    );
    assert_eq!(
        table_separator_style_for(
            /*terminal_fg*/ None,
            Some((0, 0, 0)),
            StdoutColorLevel::TrueColor,
        ),
        expected
    );
}
