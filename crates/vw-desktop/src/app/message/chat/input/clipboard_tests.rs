use super::{
    clipboard_attachment_payload, clipboard_image_dimensions, clipboard_text_payload,
    resolved_clipboard_message,
};
use crate::app::Message;
use crate::app::message::chat::{ChatMessage, ClipboardPastePayload};

#[test]
fn clipboard_tests_module_is_wired() {
    assert!(module_path!().ends_with("clipboard_tests"));
}

#[test]
fn read_clipboard_for_input_builds_task_without_touching_callers_state() {
    let _task = super::read_clipboard_for_input();
}

#[test]
fn clipboard_text_payload_preserves_non_empty_text() {
    let payload = clipboard_text_payload(Some("clipboard text".to_string()));

    assert!(matches!(
        payload,
        ClipboardPastePayload::Text(text) if text == "clipboard text"
    ));
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn clipboard_text_payload_caches_text_above_two_hundred_characters_as_attachment() {
    let content = "中".repeat(201);
    let payload = clipboard_text_payload(Some(content.clone()));

    let ClipboardPastePayload::AttachmentPath(path) = payload else {
        panic!("large clipboard text should become a document attachment");
    };
    assert_eq!(std::fs::read_to_string(&path).unwrap(), content);

    std::fs::remove_file(path).unwrap();
}

#[test]
fn clipboard_text_payload_treats_empty_or_unavailable_content_as_empty() {
    assert!(matches!(clipboard_text_payload(Some(String::new())), ClipboardPastePayload::Empty));
    assert!(matches!(clipboard_text_payload(None), ClipboardPastePayload::Empty));
}

#[test]
fn clipboard_text_payload_preserves_whitespace_only_text() {
    assert!(matches!(
        clipboard_text_payload(Some("   ".to_string())),
        ClipboardPastePayload::Text(text) if text == "   "
    ));
}

#[test]
fn clipboard_image_dimensions_accept_zero_and_u32_limits() {
    assert_eq!(clipboard_image_dimensions(0, 0), Ok((0, 0)));

    let maximum = usize::try_from(u32::MAX).expect("u32 must fit in usize on supported targets");
    assert_eq!(clipboard_image_dimensions(maximum, maximum), Ok((u32::MAX, u32::MAX)));
}

#[cfg(target_pointer_width = "64")]
#[test]
fn clipboard_image_dimensions_reports_width_and_height_overflow() {
    let overflow = u32::MAX as usize + 1;

    assert_eq!(clipboard_image_dimensions(overflow, 1), Err("剪贴板图片宽度超出范围"));
    assert_eq!(clipboard_image_dimensions(1, overflow), Err("剪贴板图片高度超出范围"));
}

#[test]
fn clipboard_attachment_payload_preserves_success_path_and_error() {
    let success = clipboard_attachment_payload(Ok("/tmp/clipboard.png".to_string()));
    let failure = clipboard_attachment_payload(Err("persist failed".to_string()));

    assert!(matches!(
        success,
        ClipboardPastePayload::AttachmentPath(path) if path == "/tmp/clipboard.png"
    ));
    assert!(matches!(
        failure,
        ClipboardPastePayload::Error(error) if error == "persist failed"
    ));
}

#[test]
fn clipboard_attachment_payload_preserves_empty_success_path() {
    assert!(matches!(
        clipboard_attachment_payload(Ok(String::new())),
        ClipboardPastePayload::AttachmentPath(path) if path.is_empty()
    ));
}

#[test]
fn resolved_clipboard_message_wraps_payload_for_chat_update() {
    let message = resolved_clipboard_message(ClipboardPastePayload::Text("paste me".to_string()));

    assert!(matches!(
        message,
        Message::Chat(ChatMessage::ClipboardPasteResolved(ClipboardPastePayload::Text(text)))
            if text == "paste me"
    ));
}
