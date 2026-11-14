use super::*;
use vw_api_types::computer_control::{ControlDocumentId, NativeInput};

#[test]
fn cc_control_17_unknown_state_keeps_native_input_closed() {
    let target = ComputerControlTarget::NativeApp { bundle_id: "com.example.App".into(), process_id: 1 };
    let action = ComputerControlAction::NativeInput { input: NativeInput::Activate };
    assert_eq!(decode_lock(None), LockState::Unknown);
    assert_eq!(require_execution(LockState::Unknown, &target, &action), Err(ControlErrorCode::Unsupported));
}

#[test]
fn cc_control_17_locked_chrome_fill_uses_document_executor() {
    let target = ComputerControlTarget::ChromeTab { tab_id: 1, origin: "https://example.com".into() };
    let action = ComputerControlAction::Chrome { command: ChromeCommand::Fill { selector: "#field".into(), text: "value".into() } };
    assert_eq!(require_execution(LockState::Locked, &target, &action), Ok(()));
}

#[test]
fn cc_control_17_locked_navigation_cannot_expand_target() {
    let target = ComputerControlTarget::ChromeTab { tab_id: 1, origin: "https://example.com".into() };
    let action = ComputerControlAction::Chrome { command: ChromeCommand::Navigate { url: "https://other.example".into() } };
    assert_eq!(require_execution(LockState::Locked, &target, &action), Err(ControlErrorCode::Unsupported));
}

#[test]
fn cc_control_17_locked_office_uses_document_commands() {
    let target = ComputerControlTarget::WordDocument { document_id: ControlDocumentId("fixture".into()) };
    let action = ComputerControlAction::Word { command: WordCommand::ReplaceText { search: "before".into(), replacement: "after".into() } };
    assert_eq!(require_execution(LockState::Locked, &target, &action), Ok(()));
    assert_eq!(require_execution(LockState::Unknown, &target, &action), Ok(()));
}

#[test]
fn cc_control_17_locked_word_selection_cannot_use_gui_focus() {
    let target = ComputerControlTarget::WordDocument { document_id: ControlDocumentId("fixture".into()) };
    let action = ComputerControlAction::Word { command: WordCommand::InsertText { position: WordInsertPosition::Selection, text: "value".into() } };
    assert_eq!(require_execution(LockState::Locked, &target, &action), Err(ControlErrorCode::Unsupported));
    assert_eq!(require_execution(LockState::Unlocked, &target, &action), Ok(()));
}

#[test]
fn cc_control_17_mismatched_surface_never_enables_locked_executor() {
    let target = ComputerControlTarget::NativeApp { bundle_id: "com.example.App".into(), process_id: 1 };
    let action = ComputerControlAction::Chrome { command: ChromeCommand::Snapshot };
    assert!(!structured_execution(&target, &action));
}
