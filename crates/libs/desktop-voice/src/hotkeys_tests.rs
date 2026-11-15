use super::*;
use std::time::{Duration, Instant};
#[test]
fn voice_task_15_hotkey_repeat_does_not_restart_capture() {
    let now = Instant::now();
    let mut state = Machine::default();
    assert_eq!(state.press(now, false), Some(Action::Start));
    assert_eq!(state.press(now + Duration::from_millis(50), true), None);
}
#[test]
fn voice_task_15_hotkey_double_tap_enters_hands_free() {
    let now = Instant::now();
    let mut state = Machine::default();
    assert_eq!(state.press(now, false), Some(Action::Start));
    assert_eq!(state.release(now + Duration::from_millis(60)), None);
    assert_eq!(state.press(now + Duration::from_millis(160), false), None);
    assert_eq!(state.release(now + Duration::from_millis(200)), None);
    assert!(state.active);
    assert_eq!(state.tick(now + Duration::from_millis(600)), None);
    assert!(state.active);
    assert_eq!(state.press(now + Duration::from_secs(1), false), Some(Action::Finish));
}
#[test]
fn voice_task_15_hold_release_finishes_and_late_release_is_ignored() {
    let now = Instant::now();
    let mut state = Machine::default();
    assert_eq!(state.press(now, false), Some(Action::Start));
    assert_eq!(state.release(now + Duration::from_secs(1)), Some(Action::Finish));
    assert_eq!(state.release(now + Duration::from_secs(2)), None);
    assert!(!state.active);
}
#[test]
fn voice_task_15_single_tap_waits_for_double_tap_then_finishes_once() {
    let now = Instant::now();
    let mut state = Machine::default();
    state.press(now, false);
    state.release(now + Duration::from_millis(50));
    assert_eq!(state.tick(now + Duration::from_millis(200)), None);
    assert_eq!(state.tick(now + Duration::from_millis(331)), Some(Action::Finish));
    assert_eq!(state.tick(now + Duration::from_secs(2)), None);
}
#[test]
fn voice_task_15_late_second_press_cannot_enter_hands_free() {
    let now = Instant::now();
    let mut state = Machine::default();
    state.press(now, false);
    state.release(now + Duration::from_millis(50));
    assert_eq!(state.press(now + Duration::from_secs(1), false), Some(Action::Finish));
    assert!(!state.active);
}
#[test]
fn voice_task_15_escape_and_safety_loss_cancel_without_transcription() {
    let now = Instant::now();
    for input in [Input::Escape, Input::Unsafe] {
        let mut state = Machine::default();
        state.press(now, false);
        assert_eq!(state.input(input, now), Some(Action::Cancel));
        assert_eq!(state.release(now), None);
        assert_eq!(state.tick(now + Duration::from_secs(1)), None);
    }
}
#[test]
fn voice_task_15_chat_shortcut_ignores_repeat_and_active_dictation() {
    let now = Instant::now();
    let mut state = Machine::default();
    assert_eq!(state.input(Input::Chat(true), now), None);
    assert_eq!(state.input(Input::Chat(false), now), Some(Action::Chat));
    state.press(now, false);
    assert_eq!(state.input(Input::Chat(false), now), None);
}
#[test]
fn voice_task_15_fn_plain_duplicate_and_reserved_chords_are_rejected() {
    assert!(Shortcut { key: 63, modifiers: 0 }.validate().is_err());
    assert!(Shortcut { key: 0, modifiers: SHIFT }.validate().is_err());
    assert!(Shortcut { key: 49, modifiers: COMMAND | SHIFT }.validate().is_err());
    assert!(Shortcut { key: 0, modifiers: CONTROL | ALT }.validate().is_ok());
    let shortcut = Some(Shortcut { key: 0, modifiers: CONTROL | ALT });
    assert!(Config { enabled: false, dictation: shortcut, chat: shortcut }.validate().is_err());
}
#[test]
fn voice_task_15_disable_preserves_chords_and_unregisters_machine_ownership() {
    let now = Instant::now();
    let mut runtime = Runtime::default();
    runtime.machine.press(now, false);
    runtime.stop_session();
    runtime.reload();
    assert!(runtime.blocked);
    assert!(runtime.poll(|| Ok(Config::default())).is_empty());
    assert!(!runtime.machine.active);
    assert_eq!(runtime.machine.release(now), None);
    runtime.resume_after_save();
    assert!(!runtime.blocked);
}
