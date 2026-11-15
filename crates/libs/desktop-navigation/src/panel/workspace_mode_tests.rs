use super::{State as App, Message as ViewMessage, update};
#[test]
fn workspace_mode_popover_toggle_and_close_are_local() {
    let mut app = App::default();
    assert!(!app.show_workspace_mode_popover);

    let _ = update(&mut app, ViewMessage::ToggleWorkspaceModePopover);
    assert!(app.show_workspace_mode_popover);

    let _ = update(&mut app, ViewMessage::CloseWorkspaceModePopover);
    assert!(!app.show_workspace_mode_popover);
}
