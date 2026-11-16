use super::*;

#[test]
fn editing_last_queued_message_replaces_it_instead_of_duplicating_it() {
    let mut config = ChatWidgetConfig::default();
    config.footer_data.is_task_running = true;
    let mut chat = ChatWidget::new(config);
    chat.queue_input("before edit".to_string(), Vec::new());

    let edit_action = chat.handle_key_event(KeyEvent::new(KeyCode::Up, KeyModifiers::ALT));

    assert_eq!(edit_action, V3Action::None);
    assert_eq!(chat.draft(), "before edit");
    assert_eq!(chat.queued_input_count(), 0);

    chat.replace_draft("after edit");
    let queue_action = chat.handle_key_event(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE));
    let V3Action::Queue { text, attachments } = queue_action else {
        panic!("expected edited draft to be queued");
    };
    chat.queue_input(text, attachments);

    assert_eq!(chat.queued_input_count(), 1);
    assert_eq!(chat.pop_queued_input(), Some(("after edit".to_string(), Vec::new())));
    assert_eq!(chat.pop_queued_input(), None);
}

#[test]
fn shift_left_edits_the_last_queued_message_and_keeps_fifo_order() {
    let mut chat = ChatWidget::new(ChatWidgetConfig::default());
    chat.queue_input("first".to_string(), Vec::new());
    chat.queue_input("second".to_string(), Vec::new());

    chat.handle_key_event(KeyEvent::new(KeyCode::Left, KeyModifiers::SHIFT));

    assert_eq!(chat.draft(), "second");
    assert_eq!(chat.queued_input_count(), 1);
    assert_eq!(chat.pop_queued_input(), Some(("first".to_string(), Vec::new())));
}

#[test]
fn terminal_req_05_queue_edit_preserves_rich_input_once() {
    use tui_contracts::event::{V3Attachment, V3AttachmentKind};
    let mut config = ChatWidgetConfig::default();
    config.footer_data.is_task_running = true;
    let mut chat = ChatWidget::new(config);
    let payload = "中😀".repeat(1001);
    chat.handle_paste(payload.clone());
    let image = V3Attachment {
        kind: V3AttachmentKind::Image,
        label: "clipboard".into(),
        path: None,
        media_type: Some("image/png".into()),
        text: None,
        bytes: Some(vec![1, 2, 3]),
    };
    chat.add_attachment(image.clone());
    let V3Action::Queue { text, attachments } =
        chat.handle_key_event(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE))
    else {
        panic!("expected queue");
    };
    chat.queue_input(text, attachments);
    chat.handle_key_event(KeyEvent::new(KeyCode::Up, KeyModifiers::ALT));
    assert_eq!(chat.queued_input_count(), 0);
    assert_eq!(chat.draft(), "[Pasted text: 2002 chars]");
    let V3Action::Queue { text, attachments } =
        chat.handle_key_event(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE))
    else {
        panic!("expected requeue");
    };
    assert_eq!(text, payload);
    assert_eq!(attachments, vec![image.clone()]);
    chat.queue_input(text, attachments);
    assert_eq!(chat.pop_queued_input(), Some((payload, vec![image])));
    assert_eq!(chat.pop_queued_input(), None);
}
