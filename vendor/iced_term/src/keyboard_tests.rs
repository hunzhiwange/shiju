use super::{BindingAction, BindingsLayout};
use alacritty_terminal::term::TermMode;
use iced_core::keyboard::{key::Named, Key, Modifiers};

fn bytes(
    key: Key,
    modifiers: Modifiers,
    text: Option<&str>,
    mode: TermMode,
) -> Vec<u8> {
    match BindingsLayout::new().key_action(&key, modifiers, text, mode) {
        BindingAction::Char(c) => c.to_string().into_bytes(),
        BindingAction::Esc(s) => s.into_bytes(),
        action => panic!("expected PTY input, got {action:?}"),
    }
}

#[test]
fn enter_submits_and_modified_enter_remains_distinguishable() {
    for key in [
        Key::Named(Named::Enter),
        Key::Character("\r".into()),
        Key::Character("\n".into()),
    ] {
        assert_eq!(
            bytes(
                key.clone(),
                Modifiers::empty(),
                Some("\n"),
                TermMode::empty()
            ),
            b"\r"
        );
        assert_eq!(
            bytes(key.clone(), Modifiers::SHIFT, None, TermMode::empty()),
            b"\x1b[13;2u"
        );
        assert_eq!(
            bytes(key, Modifiers::ALT, None, TermMode::empty()),
            b"\x1b\r"
        );
    }
    assert_eq!(
        bytes(
            Key::Character("j".into()),
            Modifiers::CTRL,
            None,
            TermMode::empty()
        ),
        b"\n"
    );
}

#[test]
fn cursor_keys_follow_application_mode_and_control_modifiers() {
    for (key, normal, application, control) in [
        (Named::ArrowUp, "\x1b[A", "\x1bOA", "\x1b[1;5A"),
        (Named::ArrowDown, "\x1b[B", "\x1bOB", "\x1b[1;5B"),
        (Named::ArrowRight, "\x1b[C", "\x1bOC", "\x1b[1;5C"),
        (Named::ArrowLeft, "\x1b[D", "\x1bOD", "\x1b[1;5D"),
    ] {
        assert_eq!(
            bytes(Key::Named(key), Modifiers::empty(), None, TermMode::empty()),
            normal.as_bytes()
        );
        assert_eq!(
            bytes(
                Key::Named(key),
                Modifiers::empty(),
                None,
                TermMode::APP_CURSOR
            ),
            application.as_bytes()
        );
        assert_eq!(
            bytes(Key::Named(key), Modifiers::CTRL, None, TermMode::APP_CURSOR),
            control.as_bytes()
        );
    }
}

#[test]
fn special_keys_emit_terminal_sequences() {
    for (key, modifiers, expected) in [
        (Named::Tab, Modifiers::SHIFT, "\x1b[Z"),
        (Named::Backspace, Modifiers::CTRL, "\x08"),
        (Named::Space, Modifiers::CTRL, "\0"),
        (Named::Insert, Modifiers::ALT, "\x1b[2;3~"),
        (Named::F1, Modifiers::CTRL, "\x1b[1;5P"),
        (Named::F2, Modifiers::CTRL, "\x1b[1;5Q"),
        (Named::F3, Modifiers::CTRL, "\x1b[1;5R"),
        (Named::F4, Modifiers::CTRL, "\x1b[1;5S"),
    ] {
        assert_eq!(
            bytes(Key::Named(key), modifiers, None, TermMode::empty()),
            expected.as_bytes()
        );
    }
    assert_eq!(
        bytes(
            Key::Character("\\".into()),
            Modifiers::CTRL,
            None,
            TermMode::empty()
        ),
        b"\x1c"
    );
}

#[test]
fn character_shortcuts_use_event_modifiers_without_text() {
    assert_eq!(
        bytes(
            Key::Character("C".into()),
            Modifiers::CTRL,
            None,
            TermMode::empty()
        ),
        b"\x03"
    );
    assert_eq!(
        bytes(
            Key::Character("b".into()),
            Modifiers::ALT,
            None,
            TermMode::empty()
        ),
        b"\x1bb"
    );
    assert_eq!(
        bytes(
            Key::Character("e".into()),
            Modifiers::ALT,
            Some("é"),
            TermMode::empty()
        ),
        "é".as_bytes()
    );
    assert_eq!(
        bytes(
            Key::Character("中".into()),
            Modifiers::empty(),
            Some("中文"),
            TermMode::empty()
        ),
        "中文".as_bytes()
    );
    assert_eq!(
        BindingsLayout::new().key_action(
            &Key::Character("q".into()),
            Modifiers::LOGO,
            Some("q"),
            TermMode::empty()
        ),
        BindingAction::Ignore
    );
}

#[cfg(unix)]
#[tokio::test]
async fn interactive_pty_receives_enter_and_navigation_bytes_unchanged() {
    use crate::backend::{Backend, Command};
    use crate::settings::BackendSettings;
    use std::time::Duration;

    let (sender, _receiver) = tokio::sync::mpsc::channel(100);
    let mut backend = Backend::new(
        1,
        sender,
        BackendSettings {
            program: "/bin/sh".into(),
            args: vec![
                "-c".into(),
                "stty raw -echo; printf READY; od -An -tx1 -N 13; sleep 1"
                    .into(),
            ],
            ..BackendSettings::default()
        },
    )
    .unwrap();
    let wait_for = |backend: &Backend, expected: &str| {
        backend
            .text()
            .join(" ")
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
            .contains(expected)
    };
    tokio::time::timeout(Duration::from_secs(5), async {
        while !wait_for(&backend, "READY") {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("raw PTY should become ready");

    for (key, modifiers, mode) in [
        (Named::Enter, Modifiers::empty(), TermMode::empty()),
        (Named::ArrowUp, Modifiers::empty(), TermMode::APP_CURSOR),
        (Named::Tab, Modifiers::SHIFT, TermMode::empty()),
        (Named::ArrowLeft, Modifiers::CTRL, TermMode::empty()),
    ] {
        backend.handle(Command::Write(bytes(
            Key::Named(key),
            modifiers,
            None,
            mode,
        )));
    }
    tokio::time::timeout(Duration::from_secs(5), async {
        // CR, application Up, BackTab and Ctrl+Left, captured by a raw-mode child.
        while !wait_for(&backend, "0d 1b 4f 41 1b 5b 5a 1b 5b 31 3b 35 44") {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap_or_else(|_| panic!("PTY output: {:?}", backend.text()));
}
