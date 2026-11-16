use crate::interaction::{FileTreeClipboard, FileTreeClipboardMode, InteractionState};
fn visible() -> Vec<String> {
    vec!["/project/a.txt".into(), "/project/z.txt".into()]
}
fn selected() -> InteractionState {
    let mut state = InteractionState::default();
    let visible = visible();
    for path in &visible {
        state.select("/project", path, true, false, &visible);
    }
    state
}
#[test]
fn right_clicking_unselected_item_replaces_selection() {
    let mut app = selected();
    let visible = vec!["/project/a.txt".into(), "/project/z.txt".into(), "/project/b.txt".into()];
    app.right_click(
        Some("/project"),
        "/project/b.txt".into(),
        "tree-file".into(),
        iced::Point::ORIGIN,
        &visible,
    );
    assert_eq!(app.menu_paths(Some("/project"), &visible), ["/project/b.txt"]);
    assert_eq!(app.file_tree_selection.paths.len(), 1);
}

#[test]
fn partial_cut_completion_keeps_failed_sources_and_does_not_clear_new_clipboard() {
    let mut app = selected();
    app.set_clipboard(FileTreeClipboardMode::Cut, visible());
    let clipboard = app.file_tree_clipboard.clone().unwrap();
    app.complete_paste(clipboard.clone(), &["/project/a.txt".into()]);
    assert_eq!(app.file_tree_clipboard.as_ref().unwrap().src_paths, ["/project/z.txt"]);
    let replacement = FileTreeClipboard {
        mode: FileTreeClipboardMode::Copy,
        src_paths: vec!["/project/new.txt".into()],
    };
    app.file_tree_clipboard = Some(replacement.clone());
    app.complete_paste(clipboard, &["/project/a.txt".into()]);
    assert_eq!(app.file_tree_clipboard, Some(replacement));
}

#[test]
fn partial_delete_completion_only_clears_successful_selection_in_original_project() {
    let mut app = selected();
    app.complete_delete(true, &["/project/a.txt".into()]);
    assert_eq!(
        app.file_tree_selection.paths.iter().cloned().collect::<Vec<_>>(),
        ["/project/z.txt"]
    );
    let selection = app.file_tree_selection.paths.clone();
    app.complete_delete(false, &["/project/a.txt".into()]);
    assert_eq!(app.file_tree_selection.paths, selection);
}
