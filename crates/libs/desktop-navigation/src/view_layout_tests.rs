#[test]
fn global_command_shortcuts_are_case_insensitive_and_require_command_modifier() {
    use super::GlobalCommand;
    use iced::keyboard::{Key, Modifiers};

    assert!(matches!(
        super::global_command(&Key::Character("P".into()), Modifiers::COMMAND),
        Some(GlobalCommand::Search)
    ));
    assert!(matches!(
        super::global_command(&Key::Character("c".into()), Modifiers::COMMAND),
        Some(GlobalCommand::Copy)
    ));
    assert!(super::global_command(&Key::Character("p".into()), Modifiers::NONE).is_none());
    assert!(
        super::global_command(&Key::Character("x".into()), Modifiers::COMMAND).is_none()
    );
}
