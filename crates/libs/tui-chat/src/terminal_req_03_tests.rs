use super::*;

#[test]
fn terminal_req_03_overlay_does_not_read_clipboard_or_change_draft() {
    let mut widget = ChatWidget::new(ChatWidgetConfig::default());
    widget.replace_draft("中文草稿🙂");
    widget.show_selection_overlay("test", "选择", vec![V3SelectionItem::new("one", "一")]);
    widget.handle_key_event_with_image_reader(
        KeyEvent::new(KeyCode::Char('v'), KeyModifiers::CONTROL),
        || panic!("overlay must isolate the system clipboard"),
    );
    assert!(widget.has_overlay());
    assert_eq!(widget.draft(), "中文草稿🙂");
}

#[test]
fn terminal_req_03_altgr_does_not_read_clipboard() {
    let mut widget = ChatWidget::new(ChatWidgetConfig::default());
    widget.handle_key_event_with_image_reader(
        KeyEvent::new(KeyCode::Char('v'), KeyModifiers::CONTROL | KeyModifiers::ALT),
        || panic!("AltGr must not read the system clipboard"),
    );
}

#[test]
fn terminal_req_03_clipboard_error_preserves_unicode_draft_and_reports_error() {
    let mut widget = ChatWidget::new(ChatWidgetConfig::default());
    widget.replace_draft("中文草稿🙂");
    let action = widget.handle_key_event_with_image_reader(
        KeyEvent::new(KeyCode::Char('v'), KeyModifiers::ALT),
        || Err("剪贴板中没有可用图片".into()),
    );
    assert_eq!(action, V3Action::None);
    assert_eq!(widget.draft(), "中文草稿🙂");
    let area = Rect::new(0, 0, 80, 20);
    let mut buffer = Buffer::empty(area);
    widget.render(area, &mut buffer);
    // Ratatui stores a blank continuation cell after each wide Chinese glyph.
    let rendered: String = buffer
        .content
        .iter()
        .map(|cell| cell.symbol())
        .filter(|symbol| *symbol != " ")
        .collect();
    assert!(rendered.contains("剪贴板中没有可用图片"));
}
