//! Lock observation is separate from authorization and execution capability.
//! No authentication changes, unlock operations or login-window input live here.

use vw_api_computer_control::{
    ComputerControlAction, ComputerControlTarget, ControlErrorCode, MouseButton, NativeInput,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LockState {
    Unlocked,
    Locked,
    Unknown,
}

/// Whether this executable includes the restricted native AX backend.
pub const fn supports_locked_execution() -> bool {
    cfg!(target_os = "macos")
}

pub(crate) fn require_execution(
    state: LockState,
    target: &ComputerControlTarget,
    action: &ComputerControlAction,
) -> Result<(), ControlErrorCode> {
    let native = supports_locked_execution() && matches!((target, action),
        (ComputerControlTarget::NativeApp { .. }, ComputerControlAction::NativeObserve { screenshot: false })
        | (ComputerControlTarget::NativeApp { .. }, ComputerControlAction::NativeInput {
            input: NativeInput::TypeText { .. } | NativeInput::Click { button: MouseButton::Left, .. }
        })
    );
    let ComputerControlTarget::NativeApp { bundle_id, .. } = target;
    check_native_target(bundle_id)?;
    if state == LockState::Unlocked || native {
        Ok(())
    } else {
        Err(ControlErrorCode::Unsupported)
    }
}

/// Authentication surfaces are never application automation targets.
pub(super) fn check_native_target(bundle_id: &str) -> Result<(), ControlErrorCode> {
    let id = bundle_id.to_ascii_lowercase();
    if matches!(id.as_str(), "com.apple.loginwindow" | "com.apple.securityagent"
        | "com.apple.screensaver.engine" | "com.apple.authorizationhost") {
        Err(ControlErrorCode::PermissionDenied)
    } else { Ok(()) }
}

#[cfg(any(target_os = "macos", test))]
fn decode_macos_lock(value: Result<Option<bool>, ()>, active_console: bool) -> LockState {
    match value {
        Ok(Some(true)) => LockState::Locked,
        Ok(Some(false)) => LockState::Unlocked,
        // WindowServer can omit the lock key on a logged-in, unlocked console.
        Ok(None) if active_console => LockState::Unlocked,
        Ok(None) | Err(()) => LockState::Unknown,
    }
}

pub fn lock_state() -> LockState {
    #[cfg(target_os = "macos")]
    {
        macos_lock_state()
    }
    #[cfg(not(target_os = "macos"))]
    {
        LockState::Unknown
    }
}

#[cfg(target_os = "macos")]
fn macos_lock_state() -> LockState {
    use std::ffi::c_void;
    type Object = *const c_void;
    #[link(name = "CoreGraphics", kind = "framework")]
    unsafe extern "C" {
        fn CGSessionCopyCurrentDictionary() -> Object;
    }
    #[link(name = "CoreFoundation", kind = "framework")]
    unsafe extern "C" {
        fn CFRelease(value: Object);
        fn CFStringCreateWithCString(allocator: Object, bytes: *const std::ffi::c_char, encoding: u32) -> Object;
        fn CFDictionaryGetValue(dictionary: Object, key: Object) -> Object;
        fn CFGetTypeID(value: Object) -> usize;
        fn CFBooleanGetTypeID() -> usize;
        fn CFBooleanGetValue(value: Object) -> bool;
    }
    // SAFETY: Create/Copy references are released exactly once. Borrowed values
    // remain within the dictionary lifetime; type-check before reading a boolean.
    unsafe {
        let dictionary = CGSessionCopyCurrentDictionary();
        if dictionary.is_null() {
            return LockState::Unknown;
        }
        let read_bool = |name: &std::ffi::CStr| {
            let key = CFStringCreateWithCString(std::ptr::null(), name.as_ptr(), 0x08000100);
            if key.is_null() {
                return Err(());
            }
            let value = CFDictionaryGetValue(dictionary, key);
            CFRelease(key);
            if value.is_null() {
                Ok(None)
            } else if CFGetTypeID(value) == CFBooleanGetTypeID() {
                Ok(Some(CFBooleanGetValue(value)))
            } else {
                Err(())
            }
        };
        let locked = read_bool(c"CGSSessionScreenIsLocked");
        let active_console = locked == Ok(None)
            && read_bool(c"kCGSSessionOnConsoleKey") == Ok(Some(true))
            && read_bool(c"kCGSessionLoginDoneKey") == Ok(Some(true));
        CFRelease(dictionary);
        decode_macos_lock(locked, active_console)
    }
}

#[cfg(test)]
#[path = "locked_session_tests.rs"]
mod tests;
