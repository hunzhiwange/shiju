//! Lock observation is separate from authorization and execution capability.
//! No authentication changes, unlock operations or login-window input live here.

use vw_api_types::computer_control::{
    ComputerControlAction, ComputerControlTarget, ChromeCommand, ControlErrorCode,
    WordCommand, WordInsertPosition, NativeInput, MouseButton,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LockState {
    Unlocked,
    Locked,
    Unknown,
}

/// These commands execute inside an already paired application runtime, without
/// screen capture, system input, navigation, new pairing or an unlock operation.
/// This is dispatch capability, not evidence of successful execution while locked.
pub(crate) fn structured_execution(
    target: &ComputerControlTarget,
    action: &ComputerControlAction,
) -> bool {
    matches!((target, action),
        (ComputerControlTarget::ChromeTab { .. }, ComputerControlAction::Chrome {
            command: ChromeCommand::Snapshot | ChromeCommand::Fill { .. }
        })
        | (ComputerControlTarget::ExcelWorkbook { .. }, ComputerControlAction::Excel { .. })
        | (ComputerControlTarget::WordDocument { .. }, ComputerControlAction::Word {
            command: WordCommand::ReadDocument | WordCommand::FindText { .. }
                | WordCommand::ReplaceText { .. }
                | WordCommand::InsertText {
                    position: WordInsertPosition::Start | WordInsertPosition::End, ..
                }
        })
    )
}

pub(crate) fn require_execution(
    state: LockState,
    target: &ComputerControlTarget,
    action: &ComputerControlAction,
) -> Result<(), ControlErrorCode> {
    let native = cfg!(target_os = "macos") && matches!((target, action),
        (ComputerControlTarget::NativeApp { .. }, ComputerControlAction::NativeObserve { screenshot: false })
        | (ComputerControlTarget::NativeApp { .. }, ComputerControlAction::NativeInput {
            input: NativeInput::TypeText { .. } | NativeInput::Click { button: MouseButton::Left, .. }
        })
    );
    if let ComputerControlTarget::NativeApp { bundle_id, .. } = target {
        check_native_target(bundle_id)?;
    }
    if state == LockState::Unlocked || structured_execution(target, action) || native {
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

/// Missing or malformed evidence is unknown, never an unlocked session.
fn decode_lock(value: Option<bool>) -> LockState {
    match value {
        Some(true) => LockState::Locked,
        Some(false) => LockState::Unlocked,
        None => LockState::Unknown,
    }
}

pub fn lock_state() -> LockState {
    #[cfg(target_os = "macos")]
    {
        macos_lock_state()
    }
    #[cfg(not(target_os = "macos"))]
    {
        decode_lock(None)
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
        fn CFStringCreateWithCString(allocator: Object, bytes: *const u8, encoding: u32) -> Object;
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
        let key = CFStringCreateWithCString(
            std::ptr::null(), b"CGSSessionScreenIsLocked\0".as_ptr(), 0x08000100,
        );
        if key.is_null() {
            CFRelease(dictionary);
            return LockState::Unknown;
        }
        let value = CFDictionaryGetValue(dictionary, key);
        let locked = if !value.is_null() && CFGetTypeID(value) == CFBooleanGetTypeID() {
            Some(CFBooleanGetValue(value))
        } else {
            None
        };
        CFRelease(key);
        CFRelease(dictionary);
        decode_lock(locked)
    }
}

#[cfg(test)]
#[path = "locked_session_tests.rs"]
mod tests;
