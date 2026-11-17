use crate::interaction::{InteractionState, handle_local};
type LocalMessage = crate::project_message::message::ProjectMessage<(), (), ()>;
#[test]
fn rename_changed_and_cancel_manage_rename_state() {
    let mut app = InteractionState::default();
    app.file_tree_rename_path = Some("/tmp/a.txt".to_string());

    let _ = handle_local(&mut app, &LocalMessage::FileTreeRenameChanged("b.txt".to_string()));
    assert_eq!(app.file_tree_rename_value, "b.txt");

    let _ = handle_local(&mut app, &LocalMessage::FileTreeRenameCancel);

    assert!(app.file_tree_rename_path.is_none());
    assert!(app.file_tree_rename_value.is_empty());
}

#[test]
fn drag_start_and_end_transfer_pending_drop_state() {
    let mut app = InteractionState::default();

    let _ = handle_local(
        &mut app,
        &LocalMessage::FileTreeDragStart("/tmp/a.txt".to_string(), Some((3, 4))),
    );
    assert_eq!(app.dragging_file_paths, vec!["/tmp/a.txt".to_string()]);
    assert_eq!(app.dragging_file_position, Some((3, 4)));

    let _ = handle_local(&mut app, &LocalMessage::FileTreeDragEnd);

    assert!(app.dragging_file_paths.is_empty());
    assert_eq!(app.pending_drop_file_paths, vec!["/tmp/a.txt".to_string()]);
    assert_eq!(app.pending_drop_file_position, Some((3, 4)));
}

#[test]
fn selected_drag_freezes_all_items_and_unselected_drag_replaces_selection() {
    let mut app = InteractionState::default();
    let visible: Vec<String> = vec!["/project/a.txt".into(), "/project/b.txt".into(), "/project/c.txt".into()];
    for path in &visible[..2] {
        app.select("/project", path, true, false, &visible);
    }
    app.start_internal_drag("/project".into(), "/project/b.txt".into(), &visible);
    assert_eq!(app.dragging_file_paths, ["/project/a.txt", "/project/b.txt"]);
    app.file_tree_selection.paths.clear();
    assert_eq!(app.dragging_file_paths.len(), 2);
    app.clear_drag();
    app.start_internal_drag("/project".into(), "/project/c.txt".into(), &visible);
    assert_eq!(app.dragging_file_paths, ["/project/c.txt"]);
    assert!(app.file_tree_selection.contains("/project", "/project/c.txt"));
    assert_eq!(app.file_tree_selection.paths.len(), 1);
}

#[test]
fn file_tree_right_click_and_close_manage_menu_state() {
    let mut app = InteractionState::default();

    app.right_click(
        None,
        "/tmp/a.txt".to_string(),
        "tree".to_string(),
        iced::Point::new(10.0, 20.0),
        &[],
    );
    assert_eq!(app.file_tree_menu_path.as_deref(), Some("/tmp/a.txt"));
    assert_eq!(app.file_tree_menu_source.as_deref(), Some("tree"));
    assert_eq!(app.file_tree_menu_anchor, Some(iced::Point::new(10.0, 20.0)));

    app.close_menu();

    assert!(app.file_tree_menu_path.is_none());
    assert!(app.file_tree_menu_source.is_none());
    assert!(app.file_tree_menu_anchor.is_none());
}
