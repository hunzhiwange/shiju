use super::*;
fn target() -> Target {
    Target { pid: 42, window: 17, label: "Fixture · 单窗口".into() }
}
#[test]
fn voice_task_17_screen_context_disabled_captures_nothing() {
    let mut state = State::default();
    let mut captured = 0;
    if let Some(ticket) = state.begin(target(), true) {
        let _ = ticket.capture(|_| {
            captured += 1;
            Ok(())
        });
    }
    assert_eq!(captured, 0, "disabled sharing must never enter the OS capture boundary");
}
#[test]
fn voice_task_17_screen_context_rejects_late_frame() {
    let mut state = State::default();
    state.set_enabled(true);
    let ticket = state.begin(target(), true).unwrap();
    let frame = ticket.capture(|_| Ok(vec![1, 2, 3])).unwrap();
    state.cancel();
    assert!(
        state.accept(&ticket, frame, true).is_none(),
        "cancelled/session-retired frame must never be sent"
    );
}

#[test]
fn voice_task_17_screen_denied_or_cancelled_enters_no_capture_boundary() {
    let mut state = State::default();
    state.set_enabled(true);
    let mut captures = 0;
    assert!(state.begin(target(), false).is_none());
    let ticket = state.begin(target(), true).unwrap();
    state.cancel();
    assert!(
        ticket
            .capture(|_| {
                captures += 1;
                Ok(())
            })
            .is_err()
    );
    assert_eq!(captures, 0);
}
#[test]
fn voice_task_17_screen_only_one_frame_for_current_ticket() {
    let mut state = State::default();
    state.set_enabled(true);
    let old = state.begin(target(), true).unwrap();
    let current = state.begin(target(), true).unwrap();
    assert!(state.accept(&old, vec![0], true).is_none());
    assert!(state.accept(&current, vec![1], false).is_none());
    assert_eq!(state.accept(&current, vec![1], true), Some(vec![1]));
    assert!(state.accept(&current, vec![1], true).is_none());
    state.set_enabled(false);
    assert!(!current.is_live());
}
#[test]
fn voice_task_17_screen_owner_drop_retires_inflight_worker() {
    let ticket = {
        let mut state = State::default();
        state.set_enabled(true);
        state.begin(target(), true).unwrap()
    };
    assert!(!ticket.is_live());
}
