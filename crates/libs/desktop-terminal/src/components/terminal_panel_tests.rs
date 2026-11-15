use iced::{Theme, Background};
fn assert_close(actual: f32, expected: f32) {
    assert!((actual - expected).abs() < 0.001, "expected {expected}, got {actual}");
}

#[test]
fn terminal_surface_uses_theme_base_background() {
    for theme in [Theme::Dark, Theme::Light] {
        let style = super::terminal_surface_style(&theme);
        assert_eq!(
            style.background,
            Some(Background::Color(theme.extended_palette().background.base.color))
        );
    }
}

#[test]
fn terminal_content_bottom_padding_is_half_the_other_edges() {
    let padding = super::terminal_content_padding(1);

    assert_close(padding.top, 8.0);
    assert_close(padding.right, 8.0);
    assert_close(padding.bottom, 4.0);
    assert_close(padding.left, 8.01);
}

#[test]
fn terminal_tab_width_counts_unicode_scalars_and_clamps_bounds() {
    assert_close(super::terminal_tab_width(""), 92.0);
    assert_close(super::terminal_tab_width("终端标签名称一二"), 129.2);
    assert_close(super::terminal_tab_width(&"界".repeat(40)), 260.0);
}

#[test]
fn terminal_tab_width_counts_combining_marks_and_emoji_as_scalars() {
    assert_close(super::terminal_tab_width("e\u{301}"), 92.0);
    assert_close(super::terminal_tab_width("😀😀😀😀"), 95.6);
}

#[test]
fn terminal_tab_active_state_defaults_to_first_or_matches_requested_id() {
    assert!(super::terminal_tab_is_active(None, 7, 0));
    assert!(!super::terminal_tab_is_active(None, 8, 1));
    assert!(!super::terminal_tab_is_active(Some(8), 7, 0));
    assert!(super::terminal_tab_is_active(Some(8), 8, 1));
}

#[test]
fn active_terminal_tab_index_prefers_match_then_falls_back_to_first() {
    assert_eq!(
        super::active_terminal_tab_index(None, std::iter::empty::<u64>()),
        None,
    );
    assert_eq!(super::active_terminal_tab_index(None, [7, 8]), Some(0));
    assert_eq!(super::active_terminal_tab_index(Some(8), [7, 8]), Some(1));
    assert_eq!(super::active_terminal_tab_index(Some(9), [7, 8]), Some(0));
}

#[test]
fn active_terminal_tab_index_returns_first_duplicate_match() {
    assert_eq!(super::active_terminal_tab_index(Some(7), [7, 8, 7]), Some(0),);
}
