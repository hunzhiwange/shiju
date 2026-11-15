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
