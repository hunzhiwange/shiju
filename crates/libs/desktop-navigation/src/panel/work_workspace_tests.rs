use super::{State as App, Message as ViewMessage};
use super::{update as update_panel, WorkResourceMode};
#[test]
fn work_resources_toggle_hides_whole_panel_and_mode_selection_restores_it() {
    let mut app = App::default();
    assert!(!app.work_resources_visible);

    let _task = update_panel(&mut app, ViewMessage::WorkResourcesToggled);
    assert!(app.work_resources_visible);

    app.dragging_work_split = true;
    app.work_split_drag_anchor_x = Some(640.0);
    app.work_resource_fullscreen = true;
    app.work_resource_half_fullscreen = true;

    let _task = update_panel(&mut app, ViewMessage::WorkResourcesToggled);

    assert!(!app.work_resources_visible);
    assert!(!app.dragging_work_split);
    assert!(app.work_split_drag_anchor_x.is_none());
    assert!(!app.work_resource_fullscreen);
    assert!(!app.work_resource_half_fullscreen);

    let _task =
        update_panel(&mut app, ViewMessage::WorkResourceModeSelected(WorkResourceMode::Browser));
    assert!(app.work_resources_visible);
    assert_eq!(app.work_resource_mode, WorkResourceMode::Browser);
}
#[test]
fn work_workspace_space_toggle_is_path_scoped() {
    let mut app = App::default();
    let first_path = "/workspace/project".to_string();
    let second_path = "/workspace/project-copy".to_string();

    let _task = update_panel(&mut app, ViewMessage::WorkSpaceToggled(first_path.clone()));
    assert!(app.work_expanded_spaces.contains(&first_path));
    assert!(!app.work_expanded_spaces.contains(&second_path));

    let _task = update_panel(&mut app, ViewMessage::WorkSpaceToggled(second_path.clone()));
    assert!(app.work_expanded_spaces.contains(&first_path));
    assert!(app.work_expanded_spaces.contains(&second_path));

    let _task = update_panel(&mut app, ViewMessage::WorkSpaceToggled(first_path.clone()));
    assert!(!app.work_expanded_spaces.contains(&first_path));
    assert!(app.work_expanded_spaces.contains(&second_path));
}
