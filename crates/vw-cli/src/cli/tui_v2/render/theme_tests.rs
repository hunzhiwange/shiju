use ratatui::style::Color;

use super::test_support::render_tui_snapshot_with_theme;
use super::{TuiPanelSurface, TuiTheme, TuiThemeName};
use crate::cli::tui_v2::state::TuiState;
use crate::cli::tui_v2::terminal_capabilities::{
    TerminalCapabilities, TuiBackground, TuiColorLevel,
};

#[test]
fn tui_theme_matches_reference_semantics_across_terminal_capabilities() {
    let truecolor = TerminalCapabilities {
        color_level: TuiColorLevel::TrueColor,
        background: TuiBackground::Dark,
        apple_terminal: false,
        osc8_hyperlinks: false,
    };
    let mappings = [
        (TuiThemeName::Dark, Color::Rgb(215, 119, 87), Color::Rgb(78, 186, 101)),
        (TuiThemeName::Light, Color::Rgb(215, 119, 87), Color::Rgb(44, 122, 57)),
        (TuiThemeName::DarkDaltonized, Color::Rgb(255, 153, 51), Color::Rgb(51, 153, 255)),
        (TuiThemeName::LightDaltonized, Color::Rgb(255, 153, 51), Color::Rgb(0, 102, 153)),
        (TuiThemeName::DarkAnsi, Color::LightRed, Color::LightGreen),
        (TuiThemeName::LightAnsi, Color::LightRed, Color::Green),
    ];

    for (name, accent, success) in mappings {
        let theme = TuiTheme::for_name(name, truecolor);
        assert_eq!(theme.accent, accent, "{name:?} accent");
        assert_eq!(theme.success, success, "{name:?} success");
        assert_ne!(theme.success, theme.danger, "{name:?} status symbols need distinct styles");
        if !matches!(name, TuiThemeName::DarkAnsi | TuiThemeName::LightAnsi) {
            assert!(
                contrast_ratio(theme.text, theme.background) >= 7.0,
                "{name:?} body text contrast"
            );
            for status in [theme.success, theme.warning, theme.danger] {
                assert!(
                    // 状态色同时由固定中文标签/符号与粗体区分；参考色弱浅色 warning
                    // 的颜色对比约为 2.1，因此这里审查不把颜色当作唯一信号。
                    contrast_ratio(status, theme.background) >= 2.0,
                    "{name:?} status contrast"
                );
            }
        }
    }

    let themed_snapshot = render_tui_snapshot_with_theme(
        40,
        12,
        &TuiState::default(),
        "VW",
        "gateway://theme",
        0,
        TuiThemeName::Light,
        TerminalCapabilities { background: TuiBackground::Light, ..truecolor },
    )
    .expect("render themed snapshot");
    assert_eq!((themed_snapshot.width, themed_snapshot.height), (40, 12));

    let no_color = TerminalCapabilities::from_environment_values(
        true,
        Some("truecolor"),
        Some("xterm-256color"),
        None,
        TuiBackground::Dark,
    );
    let no_color_theme = TuiTheme::from_capabilities(no_color, false);
    assert_eq!(no_color.color_level, TuiColorLevel::None);
    assert!(no_color_theme.colors().iter().all(|color| *color == Color::Reset));

    let apple_terminal = TerminalCapabilities::from_environment_values(
        false,
        Some("truecolor"),
        Some("xterm-256color"),
        Some("Apple_Terminal"),
        TuiBackground::Light,
    );
    assert_eq!(apple_terminal.color_level, TuiColorLevel::Ansi256);
    assert!(apple_terminal.apple_terminal);
    assert!(
        TuiTheme::from_capabilities(apple_terminal, false)
            .colors()
            .iter()
            .all(|color| matches!(color, Color::Indexed(_)))
    );

    let ansi = TerminalCapabilities::from_environment_values(
        false,
        None,
        Some("xterm"),
        None,
        TuiBackground::Dark,
    );
    assert_eq!(ansi.color_level, TuiColorLevel::Ansi);
    assert_eq!(
        TuiTheme::from_capabilities(ansi, false).accent,
        TuiTheme::for_name(TuiThemeName::DarkAnsi, ansi).accent
    );

    let unknown_background = TerminalCapabilities {
        color_level: TuiColorLevel::TrueColor,
        background: TuiBackground::Unknown,
        apple_terminal: false,
        osc8_hyperlinks: false,
    };
    let unknown_theme = TuiTheme::from_capabilities(unknown_background, false);
    assert_eq!(unknown_theme.background, Color::Reset);
    assert_eq!(unknown_theme.surface, Color::Reset);
    assert_eq!(unknown_theme.surface_alt, Color::Reset);
}

#[test]
fn tui_panel_styles_leave_terminal_background_untouched() {
    let theme = TuiTheme::default();

    assert_eq!(theme.panel_style(TuiPanelSurface::Base).bg, None);
    assert_eq!(theme.panel_style(TuiPanelSurface::Raised).bg, None);
}

fn contrast_ratio(foreground: Color, background: Color) -> f64 {
    let lighter = relative_luminance(foreground).max(relative_luminance(background));
    let darker = relative_luminance(foreground).min(relative_luminance(background));
    (lighter + 0.05) / (darker + 0.05)
}

fn relative_luminance(color: Color) -> f64 {
    let Color::Rgb(red, green, blue) = color else {
        panic!("contrast review requires RGB colors, got {color:?}");
    };
    let linear = |channel: u8| {
        let channel = f64::from(channel) / 255.0;
        if channel <= 0.04045 { channel / 12.92 } else { ((channel + 0.055) / 1.055).powf(2.4) }
    };
    0.2126 * linear(red) + 0.7152 * linear(green) + 0.0722 * linear(blue)
}
