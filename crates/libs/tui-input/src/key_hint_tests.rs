use super::*;

#[test]
fn is_press_accepts_press_and_repeat_but_rejects_release() {
    let binding = ctrl(KeyCode::Char('k'));
    let press = KeyEvent::new(KeyCode::Char('k'), KeyModifiers::CONTROL);
    let repeat = KeyEvent { kind: KeyEventKind::Repeat, ..press };
    let release = KeyEvent { kind: KeyEventKind::Release, ..press };
    let wrong_modifiers = KeyEvent::new(KeyCode::Char('k'), KeyModifiers::NONE);

    assert!(binding.is_press(press));
    assert!(binding.is_press(repeat));
    assert!(!binding.is_press(release));
    assert!(!binding.is_press(wrong_modifiers));
}

#[test]
fn keybinding_list_ext_matches_any_binding() {
    let bindings = [plain(KeyCode::Char('a')), ctrl(KeyCode::Char('b'))];

    assert!(bindings.is_pressed(KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE)));
    assert!(bindings.is_pressed(KeyEvent::new(KeyCode::Char('b'), KeyModifiers::CONTROL)));
    assert!(!bindings.is_pressed(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::NONE)));
}

#[test]
fn shifted_letter_binding_matches_uppercase_char_events() {
    let binding = shift(KeyCode::Char('a'));

    assert!(binding.is_press(KeyEvent::new(KeyCode::Char('a'), KeyModifiers::SHIFT)));
    assert!(binding.is_press(KeyEvent::new(KeyCode::Char('A'), KeyModifiers::NONE)));
    assert!(binding.is_press(KeyEvent::new(KeyCode::Char('A'), KeyModifiers::SHIFT)));
}

#[test]
fn shift_letter_binding_preserves_other_modifiers_with_uppercase_compat() {
    let binding =
        KeyBinding::new(KeyCode::Char('i'), KeyModifiers::CONTROL | KeyModifiers::SHIFT);

    assert!(binding.is_press(KeyEvent::new(KeyCode::Char('I'), KeyModifiers::CONTROL)));
}

#[test]
fn shift_letter_binding_does_not_match_plain_lowercase_or_other_uppercase() {
    let binding = shift(KeyCode::Char('o'));

    assert!(!binding.is_press(KeyEvent::new(KeyCode::Char('o'), KeyModifiers::NONE)));
    assert!(!binding.is_press(KeyEvent::new(KeyCode::Char('P'), KeyModifiers::NONE)));
}

#[test]
fn ctrl_letter_binding_matches_c0_control_char_events() {
    let binding = ctrl(KeyCode::Char('p'));

    assert!(binding.is_press(KeyEvent::new(KeyCode::Char('\u{0010}'), KeyModifiers::NONE)));
    assert!(!binding.is_press(KeyEvent::new(KeyCode::Char('\u{0010}'), KeyModifiers::ALT)));
}

#[test]
fn ctrl_bindings_match_all_supported_c0_control_char_events() {
    let cases = [
        (' ', '\u{0000}'),
        ('a', '\u{0001}'),
        ('b', '\u{0002}'),
        ('c', '\u{0003}'),
        ('d', '\u{0004}'),
        ('e', '\u{0005}'),
        ('f', '\u{0006}'),
        ('g', '\u{0007}'),
        ('h', '\u{0008}'),
        ('i', '\u{0009}'),
        ('j', '\u{000a}'),
        ('k', '\u{000b}'),
        ('l', '\u{000c}'),
        ('m', '\u{000d}'),
        ('n', '\u{000e}'),
        ('o', '\u{000f}'),
        ('p', '\u{0010}'),
        ('q', '\u{0011}'),
        ('r', '\u{0012}'),
        ('s', '\u{0013}'),
        ('t', '\u{0014}'),
        ('u', '\u{0015}'),
        ('v', '\u{0016}'),
        ('w', '\u{0017}'),
        ('x', '\u{0018}'),
        ('y', '\u{0019}'),
        ('z', '\u{001a}'),
        ('4', '\u{001c}'),
        ('5', '\u{001d}'),
        ('6', '\u{001e}'),
        ('7', '\u{001f}'),
    ];

    for (ctrl_char, c0_char) in cases {
        assert!(
            ctrl(KeyCode::Char(ctrl_char))
                .is_press(KeyEvent::new(KeyCode::Char(c0_char), KeyModifiers::NONE)),
            "expected raw C0 {c0_char:?} to match ctrl-{ctrl_char}"
        );
        assert!(
            !ctrl(KeyCode::Char(ctrl_char))
                .is_press(KeyEvent::new(KeyCode::Char(c0_char), KeyModifiers::ALT)),
            "expected modified raw C0 {c0_char:?} not to match ctrl-{ctrl_char}"
        );
    }
}

#[test]
fn ctrl_binding_does_not_match_ambiguous_c0_escape_or_delete() {
    assert!(
        !ctrl(KeyCode::Char('['))
            .is_press(KeyEvent::new(KeyCode::Char('\u{001b}'), KeyModifiers::NONE,))
    );
    assert!(
        !ctrl(KeyCode::Char('?'))
            .is_press(KeyEvent::new(KeyCode::Char('\u{007f}'), KeyModifiers::NONE,))
    );
}

#[test]
fn history_search_ctrl_bindings_match_c0_control_char_events() {
    assert!(
        ctrl(KeyCode::Char('r'))
            .is_press(KeyEvent::new(KeyCode::Char('\u{0012}'), KeyModifiers::NONE))
    );
    assert!(
        ctrl(KeyCode::Char('s'))
            .is_press(KeyEvent::new(KeyCode::Char('\u{0013}'), KeyModifiers::NONE))
    );
}

#[test]
fn ctrl_alt_sets_both_modifiers() {
    assert_eq!(
        ctrl_alt(KeyCode::Char('v')).parts(),
        (KeyCode::Char('v'), KeyModifiers::CONTROL | KeyModifiers::ALT)
    );
}

#[test]
fn has_ctrl_or_alt_checks_supported_modifier_combinations() {
    assert!(!has_ctrl_or_alt(KeyModifiers::NONE));
    assert!(has_ctrl_or_alt(KeyModifiers::CONTROL));
    assert!(has_ctrl_or_alt(KeyModifiers::ALT));

    #[cfg(windows)]
    assert!(!has_ctrl_or_alt(KeyModifiers::CONTROL | KeyModifiers::ALT));
    #[cfg(not(windows))]
    assert!(has_ctrl_or_alt(KeyModifiers::CONTROL | KeyModifiers::ALT));
}
