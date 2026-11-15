use super::*;
fn target() -> screen_context::Target {
    screen_context::Target { pid: 42, window: 7, label: "中性窗口".into() }
}
#[test]
fn voice_task_17_old_completion_cannot_cancel_new_share() {
    let mut screen = screen_context::State::default();
    screen.set_enabled(true);
    let old = screen.begin(target(), true).unwrap();
    let current = screen.begin(target(), true).unwrap();
    let _ =
        update(&mut screen, ScreenEvent::Ready(old, Owner::Cloud(VoiceCounter::FIRST), Ok(vec![1])), false, false, None);
    assert!(current.is_live());
    assert!(screen.owns(&current));
}
