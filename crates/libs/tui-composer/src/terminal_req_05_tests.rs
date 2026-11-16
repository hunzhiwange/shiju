use super::*;
use tui_contracts::event::V3AttachmentKind;

fn attachment(label: &str) -> V3Attachment {
    V3Attachment {
        kind: V3AttachmentKind::Image,
        label: label.into(),
        path: None,
        media_type: Some("image/png".into()),
        text: None,
        bytes: Some(vec![1, 2, 3]),
    }
}

#[test]
fn terminal_req_05_history_preserves_ordered_attachments() {
    let mut composer = ChatComposer::new();
    composer.replace_text("图片😀");
    let images = vec![attachment("first"), attachment("second")];
    for image in &images {
        composer.add_attachment(image.clone());
    }
    composer.submit_or_queue(false);
    assert!(composer.restore_history_previous());
    assert_eq!(composer.text(), "图片😀");
    assert_eq!(composer.attachments(), images);
}

#[test]
fn terminal_req_05_attachment_only_submission_is_recalled() {
    let mut composer = ChatComposer::new();
    composer.add_attachment(attachment("only"));
    composer.submit_or_queue(false);
    assert!(composer.restore_history_previous());
    assert_eq!(composer.attachments(), &[attachment("only")]);
}

#[test]
fn terminal_req_05_same_text_with_different_attachments_is_distinct() {
    let mut composer = ChatComposer::new();
    for label in ["first", "second"] {
        composer.replace_text("same");
        composer.add_attachment(attachment(label));
        composer.submit_or_queue(false);
    }
    assert_eq!(composer.history.entries().len(), 2);
    composer.restore_history_previous();
    assert_eq!(composer.attachments(), &[attachment("second")]);
    composer.restore_history_previous();
    assert_eq!(composer.attachments(), &[attachment("first")]);
}

#[test]
fn terminal_req_05_navigation_returns_rich_unicode_draft_and_cursor() {
    let mut composer = ChatComposer::new();
    composer.replace_text("old");
    composer.submit_or_queue(false);
    composer.replace_text("中😀suffix");
    composer.textarea.set_cursor("中".len());
    composer.handle_paste("粘".repeat(1001));
    composer.add_attachment(attachment("draft"));
    let draft = composer.textarea.draft();
    composer.restore_history_previous();
    assert!(composer.attachments().is_empty());
    assert!(composer.restore_history_next());
    assert_eq!(composer.textarea.draft(), draft);
    assert_eq!(composer.attachments(), &[attachment("draft")]);
}

#[test]
fn terminal_req_05_history_retains_atomic_paste_payloads() {
    let mut composer = ChatComposer::new();
    let payload = "😀".repeat(1001);
    composer.handle_paste(payload.clone());
    composer.submit_or_queue(false);
    composer.restore_history_previous();
    assert_eq!(composer.text(), "[Pasted text: 1001 chars]");
    assert_eq!(composer.expanded_text(), payload);
    composer.textarea.input(KeyEvent::new(KeyCode::Backspace, KeyModifiers::NONE));
    assert!(composer.expanded_text().is_empty());
}

#[test]
fn terminal_req_05_failed_submission_restores_rich_draft_without_duplicate_attachment() {
    let mut composer = ChatComposer::new();
    composer.handle_paste("粘".repeat(1001));
    composer.add_attachment(attachment("image"));
    let draft = composer.textarea.draft();
    let V3Action::Submit { text, attachments } = composer.submit_or_queue(false) else {
        panic!("expected submission");
    };
    composer.restore_submission(&text, attachments.clone());
    composer.restore_submission(&text, attachments.clone());
    assert_eq!(composer.textarea.draft(), draft);
    let V3Action::Submit { text: retried, attachments: images } = composer.submit_or_queue(false)
    else {
        panic!("expected retry");
    };
    assert_eq!(retried, text);
    assert_eq!(images, attachments);
}

#[test]
fn terminal_req_05_cancel_resets_navigation_and_discards_rich_draft() {
    let mut composer = ChatComposer::new();
    composer.replace_text("history");
    composer.submit_or_queue(false);
    composer.handle_paste("粘".repeat(1001));
    composer.add_attachment(attachment("cancelled"));
    composer.restore_history_previous();
    composer.handle_key_event(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
    assert!(!composer.restore_history_next());
    assert!(composer.text().is_empty());
    assert!(composer.attachments().is_empty());
}

#[test]
fn terminal_req_05_unmatched_failure_replaces_attachments() {
    let mut composer = ChatComposer::new();
    composer.add_attachment(attachment("stale"));
    composer.restore_submission("missing history 中😀", vec![attachment("restored")]);
    assert_eq!(composer.expanded_text(), "missing history 中😀");
    assert_eq!(composer.attachments(), &[attachment("restored")]);
}

#[test]
fn terminal_req_05_repeated_rich_submission_deduplicates_and_resets_navigation() {
    let mut composer = ChatComposer::new();
    for _ in 0..2 {
        composer.handle_paste("x".repeat(1001));
        composer.add_attachment(attachment("same"));
        composer.submit_or_queue(false);
    }
    assert_eq!(composer.history.entries().len(), 1);
    composer.restore_history_previous();
    composer.submit_or_queue(false);
    assert!(!composer.restore_history_next());
}

#[test]
fn terminal_req_05_multiple_atomic_pastes_preserve_insertion_order() {
    let mut composer = ChatComposer::new();
    let first = "中".repeat(1001);
    let second = "😀".repeat(1002);
    composer.handle_paste(first.clone());
    composer.textarea.insert_str(" / ");
    composer.handle_paste(second.clone());
    composer.submit_or_queue(false);
    composer.restore_history_previous();
    assert_eq!(composer.expanded_text(), format!("{first} / {second}"));
    assert_eq!(composer.textarea.text_element_ranges().count(), 2);
}
