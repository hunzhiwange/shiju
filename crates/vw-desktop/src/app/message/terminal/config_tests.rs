#[test]
fn test_module_is_wired() {
    let module = std::hint::black_box(module_path!());

    assert!(module.ends_with("config_tests"));
}

fn terminal() -> crate::app::TerminalState {
    crate::app::TerminalState::default()
}

#[test]
fn shell_selection_updates_state_and_returns_stable_config_key() {
    for (shell, expected_key) in
        [(crate::app::Shell::Bash, "bash"), (crate::app::Shell::Zsh, "zsh")]
    {
        let mut terminal = terminal();

        let update = super::apply_update(
            &mut terminal,
            &iced::Theme::Dark,
            super::TerminalMessage::ShellSelected(shell),
        );

        assert_eq!(terminal.shell, shell);
        assert!(matches!(
            update,
            Some(super::TerminalConfigUpdate::Shell(key)) if key == expected_key
        ));
    }
}

#[test]
fn system_theme_selection_updates_state_and_returns_config_key() {
    let mut terminal = terminal();

    let update = super::apply_update(
        &mut terminal,
        &iced::Theme::Dark,
        super::TerminalMessage::ThemeSelected(crate::app::TerminalTheme::System),
    );

    assert_eq!(terminal.theme, crate::app::TerminalTheme::System);
    assert!(matches!(update, Some(super::TerminalConfigUpdate::Theme("system"))));
}

#[test]
fn font_selection_preserves_exact_family_for_state_and_persistence() {
    let mut terminal = terminal();

    let update = super::apply_update(
        &mut terminal,
        &iced::Theme::Dark,
        super::TerminalMessage::FontSelected("  Fira Code  ".to_string()),
    );

    assert_eq!(terminal.font_family, "  Fira Code  ");
    assert!(matches!(
        update,
        Some(super::TerminalConfigUpdate::FontFamily(family)) if family == "  Fira Code  "
    ));
}

#[test]
fn font_size_selection_preserves_boundary_values() {
    for size in [0.0, 18.5, f32::INFINITY] {
        let mut terminal = terminal();

        let update = super::apply_update(
            &mut terminal,
            &iced::Theme::Dark,
            super::TerminalMessage::FontSizeChanged(size),
        );

        assert_eq!(terminal.font_size, size);
        assert!(matches!(
            update,
            Some(super::TerminalConfigUpdate::FontSize(value)) if value == size
        ));
    }
}

#[test]
fn unrelated_message_leaves_terminal_unchanged_and_skips_persistence() {
    let mut terminal = terminal();
    let original_shell = terminal.shell;
    let original_theme = terminal.theme;
    let original_family = terminal.font_family.clone();
    let original_size = terminal.font_size;

    let update =
        super::apply_update(&mut terminal, &iced::Theme::Dark, super::TerminalMessage::DragStarted);

    assert!(update.is_none());
    assert_eq!(terminal.shell, original_shell);
    assert_eq!(terminal.theme, original_theme);
    assert_eq!(terminal.font_family, original_family);
    assert_eq!(terminal.font_size, original_size);
}

#[test]
fn update_with_unrelated_message_leaves_app_terminal_unchanged() {
    let (mut app, _task) = crate::app::App::new();
    let original_shell = app.terminal.shell;
    let original_theme = app.terminal.theme;
    let original_family = app.terminal.font_family.clone();
    let original_size = app.terminal.font_size;

    let _task = super::update(&mut app, super::TerminalMessage::DragStarted);

    assert_eq!(app.terminal.shell, original_shell);
    assert_eq!(app.terminal.theme, original_theme);
    assert_eq!(app.terminal.font_family, original_family);
    assert_eq!(app.terminal.font_size, original_size);
}
