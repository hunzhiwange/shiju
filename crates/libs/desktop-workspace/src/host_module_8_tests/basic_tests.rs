use crate::basic::{InputState, handle_input};
type LocalMessage = crate::project_message::message::ProjectMessage<(), (), ()>;
#[test]
fn project_path_changed_updates_input() {
    let mut state = InputState::default();
    let mut files = Vec::new();

    let task = handle_input(
        &mut state,
        &mut files,
        &LocalMessage::ProjectPathChanged("/tmp/project".to_string()),
    );

    assert!(task);
    assert_eq!(state.project_path_input, "/tmp/project");
}

#[test]
fn file_url_changed_and_add_file_pressed_append_file_once() {
    let mut state = InputState::default();
    let mut files = Vec::new();

    let _ = handle_input(
        &mut state,
        &mut files,
        &LocalMessage::FileUrlChanged("https://example.test/a.rs".to_string()),
    );
    let _ = handle_input(&mut state, &mut files, &LocalMessage::AddFilePressed);

    assert_eq!(files, vec!["https://example.test/a.rs".to_string()]);
    assert!(state.file_url_input.is_empty());
}

#[test]
fn add_file_pressed_ignores_empty_input() {
    let mut state = InputState::default();
    let mut files = Vec::new();
    state.file_url_input.clear();

    let _ = handle_input(&mut state, &mut files, &LocalMessage::AddFilePressed);

    assert!(files.is_empty());
}

#[test]
fn remove_attached_file_removes_matching_path() {
    let mut state = InputState::default();
    let mut files = vec!["/tmp/a.txt".to_string(), "/tmp/b.txt".to_string()];

    let _ = handle_input(
        &mut state,
        &mut files,
        &LocalMessage::RemoveAttachedFile("/tmp/a.txt".to_string()),
    );

    assert_eq!(files, vec!["/tmp/b.txt".to_string()]);
}

#[test]
fn remove_attached_file_removes_every_duplicate_and_preserves_other_paths() {
    let mut state = InputState::default();
    let mut files = vec!["a.txt".to_string(), "b.txt".to_string(), "a.txt".to_string()];

    let _ = handle_input(
        &mut state,
        &mut files,
        &LocalMessage::RemoveAttachedFile("a.txt".to_string()),
    );

    assert_eq!(files, ["b.txt"]);
}
