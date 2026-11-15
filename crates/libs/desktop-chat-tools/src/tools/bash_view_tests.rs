use super::*;
use iced::Padding;
#[test]
fn bash_rows_remove_left_indent() {
    assert_eq!(bash_view_padding("bash"), Padding { top: 2.0, right: 6.0, bottom: 2.0, left: 0.0 });
}

#[test]
fn bash_command_preview_is_single_line_and_truncates_overflow() {
    assert_eq!(
        bash_command_preview(
            desktop_i18n::I18n::new(config_ui::DesktopLocale::SimplifiedChinese),
            "cargo check\n  --all-targets"
        ),
        "cargo check --all-targets"
    );

    let preview = bash_command_preview(
        desktop_i18n::I18n::new(config_ui::DesktopLocale::SimplifiedChinese),
        &"x".repeat(100),
    );
    assert!(!preview.contains('\n'));
    assert!(preview.ends_with('…'));
    assert_eq!(preview.chars().count(), 73);
}
