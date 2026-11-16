#[derive(Default)]
struct ResizeFixture {
    terminal: crate::state::TerminalState<crate::Shell>,
    window_size: (f32, f32),
}
fn test_app() -> ResizeFixture {
    ResizeFixture::default()
}
#[test]
fn terminal_drag_anchors_then_clamps_height_to_window() {
    let mut app = test_app();
    app.window_size = (800.0, 500.0);
    app.terminal.is_dragging = true;
    app.terminal.drag_anchor_y = None;
    app.terminal.drag_start_height = 300.0;

    let _task = app.terminal.pointer_resize(400.0, app.window_size.1);
    assert_eq!(app.terminal.drag_anchor_y, Some(400.0));

    let _task = app.terminal.pointer_resize(100.0, app.window_size.1);
    assert_eq!(app.terminal.height, 400.0);

    let _task = app.terminal.pointer_resize(1_000.0, app.window_size.1);
    assert_eq!(app.terminal.height, 120.0);
}

#[test]
fn terminal_resize_tiny_window_keeps_height_within_window_limit() {
    let mut app = test_app();
    app.window_size = (640.0, 100.0);
    app.terminal.is_dragging = true;
    app.terminal.drag_anchor_y = Some(50.0);
    app.terminal.drag_start_height = 120.0;
    let _ = app.terminal.pointer_resize(-100.0, app.window_size.1);
    assert_eq!(app.terminal.height, 80.0);
}
