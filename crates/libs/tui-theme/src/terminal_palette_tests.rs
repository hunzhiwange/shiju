use super::*;
use pretty_assertions::assert_eq;

#[test]
fn best_color_uses_truecolor_without_quantization() {
    assert_eq!(
        best_color_for_color_level((12, 34, 56), StdoutColorLevel::TrueColor),
        rgb_color((12, 34, 56))
    );
}

#[test]
fn best_color_resets_for_ansi16() {
    assert_eq!(
        best_color_for_color_level((12, 34, 56), StdoutColorLevel::Ansi16),
        Color::Reset
    );
}

#[test]
fn windows_terminal_wt_session_promotes_to_truecolor() {
    assert_eq!(
        stdout_color_level_for_terminal(
            StdoutColorLevel::Ansi16,
            TerminalName::Unknown,
            /*has_wt_session*/ true,
            /*has_force_color_override*/ false,
        ),
        StdoutColorLevel::TrueColor
    );
}

#[test]
fn windows_terminal_name_promotes_ansi16_to_truecolor() {
    assert_eq!(
        stdout_color_level_for_terminal(
            StdoutColorLevel::Ansi16,
            TerminalName::WindowsTerminal,
            /*has_wt_session*/ false,
            /*has_force_color_override*/ false,
        ),
        StdoutColorLevel::TrueColor
    );
}

#[test]
fn force_color_keeps_reported_stdout_level() {
    assert_eq!(
        stdout_color_level_for_terminal(
            StdoutColorLevel::Ansi16,
            TerminalName::WindowsTerminal,
            /*has_wt_session*/ true,
            /*has_force_color_override*/ true,
        ),
        StdoutColorLevel::Ansi16
    );
}
