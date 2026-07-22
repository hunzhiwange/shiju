use super::{TerminalMessage, update};
use crate::app::{App, FocusArea};

fn test_app() -> App {
    App::new().0
}

#[test]
fn select_routes_to_tabs_and_closes_context_menu_for_unknown_tab() {
    let mut app = test_app();
    app.focus_area = FocusArea::Preview;
    app.terminal.tab_context_menu_id = Some(7);
    app.terminal.tab_context_menu_pos = Some((12.0, 24.0));

    let _task = update(&mut app, TerminalMessage::Select(u64::MAX));

    assert_eq!(app.focus_area, FocusArea::Terminal);
    assert_eq!(app.terminal.tab_context_menu_id, None);
    assert_eq!(app.terminal.tab_context_menu_pos, None);
}

#[test]
fn rename_cancel_routes_to_rename_and_closes_context_menu_for_unknown_tab() {
    let mut app = test_app();
    app.terminal.tab_context_menu_id = Some(9);
    app.terminal.tab_context_menu_pos = Some((3.0, 6.0));

    let _task = update(&mut app, TerminalMessage::RenameCancel(u64::MAX));

    assert_eq!(app.terminal.tab_context_menu_id, None);
    assert_eq!(app.terminal.tab_context_menu_pos, None);
}

#[test]
fn drag_started_routes_to_event_and_captures_drag_origin() {
    let mut app = test_app();
    app.cursor_position.y = 48.5;
    app.terminal.height = 320.0;
    app.terminal.is_dragging = false;
    app.terminal.drag_anchor_y = None;
    app.terminal.drag_start_height = 0.0;

    let _task = update(&mut app, TerminalMessage::DragStarted);

    assert!(app.terminal.is_dragging);
    assert_eq!(app.terminal.drag_anchor_y, Some(48.5));
    assert_eq!(app.terminal.drag_start_height, 320.0);
}

#[test]
fn tab_context_close_routes_to_tabs_and_clears_menu_coordinates() {
    let mut app = test_app();
    app.terminal.tab_context_menu_id = Some(17);
    app.terminal.tab_context_menu_pos = Some((8.0, 16.0));

    let _task = update(&mut app, TerminalMessage::TabContextClose);

    assert_eq!(app.terminal.tab_context_menu_id, None);
    assert_eq!(app.terminal.tab_context_menu_pos, None);
}
