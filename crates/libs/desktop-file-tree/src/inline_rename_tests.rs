use super::*;
use iced::Theme;
#[test]
fn selection_preserves_extensions_and_counts_graphemes() {
    for (name, directory, end) in [
        ("ACP验收.md", false, 5),
        ("archive.tar.gz", false, 11),
        (".gitignore", false, 10),
        ("dev.docs", true, 8),
        ("e\u{301}📚.md", false, 2),
    ] {
        assert_eq!(selection_end(name, directory), end, "{name}");
    }
}

#[test]
fn inline_input_has_no_border_or_background_in_all_states() {
    for theme in [Theme::Light, Theme::Dark] {
        for status in [
            text_input::Status::Active,
            text_input::Status::Hovered,
            text_input::Status::Focused { is_hovered: false },
            text_input::Status::Disabled,
        ] {
            let style = input_style(&theme, status);
            assert_eq!(style.border.width, 0.0);
            assert_eq!(style.background, Background::Color(Color::TRANSPARENT));
        }
    }
}
