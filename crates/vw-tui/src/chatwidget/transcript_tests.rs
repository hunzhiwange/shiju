use super::*;

fn mouse(kind: MouseEventKind, column: u16, row: u16) -> MouseEvent {
    MouseEvent {
        kind,
        column,
        row,
        modifiers: crossterm::event::KeyModifiers::NONE,
    }
}

#[test]
fn active_cell_commits_into_history_and_invalidates_layout() {
    let mut state = TranscriptState::default();
    state.set_active_event(Some(V3DisplayEvent::Progress {
        label: "working".into(),
        detail: None,
    }));
    state.active_cell_layout.set(Some(ActiveCellLayoutCache {
        key: ActiveCellLayoutCacheKey {
            cell_identity: 1,
            revision: state.active_cell_revision,
            width: 80,
            syntax_theme_revision: 0,
        },
        desired_height: 1,
    }));

    state.commit_active_cell();

    assert_eq!(state.committed_history.len(), 1);
    assert!(state.active_cell.is_none());
    assert_eq!(state.active_cell_layout.get(), None);
}

#[test]
fn transcript_lines_join_committed_active_and_live_cells() {
    let mut state = TranscriptState::default();
    state.push_history(V3DisplayEvent::System { text: "committed".into() });
    state.set_active_event(Some(V3DisplayEvent::Progress {
        label: "active".into(),
        detail: None,
    }));
    state.set_live_tail_event(Some(V3DisplayEvent::Assistant { markdown: "live".into() }));

    let committed = state
        .transcript_lines(40, false)
        .into_iter()
        .map(|line| line.to_string())
        .collect::<Vec<_>>()
        .join("\n");
    let full = state
        .transcript_lines(40, true)
        .into_iter()
        .map(|line| line.to_string())
        .collect::<Vec<_>>()
        .join("\n");

    assert!(committed.contains("committed"));
    assert!(!committed.contains("active"));
    assert!(!committed.contains("live"));
    assert!(full.contains("committed"));
    assert!(full.contains("active"));
    assert!(full.contains("live"));
}

#[test]
fn session_header_metadata_updates_in_place() {
    let mut state = TranscriptState::default();
    let display = |title: &str| crate::V3SessionDisplayData {
        version: "0.4.0".into(),
        cwd: "/workspace".into(),
        gateway_endpoint: "127.0.0.1:42617".into(),
        model: "token-plan/qwen3.7-plus".into(),
        permissions: "Default · full access".into(),
        session_id: Some("session-1".into()),
        session_title: Some(title.into()),
        token_usage: crate::V3TokenUsage::default(),
        resumed: true,
    };
    state.push_cell(Box::new(crate::history_cell::SessionHistoryCell::header(display("Old"))));
    state.push_history(V3DisplayEvent::System { text: "history".into() });

    state.set_session_header(display("Generated title"));

    assert_eq!(state.history_len(), 2);
    let rendered = state
        .transcript_lines(80, false)
        .into_iter()
        .map(|line| line.to_string())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(rendered.contains("session:     Generated title"));
    assert!(!rendered.contains("session:     Old"));
    assert!(rendered.contains("history"));
}

#[test]
fn scrollbar_track_click_and_thumb_drag_update_scroll_position() {
    let state = TranscriptState::default();
    state.last_area.set(Rect::new(0, 0, 20, 10));
    state.last_content_area.set(Rect::new(0, 0, 19, 10));
    state.last_content_height.set(100);
    state.last_viewport_height.set(10);
    state.follow_bottom.set(false);

    state.handle_mouse_event(mouse(MouseEventKind::Down(MouseButton::Left), 19, 5));
    assert_eq!(state.scroll_top.get(), 50);

    state.handle_mouse_event(mouse(MouseEventKind::Drag(MouseButton::Left), 19, 9));
    assert_eq!(state.scroll_top.get(), 90);
    assert!(state.follow_bottom.get());

    state.handle_mouse_event(mouse(MouseEventKind::Up(MouseButton::Left), 19, 9));
    assert_eq!(state.scrollbar_drag_offset.get(), None);
}

#[test]
fn transcript_drag_selects_visible_text_for_copy() {
    let state = TranscriptState::default();
    state.last_area.set(Rect::new(0, 0, 6, 2));
    state.last_content_area.set(Rect::new(0, 0, 5, 2));
    *state.visible_cells.borrow_mut() = ["hello", "world"]
        .into_iter()
        .map(|line| line.chars().map(|character| character.to_string()).collect())
        .collect();

    state.handle_mouse_event(mouse(MouseEventKind::Down(MouseButton::Left), 1, 0));
    state.handle_mouse_event(mouse(MouseEventKind::Drag(MouseButton::Left), 3, 1));
    let result = state.handle_mouse_event(mouse(MouseEventKind::Up(MouseButton::Left), 3, 1));

    assert!(matches!(result, TranscriptMouseAction::Copy(text) if text == "ello\nworl"));
}
