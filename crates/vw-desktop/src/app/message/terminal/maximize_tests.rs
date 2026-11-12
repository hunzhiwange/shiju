use super::*;
use iced::widget::pane_grid::Direction;

#[tokio::test(flavor = "multi_thread")]
async fn terminal_title_double_click_maximizes_and_restores_the_original_layout() {
    let mut h = Harness::new().await;
    let instance = h.app.terminal.panes.instance.clone();
    let left = h.app.terminal.panes.focused;
    let first = h.app.terminal.active_id.unwrap();
    h.message(TerminalMessage::Split(instance.clone(), left, Direction::Right)).await;
    let right = h.app.terminal.panes.focused;
    let second = h.app.terminal.active_id.unwrap();
    let layout = format!("{:?}", h.app.terminal.panes.grid.layout());

    h.click(Point::new(35.0, 291.0)).await;
    assert_eq!(h.app.terminal.panes.grid.maximized(), None);
    assert_eq!(h.app.terminal.active_id, Some(first));
    h.click(Point::new(35.0, 291.0)).await;
    assert_eq!(h.app.terminal.panes.grid.maximized(), Some(left));
    assert_eq!(h.app.terminal.active_id, Some(first));
    assert!(!h.app.terminal.panes.is_visible_tab(second));
    send_terminal_input(&mut h, first, character_key("x"), Point::new(600.0, 360.0), b"x").await;

    // Double-clicking the blank part of the title bar restores the same split proportions.
    h.click(Point::new(350.0, 291.0)).await;
    assert_eq!(h.app.terminal.panes.grid.maximized(), Some(left));
    h.click(Point::new(350.0, 291.0)).await;
    assert_eq!(h.app.terminal.panes.grid.maximized(), None);
    assert_eq!(format!("{:?}", h.app.terminal.panes.grid.layout()), layout);

    h.click(Point::new(435.0, 291.0)).await;
    assert_eq!(h.app.terminal.panes.grid.maximized(), None);
    h.click(Point::new(435.0, 291.0)).await;
    assert_eq!(h.app.terminal.panes.grid.maximized(), Some(right));
    h.click(Point::new(35.0, 291.0)).await;
    assert_eq!(h.app.terminal.panes.grid.maximized(), Some(right));
    h.click(Point::new(35.0, 291.0)).await;
    assert_eq!(h.app.terminal.panes.grid.maximized(), None);

    h.message(TerminalMessage::TogglePaneMaximized(instance, right)).await;
    h.message(TerminalMessage::Close(second)).await;
    assert_eq!(h.app.terminal.panes.grid.maximized(), None);
    assert!(h.app.terminal.panes.is_visible_tab(first));
}
