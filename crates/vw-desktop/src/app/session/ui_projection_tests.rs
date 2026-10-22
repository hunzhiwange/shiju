use super::*;

#[test]
fn scroll_and_animation_preserve_projection_but_data_events_invalidate() {
    let mut app = App::new().0;
    let snapshot = Arc::new(Vec::new());
    for message in [
        Message::View(ViewMessage::PointerMoved(10.0, 20.0)),
        Message::View(ViewMessage::ActivityAnimationTick),
        Message::Chat(ChatMessage::ThinkHover(1, 0)),
        Message::Chat(ChatMessage::ThinkHoverLeave),
        Message::Chat(ChatMessage::ToolHover(1, 0)),
        Message::Chat(ChatMessage::ToolHoverLeave),
        Message::Chat(ChatMessage::FullscreenOverlayEntered),
        Message::Chat(ChatMessage::FullscreenOverlayExited),
        Message::Chat(ChatMessage::MarkdownBlockEditorAction(
            1,
            0,
            0,
            iced::widget::text_editor::Action::SelectAll,
        )),
        Message::Chat(ChatMessage::ScrollChanged {
            offset_y: 0.5,
            absolute_offset_y: 200.0,
            viewport_h: 600.0,
        }),
        Message::Chat(ChatMessage::VisibleMessageMeasurements(Vec::new())),
    ] {
        *app.session_ui_projection.agent_sessions.borrow_mut() = Some(snapshot.clone());
        let _ = app.prepare_session_ui_projection(&message);
        assert!(Arc::ptr_eq(
            app.session_ui_projection.agent_sessions.borrow().as_ref().unwrap(),
            &snapshot
        ));
    }
    let changed = Message::Project(ProjectMessage::SessionsLoaded(Ok(Vec::new())));
    app.prepare_session_ui_projection(&changed);
    assert!(app.session_ui_projection.agent_sessions.borrow().is_none());
}

#[test]
fn latest_pin_snapshot_wins_and_failure_preserves_previous_pins() {
    let mut app = App::new().0;
    app.session_ui_projection.pins_generation = 2;
    let pins = HashSet::from([("project".into(), "task".into())]);
    app.apply_session_pins(2, Ok(pins.clone()));
    app.apply_session_pins(1, Ok(HashSet::new()));
    assert_eq!(app.session_ui_projection.pinned, pins);
    app.apply_session_pins(2, Err("unavailable".into()));
    assert_eq!(app.session_ui_projection.pinned, pins);
    app.apply_session_pins(2, Ok(HashSet::new()));
    assert!(app.session_ui_projection.pinned.is_empty());
}
