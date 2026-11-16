//! 从宿主任务 23 迁入的局部路径、剪贴板载荷与跟随期限行为。
use crate::message::chat::context::mention_display_path;
use crate::message::chat::input::clipboard::ClipboardPastePayload;
use crate::message::chat::orchestration::{arm_autoscroll_hold, AUTOSCROLL_HOLD_MS};

#[test]
fn mention_display_path_makes_project_paths_relative() {
    let project_path = Some("/workspace");

    assert_eq!(mention_display_path(project_path, "/workspace/src/main.rs"), "src/main.rs");
    assert_eq!(mention_display_path(project_path, "/workspace"), "");
}

#[test]
fn mention_display_path_preserves_outside_path_and_normalizes_separators() {
    let project_path = Some("/workspace");

    assert_eq!(mention_display_path(project_path, r"C:\repo\src\main.rs"), "C:/repo/src/main.rs");
}

#[test]
fn mention_display_path_without_project_normalizes_separators() {
    let project_path = None;

    assert_eq!(mention_display_path(project_path, r"src\nested\lib.rs"), "src/nested/lib.rs");
}

#[test]
fn clipboard_payload_variants_are_constructible() {
    let payloads = [
        ClipboardPastePayload::Text("text".to_string()),
        ClipboardPastePayload::AttachmentPath("/tmp/a.png".to_string()),
        ClipboardPastePayload::Empty,
        ClipboardPastePayload::Error("boom".to_string()),
    ];

    assert_eq!(payloads.len(), 4);
}

#[test]
fn arm_autoscroll_hold_enables_follow_mode_and_extends_deadline() {
    let mut generation = 0;
    let mut auto_scroll = false;
    let mut hold_until = 0;
    let now = 1_000;

    arm_autoscroll_hold(&mut generation, &mut auto_scroll, &mut hold_until, now);

    assert!(auto_scroll);
    assert!(hold_until >= AUTOSCROLL_HOLD_MS);
    assert_eq!(hold_until, now + AUTOSCROLL_HOLD_MS);
    assert_eq!(generation, 1);
}
