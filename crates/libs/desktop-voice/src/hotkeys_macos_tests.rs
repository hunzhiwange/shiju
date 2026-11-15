//! Real CoreGraphics event objects, without posting any keys into the user's session.
use super::*;
unsafe extern "C" {
    fn CGEventCreateKeyboardEvent(source: Ref, key: u16, down: bool) -> Ref;
    fn CGEventSetFlags(event: Ref, flags: u64);
    fn CGEventSetType(event: Ref, kind: u32);
    fn CGEventSetIntegerValueField(event: Ref, field: u32, value: i64);
}
fn context() -> (Context, Receiver<(Input, Instant)>) {
    let (tx, rx) = mpsc::sync_channel(8);
    (
        Context {
            config: Config {
                enabled: true,
                dictation: Some(Shortcut { key: 0, modifiers: CONTROL | ALT }),
                chat: Some(Shortcut { key: 8, modifiers: CONTROL | ALT }),
            },
            tx,
            fault: Arc::new(AtomicBool::new(false)),
            dictation_down: false,
            chat_down: false,
        },
        rx,
    )
}
fn emit(context: &mut Context, kind: u32, key: u16, flags: u32, repeat: bool) {
    // SAFETY: create/set/release owned event objects. callback sees a live local Context;
    // no CGEventPost or system authorization call occurs.
    unsafe {
        let event = CGEventCreateKeyboardEvent(ptr::null(), key, kind == 10);
        assert!(!event.is_null());
        CGEventSetFlags(event, u64::from(flags));
        CGEventSetType(event, kind);
        CGEventSetIntegerValueField(event, 8, i64::from(repeat));
        assert_eq!(callback(ptr::null(), kind, event, (context as *mut Context).cast()), event);
        CFRelease(event);
    }
}
#[test]
fn voice_task_15_native_modifier_release_completes_without_disabling_listener() {
    let (mut context, rx) = context();
    emit(&mut context, 10, 0, CONTROL | ALT, false);
    assert!(matches!(rx.try_recv().unwrap().0, Input::Press(false)));
    emit(&mut context, 12, 59, ALT, false);
    assert!(matches!(rx.try_recv().map(|e| e.0), Ok(Input::Release)));
    assert!(!context.fault.load(Ordering::SeqCst));
    emit(&mut context, 11, 0, ALT, false);
    assert!(rx.try_recv().is_err());
}
#[test]
fn voice_task_15_native_filters_unmatched_keys_and_preserves_repeat() {
    let (mut context, rx) = context();
    emit(&mut context, 10, 1, CONTROL | ALT, false);
    emit(&mut context, 10, 0, 0, false);
    assert!(rx.try_recv().is_err());
    emit(&mut context, 10, 0, CONTROL | ALT, false);
    emit(&mut context, 10, 0, CONTROL | ALT, true);
    emit(&mut context, 11, 0, CONTROL | ALT, false);
    assert!(matches!(rx.try_recv().unwrap().0, Input::Press(false)));
    assert!(matches!(rx.try_recv().unwrap().0, Input::Press(true)));
    assert!(matches!(rx.try_recv().unwrap().0, Input::Release));
    assert!(!context.fault.load(Ordering::SeqCst));
}
#[test]
fn voice_task_15_native_overflow_and_disabled_tap_fail_closed() {
    let (mut context, _rx) = context();
    for _ in 0..9 {
        emit(&mut context, 10, 0, CONTROL | ALT, false);
    }
    assert!(context.fault.load(Ordering::SeqCst));
    let (mut context, _rx) = self::context();
    unsafe {
        callback(ptr::null(), u32::MAX, ptr::null(), (&mut context as *mut Context).cast());
    }
    assert!(context.fault.load(Ordering::SeqCst));
}

#[test]
fn voice_task_15_native_double_tap_survives_timer_and_finishes_once() {
    use super::super::hotkeys::{Action, Machine};
    let (mut context, rx) = context();
    let now = Instant::now();
    let mut machine = Machine::default();
    let mut actions = Vec::new();
    // Include a repeated down and modifier-first release in each tap. The later
    // letter-up must not create an extra release or reset hands-free mode.
    for (ms, kind, key, flags, repeat) in [
        (0, 10, 0, CONTROL | ALT, false),
        (10, 10, 0, CONTROL | ALT, true),
        (40, 12, 59, ALT, false),
        (50, 11, 0, ALT, false),
        (100, 10, 0, CONTROL | ALT, false),
        (110, 10, 0, CONTROL | ALT, true),
        (140, 12, 59, ALT, false),
        (150, 11, 0, ALT, false),
    ] {
        emit(&mut context, kind, key, flags, repeat);
        for (input, _) in rx.try_iter() {
            actions.extend(machine.input(input, now + Duration::from_millis(ms)));
        }
    }
    assert_eq!(actions, vec![Action::Start]);
    assert_eq!(machine.tick(now + Duration::from_secs(1)), None);
    assert!(machine.active);
    emit(&mut context, 10, 0, CONTROL | ALT, false);
    let (input, _) = rx.try_recv().unwrap();
    assert_eq!(machine.input(input, now + Duration::from_secs(2)), Some(Action::Finish));
    emit(&mut context, 11, 0, 0, false);
    let (input, _) = rx.try_recv().unwrap();
    assert_eq!(machine.input(input, now + Duration::from_secs(2)), None);
    assert_eq!(machine.tick(now + Duration::from_secs(3)), None);
    assert!(!machine.active);
    assert!(!context.fault.load(Ordering::SeqCst));
}
