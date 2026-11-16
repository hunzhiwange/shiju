use super::PickerFixture;
use crate::project_message::session::overlay::{self, Effect, Event, State};
#[derive(Default)]
struct Fixture {
    picker: PickerFixture,
    hovered_recent_project: Option<String>,
    session_menu_id: Option<String>,
    session_menu_anchor: Option<iced::Point>,
    project_tools_menu_path: Option<String>,
    code_menu: bool,
    creating: bool,
}
fn app() -> Fixture {
    Fixture::default()
}
fn apply(app: &mut Fixture, event: Event) -> Option<Vec<Effect>> {
    let effects = overlay::update(
        &mut State {
            code_menu: &mut app.code_menu,
            hovered: &mut app.hovered_recent_project,
            menu_id: &mut app.session_menu_id,
            menu_anchor: &mut app.session_menu_anchor,
            tools_path: &mut app.project_tools_menu_path,
            creating: &mut app.creating,
            picker_open: app.picker.new_session_picker_project.is_some(),
        },
        event,
    );
    for effect in &effects {
        if matches!(effect, Effect::ResetPicker) {
            app.picker.reset();
        }
    }
    Some(effects)
}
#[test]
fn open_recent_pressed_clears_hover_menus_and_new_session_picker_state() {
    let mut app = app();
    app.hovered_recent_project = Some("/tmp/project".to_string());
    app.session_menu_id = Some("session-1".to_string());
    app.session_menu_anchor = Some(iced::Point::new(8.0, 12.0));
    app.picker.new_session_picker_project = Some("/tmp/project".to_string());
    app.picker.new_session_picker_options =
        vec![("/tmp/worktree".to_string(), "Worktree".to_string())];
    app.picker.new_session_worktree_name = "feature".to_string();
    app.picker.new_session_delete_error = Some("stale delete".to_string());

    let task = apply(&mut app, Event::Open("/tmp/project".to_string()));

    assert!(task.is_some());
    assert!(app.hovered_recent_project.is_none());
    assert!(app.session_menu_id.is_none());
    assert!(app.session_menu_anchor.is_none());
    assert!(app.picker.new_session_picker_project.is_none());
    assert!(app.picker.new_session_picker_options.is_empty());
    assert!(app.picker.new_session_worktree_name.is_empty());
    assert!(app.picker.new_session_delete_error.is_none());
}
#[test]
fn recent_overlay_closed_preserves_open_picker_but_otherwise_resets_overlay_state() {
    let mut app = app();
    app.picker.new_session_picker_project = Some("/tmp/project".to_string());
    app.hovered_recent_project = Some("/tmp/project".to_string());

    let _ = apply(&mut app, Event::Close);
    assert!(app.picker.new_session_picker_project.is_some());
    assert!(app.hovered_recent_project.is_some());

    app.picker.new_session_picker_project = None;
    app.session_menu_id = Some("s1".to_string());
    app.project_tools_menu_path = Some("/tmp/project".to_string());

    let _ = apply(&mut app, Event::Close);

    assert!(app.hovered_recent_project.is_none());
    assert!(app.session_menu_id.is_none());
    assert!(app.project_tools_menu_path.is_none());
}
#[test]
fn recent_overlay_closed_clears_anchor_and_all_picker_state() {
    let mut app = app();
    app.hovered_recent_project = Some("/tmp/project".to_string());
    app.session_menu_id = Some("session-1".to_string());
    app.session_menu_anchor = Some(iced::Point::new(12.0, 24.0));
    app.project_tools_menu_path = Some("/tmp/project".to_string());
    app.picker.new_session_picker_options =
        vec![("/tmp/worktree".to_string(), "Worktree".to_string())];
    app.picker.new_session_worktree_name = "feature".to_string();
    app.picker.new_session_confirm_delete_directory = Some("/tmp/worktree".to_string());
    app.picker.new_session_delete_error = Some("old error".to_string());

    let _ = apply(&mut app, Event::Close);

    assert!(app.hovered_recent_project.is_none());
    assert!(app.session_menu_id.is_none());
    assert!(app.session_menu_anchor.is_none());
    assert!(app.project_tools_menu_path.is_none());
    assert!(app.picker.new_session_picker_options.is_empty());
    assert!(app.picker.new_session_worktree_name.is_empty());
    assert!(app.picker.new_session_confirm_delete_directory.is_none());
    assert!(app.picker.new_session_delete_error.is_none());
}
