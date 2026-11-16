use super::{State as App, Message as ViewMessage, update};
#[test]
fn workspace_mode_popover_toggle_and_close_are_local() {
    let mut app = App::default();
    let mut panels = super::super::settings::State::default();
    assert!(!app.show_workspace_mode_popover);

    let _ = update(&mut app, &mut panels, ViewMessage::ToggleWorkspaceModePopover);
    assert!(app.show_workspace_mode_popover);

    let _ = update(&mut app, &mut panels, ViewMessage::CloseWorkspaceModePopover);
    assert!(!app.show_workspace_mode_popover);
}
