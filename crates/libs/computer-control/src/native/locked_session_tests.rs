use super::*;

#[test]
fn cc_control_17_unknown_state_keeps_native_input_closed() {
    let target = ComputerControlTarget::NativeApp { bundle_id: "com.example.App".into(), process_id: 1 };
    let action = ComputerControlAction::NativeInput { input: NativeInput::Activate };
    assert_eq!(require_execution(LockState::Unknown, &target, &action), Err(ControlErrorCode::Unsupported));
}

#[test]
fn macos_missing_lock_key_requires_a_logged_in_console() {
    assert_eq!(decode_macos_lock(Ok(None), true), LockState::Unlocked);
    assert_eq!(decode_macos_lock(Ok(None), false), LockState::Unknown);
    assert_eq!(decode_macos_lock(Err(()), true), LockState::Unknown);
    assert_eq!(decode_macos_lock(Ok(Some(true)), true), LockState::Locked);
    assert_eq!(decode_macos_lock(Ok(Some(false)), false), LockState::Unlocked);
}
