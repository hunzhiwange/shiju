#[derive(Default)]
struct InteractionFixture {
    file_tree_interaction: desktop_workspace::interaction::InteractionState,
}
fn test_app() -> InteractionFixture {
    InteractionFixture::default()
}
#[test]
fn hovered_file_paths_are_unique_and_blocked_during_internal_drag() {
    let mut app = test_app();

    let _task = app.file_tree_interaction.hover_external_file("first.rs".to_owned());
    app.file_tree_interaction.pending_drop_file_position = Some((2, 3));
    let _task = app.file_tree_interaction.hover_external_file("first.rs".to_owned());
    app.file_tree_interaction.dragging_file_paths.push("internal.rs".to_owned());
    let _task = app.file_tree_interaction.hover_external_file("blocked.rs".to_owned());

    assert_eq!(app.file_tree_interaction.pending_drop_file_paths, vec!["first.rs"]);
    assert_eq!(app.file_tree_interaction.pending_drop_file_position, Some((2, 3)));
}

#[test]
fn hovered_files_left_clears_external_hover_but_preserves_internal_drag() {
    let mut app = test_app();
    app.file_tree_interaction.pending_drop_file_paths = vec!["external.rs".to_owned()];
    app.file_tree_interaction.pending_drop_file_position = Some((1, 2));
    app.file_tree_interaction.input_drop_hovered = true;

    let _task = app.file_tree_interaction.leave_external_files();
    assert!(app.file_tree_interaction.pending_drop_file_paths.is_empty());
    assert_eq!(app.file_tree_interaction.pending_drop_file_position, None);
    assert!(!app.file_tree_interaction.input_drop_hovered);

    app.file_tree_interaction.dragging_file_paths = vec!["internal.rs".to_owned()];
    app.file_tree_interaction.pending_drop_file_paths = vec!["kept.rs".to_owned()];
    app.file_tree_interaction.input_drop_hovered = true;
    let _task = app.file_tree_interaction.leave_external_files();
    assert_eq!(app.file_tree_interaction.pending_drop_file_paths, vec!["kept.rs"]);
    assert!(app.file_tree_interaction.input_drop_hovered);
}
