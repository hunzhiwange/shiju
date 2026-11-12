use super::*;
use crate::app::message::{ProjectMessage as P, ViewMessage};
use iced::keyboard::Modifiers;

fn selected_app() -> App {
    let mut app = App::new().0;
    app.project_path = Some("/project".into());
    app.lsp_disabled = true;
    app.set_file_index(
        "/project",
        vec!["/project/a.txt".into(), "/project/b.txt".into(), "/project/c.txt".into()],
    );
    app.file_tree_expanded.clear();
    app.file_tree_expanded_set.clear();
    for path in ["/project/a.txt", "/project/b.txt"] {
        let _ = super::super::handle(
            &mut app,
            P::FileTreeSelected {
                project_path: "/project".into(),
                path: path.into(),
                directory: false,
                modifiers: Modifiers::COMMAND,
            },
        );
    }
    app
}

fn start(app: &mut App, path: &str) {
    let _ = super::super::handle(
        app,
        P::FileTreeInternalDragStart { project_path: "/project".into(), source_path: path.into() },
    );
}

#[test]
fn selected_drag_freezes_all_items_and_unselected_drag_replaces_selection() {
    let mut app = selected_app();
    start(&mut app, "/project/b.txt");
    assert_eq!(app.dragging_file_paths, ["/project/a.txt", "/project/b.txt"]);
    app.file_tree_selection.paths.clear();
    assert_eq!(app.dragging_file_paths.len(), 2);
    clear_drag(&mut app);
    start(&mut app, "/project/c.txt");
    assert_eq!(app.dragging_file_paths, ["/project/c.txt"]);
    assert!(app.file_tree_selection.contains("/project", "/project/c.txt"));
    assert_eq!(app.file_tree_selection.paths.len(), 1);
}

#[tokio::test]
async fn selected_drag_chat_delivery_contains_every_item_for_both_release_orders() {
    use iced::futures::StreamExt;
    for global_first in [true, false] {
        let mut app = selected_app();
        start(&mut app, "/project/a.txt");
        app.input_drop_hovered = true;
        if !global_first {
            let _ = super::super::handle(&mut app, P::FileTreeDragEnd);
        }
        let task = crate::app::message::view::update(&mut app, ViewMessage::GlobalMouseReleased);
        if global_first {
            let _ = super::super::handle(&mut app, P::FileTreeDragEnd);
        }
        let mut stream = iced_runtime::task::into_stream(task).unwrap();
        assert!(matches!(
            stream.next().await,
            Some(iced_runtime::Action::Output(Message::Chat(
                crate::app::message::ChatMessage::InputAreaDragDrop
            )))
        ));
        assert_eq!(app.pending_drop_file_paths, ["/project/a.txt", "/project/b.txt"]);
        assert!(app.dragging_file_paths.is_empty());
        assert!(app.file_tree_drag_source.is_none());
        assert!(!app.file_tree_operation_in_progress);
    }
}

#[test]
fn selected_drag_folder_release_is_consumed_once_and_cancel_clears_entire_gesture() {
    for global_first in [true, false] {
        let mut app = selected_app();
        start(&mut app, "/project/a.txt");
        let _ = super::super::handle(
            &mut app,
            P::FileTreeDropTargetChanged(Some("/project/dst".into())),
        );
        if global_first {
            let _ = crate::app::message::view::update(&mut app, ViewMessage::GlobalMouseReleased);
            let _ = super::super::handle(&mut app, P::FileTreeDragEnd);
        } else {
            let _ = super::super::handle(&mut app, P::FileTreeDragEnd);
            let _ = crate::app::message::view::update(&mut app, ViewMessage::GlobalMouseReleased);
        }
        assert!(app.file_tree_operation_in_progress);
        assert!(app.pending_drop_file_paths.is_empty());
        assert!(app.dragging_file_paths.is_empty());
        assert!(app.file_tree_drag_source.is_none());
        assert_eq!(
            submit(&mut app, "/project".into(), "/project/a.txt".into(), "/project/dst".into())
                .units(),
            0
        );
    }
    let mut app = selected_app();
    start(&mut app, "/project/a.txt");
    let _ = super::super::handle(&mut app, P::FileTreeDragCancel);
    assert!(app.dragging_file_paths.is_empty());
    assert!(app.pending_drop_file_paths.is_empty());
    assert!(app.file_tree_drag_source.is_none());
    assert_eq!(app.file_tree_selection.paths.len(), 2);
}

#[test]
fn selected_drag_rejects_target_inside_any_selected_directory() {
    let mut app = selected_app();
    start(&mut app, "/project/a.txt");
    app.dragging_file_paths.push("/project/dir".into());
    let task =
        submit(&mut app, "/project".into(), "/project/a.txt".into(), "/project/dir/nested".into());
    assert_eq!(task.units(), 0);
    assert!(!app.file_tree_operation_in_progress);
    assert!(app.dragging_file_paths.is_empty());
    assert!(app.pending_drop_file_paths.is_empty());
}

#[test]
fn selected_drag_partial_completion_preserves_dirty_tabs_and_failed_selection() {
    let mut app = selected_app();
    let mut tab = super::super::file_tree_tests::preview_tab("/project/a.txt");
    tab.content = "unsaved".into();
    tab.is_dirty = true;
    app.preview_tabs.push(tab);
    app.active_preview_path = Some("/project/a.txt".into());
    let _ = complete_batch(
        &mut app,
        "/project".into(),
        vec![("/project/a.txt".into(), "/project/dst/a.txt".into())],
        vec!["b.txt: denied".into()],
    );
    assert_eq!(app.preview_tabs[0].path, "/project/dst/a.txt");
    assert_eq!(app.preview_tabs[0].content, "unsaved");
    assert!(app.preview_tabs[0].is_dirty);
    assert_eq!(app.active_preview_path.as_deref(), Some("/project/dst/a.txt"));
    assert!(app.file_tree_selection.contains("/project", "/project/dst/a.txt"));
    assert!(app.file_tree_selection.contains("/project", "/project/b.txt"));
    assert!(!app.file_tree_selection.contains("/project", "/project/a.txt"));
    assert!(app.active_toast.as_ref().unwrap().message.contains("denied"));
}
