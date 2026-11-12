// 验证文件树路径识别、名称去重及路径变更传播。
// 测试夹具围绕目标分支构造可控状态；跨模块行为由对应集成测试负责。

// 编译接线：引用父模块符号，确保测试片段仍由真实模块包含。
// 该冒烟用例只承担接线职责，具体行为由同领域测试覆盖。
#[test]
fn test_module_is_wired() {
    let module = module_path!();

    assert!(module.ends_with("file_tree_tests"));
}

fn app() -> crate::app::App {
    crate::app::App::new().0
}

pub(super) fn preview_tab(path: &str) -> crate::app::PreviewTab {
    crate::app::PreviewTab {
        path: path.to_string(),
        title: path.to_string(),
        content: String::new(),
        is_dirty: false,
        truncated: false,
        read_only: false,
        auto_save_revision: 0,
        editor: crate::app::components::editor::Editor::new("", "txt"),
        scroll_id: iced::widget::Id::unique(),
        #[cfg(not(target_arch = "wasm32"))]
        lsp_server_key: None,
        #[cfg(not(target_arch = "wasm32"))]
        lsp_uri: None,
        #[cfg(not(target_arch = "wasm32"))]
        lsp_language_id: None,
    }
}

#[test]
fn directory_detection_distinguishes_files_directories_and_unknown_paths() {
    let files = vec!["/project/src/main.rs".to_string(), "/project/README.md".to_string()];

    assert!(super::is_directory_path("/project/src", &files));
    assert!(!super::is_directory_path("/project/README.md", &files));
    assert!(super::is_directory_path("/project/empty", &files));
}

#[test]
fn unique_name_preserves_free_name_and_suffixes_collisions() {
    let directory = std::path::Path::new("/project/src");
    let mut existing = std::collections::HashSet::new();

    assert_eq!(
        super::unique_name_for_target(directory, "main.rs", &existing),
        "/project/src/main.rs"
    );

    existing.insert("/project/src/main.rs".to_string());
    existing.insert("/project/src/main copy.rs".to_string());
    assert_eq!(
        super::unique_name_for_target(directory, "main.rs", &existing),
        "/project/src/main copy 2.rs"
    );

    existing.insert("/project/src/LICENSE".to_string());
    assert_eq!(
        super::unique_name_for_target(directory, "LICENSE", &existing),
        "/project/src/LICENSE copy"
    );
}

#[test]
fn unique_name_falls_back_to_original_after_suffix_space_is_exhausted() {
    let directory = std::path::Path::new("/project/src");
    let mut existing = std::collections::HashSet::from([
        "/project/src/main.rs".to_string(),
        "/project/src/main copy.rs".to_string(),
    ]);
    for suffix in 2..=999 {
        existing.insert(format!("/project/src/main copy {suffix}.rs"));
    }

    assert_eq!(
        super::unique_name_for_target(directory, "main.rs", &existing),
        "/project/src/main.rs"
    );
}

#[test]
fn path_prefix_replacement_updates_exact_and_descendant_paths_only() {
    assert_eq!(
        super::replace_path_prefix("/project/old", "/project/old", "/project/new").as_deref(),
        Some("/project/new")
    );
    assert_eq!(
        super::replace_path_prefix("/project/old/nested/file.rs", "/project/old", "/project/new",)
            .as_deref(),
        Some("/project/new/nested/file.rs")
    );
    assert_eq!(
        super::replace_path_prefix("/project/older/file.rs", "/project/old", "/project/new"),
        None
    );
}

#[test]
fn path_display_name_uses_final_component_or_original_fallback() {
    assert_eq!(super::path_display_name("/project/src/main.rs"), "main.rs");
    assert_eq!(super::path_display_name(""), "");
    assert_eq!(super::path_display_name("/"), "/");
}

#[test]
fn file_tree_right_click_and_close_manage_menu_state() {
    let mut app = app();

    let _ = super::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::FileTreeRightClicked(
            "/tmp/a.txt".to_string(),
            "tree".to_string(),
            10.0,
            20.0,
        ),
    );

    assert_eq!(app.file_tree_menu_path.as_deref(), Some("/tmp/a.txt"));
    assert_eq!(app.file_tree_menu_source.as_deref(), Some("tree"));
    assert_eq!(app.file_tree_menu_anchor, Some(iced::Point::new(10.0, 20.0)));

    let _ =
        super::handle(&mut app, crate::app::message::project::ProjectMessage::FileTreeMenuClose);

    assert!(app.file_tree_menu_path.is_none());
    assert!(app.file_tree_menu_source.is_none());
    assert!(app.file_tree_menu_anchor.is_none());
}

#[test]
fn rename_changed_and_cancel_manage_rename_state() {
    let mut app = app();
    app.file_tree_rename_path = Some("/tmp/a.txt".to_string());

    let _ = super::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::FileTreeRenameChanged("b.txt".to_string()),
    );
    assert_eq!(app.file_tree_rename_value, "b.txt");

    let _ =
        super::handle(&mut app, crate::app::message::project::ProjectMessage::FileTreeRenameCancel);

    assert!(app.file_tree_rename_path.is_none());
    assert!(app.file_tree_rename_value.is_empty());
}

#[test]
fn drag_start_and_end_transfer_pending_drop_state() {
    let mut app = app();

    let _ = super::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::FileTreeDragStart(
            "/tmp/a.txt".to_string(),
            Some((3, 4)),
        ),
    );
    assert_eq!(app.dragging_file_paths, vec!["/tmp/a.txt".to_string()]);
    assert_eq!(app.dragging_file_position, Some((3, 4)));

    let _ = super::handle(&mut app, crate::app::message::project::ProjectMessage::FileTreeDragEnd);

    assert!(app.dragging_file_paths.is_empty());
    assert_eq!(app.pending_drop_file_paths, vec!["/tmp/a.txt".to_string()]);
    assert_eq!(app.pending_drop_file_position, Some((3, 4)));
}

// 失败与边界：覆盖非法、缺失、过期或冲突场景的既定处理规则。
// 重点确认现有状态与错误反馈不会被意外的部分更新污染。
#[test]
fn rename_save_rejects_each_invalid_name_shape() {
    for invalid_name in ["", "   ", ".", "..", "a/b", "a\\b"] {
        let mut app = app();
        app.file_tree_rename_path = Some("/tmp/a.txt".to_string());
        app.file_tree_rename_value = invalid_name.to_string();

        let _ = super::handle(
            &mut app,
            crate::app::message::project::ProjectMessage::FileTreeRenameSave,
        );

        let toast = app.active_toast.as_ref().expect("invalid name feedback");
        assert!(matches!(toast.kind, crate::app::state::ToastKind::Error));
        assert_eq!(toast.message, "文件名不合法");
        assert_eq!(app.file_tree_rename_path.as_deref(), Some("/tmp/a.txt"));
        assert_eq!(app.file_tree_rename_value, invalid_name);
    }
}

#[test]
fn rename_save_without_target_or_project_is_safely_ignored() {
    let mut app = app();
    app.file_tree_rename_value = "new.txt".to_string();
    let _ =
        super::handle(&mut app, crate::app::message::project::ProjectMessage::FileTreeRenameSave);
    assert!(app.file_tree_rename_path.is_none());

    app.file_tree_rename_path = Some("/tmp/a.txt".to_string());
    let _ =
        super::handle(&mut app, crate::app::message::project::ProjectMessage::FileTreeRenameSave);
    assert_eq!(app.file_tree_rename_path.as_deref(), Some("/tmp/a.txt"));
    assert_eq!(app.file_tree_rename_value, "new.txt");
}

#[test]
fn rename_save_with_project_queues_operation_and_clears_editor_state() {
    let mut app = app();
    app.project_path = Some("/tmp/project".to_string());
    app.file_tree_rename_path = Some("/tmp/project/a.txt".to_string());
    app.file_tree_rename_value = " b.txt ".to_string();

    let task =
        super::handle(&mut app, crate::app::message::project::ProjectMessage::FileTreeRenameSave);

    assert!(task.is_some());
    assert!(app.file_tree_rename_path.is_none());
    assert!(app.file_tree_rename_value.is_empty());
}

#[test]
fn rename_unchanged_name_closes_editor_without_moving_file() {
    let mut app = app();
    app.project_path = Some("/tmp/project".into());
    app.files = vec!["/tmp/project/ACP验收.md".into()];
    app.file_tree_rename_path = Some(app.files[0].clone());
    app.file_tree_rename_value = "ACP验收.md".into();

    let task =
        super::handle(&mut app, crate::app::message::project::ProjectMessage::FileTreeRenameSave)
            .expect("handled rename");
    assert_eq!(task.units(), 0);
    assert!(app.file_tree_rename_path.is_none());
    assert!(app.file_tree_rename_value.is_empty());
}

#[test]
fn rename_completed_updates_preview_paths_on_success() {
    let mut app = app();
    app.project_path = Some("/tmp/project".to_string());
    app.preview_tabs.push(preview_tab("/tmp/project/src/old.rs"));
    app.preview_tabs.push(preview_tab("/tmp/project/src/old.rs/nested.txt"));
    app.active_preview_path = Some("/tmp/project/src/old.rs".to_string());

    let _ = super::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::FileTreeRenameCompleted {
            old_path: "/tmp/project/src/old.rs".to_string(),
            result: Ok("/tmp/project/src/new.rs".to_string()),
        },
    );

    assert_eq!(app.preview_tabs[0].path, "/tmp/project/src/new.rs");
    assert_eq!(app.preview_tabs[1].path, "/tmp/project/src/new.rs/nested.txt");
    assert_eq!(app.active_preview_path.as_deref(), Some("/tmp/project/src/new.rs"));
}

#[test]
fn rename_completed_failure_preserves_paths_and_reports_error() {
    let mut app = app();
    app.project_path = Some("/tmp/project".to_string());
    app.preview_tabs.push(preview_tab("/tmp/project/src/old.rs"));
    app.active_preview_path = Some("/tmp/project/src/old.rs".to_string());

    let _ = super::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::FileTreeRenameCompleted {
            old_path: "/tmp/project/src/old.rs".to_string(),
            result: Err("denied".to_string()),
        },
    );

    assert_eq!(app.preview_tabs[0].path, "/tmp/project/src/old.rs");
    assert_eq!(app.active_preview_path.as_deref(), Some("/tmp/project/src/old.rs"));
    assert_eq!(app.error_message.as_deref(), Some("重命名失败: denied"));
}

#[test]
fn file_tree_action_without_menu_path_is_ignored() {
    let mut app = app();
    app.file_tree_menu_path = None;

    let _ = super::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::FileTreeAction(
            crate::app::message::project::FileTreeAction::Rename,
        ),
    );

    assert!(app.file_tree_rename_path.is_none());
}

#[test]
fn delete_action_keeps_file_until_confirmed() {
    let mut app = app();
    let target = "/tmp/project/src/main.rs";
    app.project_path = Some("/tmp/project".to_string());
    app.file_tree_menu_path = Some(target.to_string());
    app.files = vec![target.to_string(), "/tmp/project/src/lib.rs".to_string()];

    let task = super::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::FileTreeAction(
            crate::app::message::project::FileTreeAction::Delete,
        ),
    )
    .unwrap();
    assert_eq!(task.units(), 0);
    assert!(app.file_tree_menu_path.is_none());
    assert_eq!(app.files, [target, "/tmp/project/src/lib.rs"]);

    let task = super::handle(&mut app, super::ProjectMessage::FileTreeDeleteConfirmed).unwrap();
    assert_eq!(task.units(), 1);
    assert!(app.file_tree_pending_delete.is_none());
    assert_eq!(app.files, vec!["/tmp/project/src/lib.rs".to_string()]);
}

#[test]
fn delete_confirmation_removes_directory_subtree_before_gateway_completes() {
    let mut app = app();
    let target = "/tmp/project/src";
    app.project_path = Some("/tmp/project".to_string());
    app.file_tree_menu_path = Some(target.to_string());
    app.files = vec![
        target.to_string(),
        "/tmp/project/src/main.rs".to_string(),
        "/tmp/project/src/nested/lib.rs".to_string(),
        "/tmp/project/src-old/keep.rs".to_string(),
        "/tmp/project/README.md".to_string(),
    ];

    let _ = super::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::FileTreeAction(
            crate::app::message::project::FileTreeAction::Delete,
        ),
    );

    assert_eq!(app.files.len(), 5);
    let _ = super::handle(&mut app, super::ProjectMessage::FileTreeDeleteConfirmed);
    assert_eq!(
        app.files,
        vec!["/tmp/project/src-old/keep.rs".to_string(), "/tmp/project/README.md".to_string(),]
    );
}

#[test]
fn delete_confirmation_keeps_file_after_cancel_or_project_change() {
    use super::{FileTreeAction, ProjectMessage};

    let mut app = app();
    let project = "/tmp/project";
    let target = "/tmp/project/src/main.rs";
    app.files = vec![target.to_string()];

    for cancel in [true, false] {
        app.project_path = Some(project.to_string());
        app.file_tree_menu_path = Some(target.to_string());
        let _ = super::handle(&mut app, ProjectMessage::FileTreeAction(FileTreeAction::Delete));
        assert!(app.file_tree_pending_delete.is_some());

        if cancel {
            let _ = super::handle(&mut app, ProjectMessage::FileTreeDeleteCanceled);
        } else {
            app.project_path = Some("/tmp/other".to_string());
        }

        let task = super::handle(&mut app, ProjectMessage::FileTreeDeleteConfirmed).unwrap();
        assert_eq!(task.units(), 0);
        assert!(app.file_tree_pending_delete.is_none());
        assert_eq!(app.files, [target]);
    }
}

#[test]
fn copy_and_cut_actions_update_file_tree_clipboard() {
    let mut app = app();
    app.file_tree_menu_path = Some("/tmp/a.txt".to_string());
    let _ = super::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::FileTreeAction(
            crate::app::message::project::FileTreeAction::Copy,
        ),
    );
    let clipboard = app.file_tree_clipboard.as_ref().expect("clipboard");
    assert!(matches!(clipboard.mode, crate::app::state::FileTreeClipboardMode::Copy));
    assert_eq!(clipboard.src_paths, ["/tmp/a.txt"]);

    app.file_tree_menu_path = Some("/tmp/b.txt".to_string());
    let _ = super::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::FileTreeAction(
            crate::app::message::project::FileTreeAction::Cut,
        ),
    );
    let clipboard = app.file_tree_clipboard.as_ref().expect("clipboard");
    assert!(matches!(clipboard.mode, crate::app::state::FileTreeClipboardMode::Cut));
    assert_eq!(clipboard.src_paths, ["/tmp/b.txt"]);
}

// 状态转换：覆盖交互前后的字段同步、选择关系与清理规则。
// 连续操作仍需保持模块不变量，并清除已经失效的瞬态反馈。
#[test]
fn find_in_folder_action_creates_active_find_tab_and_open_tab() {
    let mut app = app();
    app.project_path = Some("/tmp/project".to_string());
    app.file_tree_menu_path = Some("/tmp/project/src".to_string());

    let _ = super::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::FileTreeAction(
            crate::app::message::project::FileTreeAction::FindInFolder,
        ),
    );

    assert_eq!(app.find_results_tabs.len(), 1);
    assert_eq!(app.find_results_tabs[0].scope_path, "/tmp/project/src");
    assert_eq!(app.active_find_results_tab_id, Some(app.find_results_tabs[0].id.clone()));
    assert!(app.show_file_manager);
    assert!(app.active_tab_id.as_deref().is_some_and(|id| id.starts_with("find:")));
}

#[test]
fn rename_action_primes_rename_fields() {
    let mut app = app();
    app.file_tree_menu_path = Some("/tmp/project/src/main.rs".to_string());

    let _ = super::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::FileTreeAction(
            crate::app::message::project::FileTreeAction::Rename,
        ),
    );

    assert_eq!(app.file_tree_rename_path.as_deref(), Some("/tmp/project/src/main.rs"));
    assert_eq!(app.file_tree_rename_value, "main.rs");
}

#[test]
fn open_and_copy_path_actions_clear_menu_and_return_tasks() {
    let mut app = app();
    app.file_tree_menu_path = Some("/tmp/project/src/main.rs".to_string());
    app.file_tree_menu_source = Some("tree".to_string());
    app.file_tree_menu_anchor = Some(iced::Point::new(1.0, 2.0));

    let open_task = super::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::FileTreeAction(
            crate::app::message::project::FileTreeAction::Open,
        ),
    );
    assert!(open_task.is_some());
    assert!(app.file_tree_menu_path.is_none());
    assert!(app.file_tree_menu_source.is_none());
    assert!(app.file_tree_menu_anchor.is_none());

    app.file_tree_menu_path = Some("/tmp/project/src/main.rs".to_string());
    let copy_task = super::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::FileTreeAction(
            crate::app::message::project::FileTreeAction::CopyPath,
        ),
    );
    assert!(copy_task.is_some());
    assert!(app.file_tree_menu_path.is_none());
}

#[test]
fn relative_copy_delete_and_add_to_chat_handle_missing_project_root() {
    let mut app = app();

    app.file_tree_menu_path = Some("/tmp/project/src/main.rs".to_string());
    let relative_copy = super::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::FileTreeAction(
            crate::app::message::project::FileTreeAction::CopyRelativePath,
        ),
    );
    assert!(relative_copy.is_some());
    assert!(app.file_tree_menu_path.is_none());

    app.file_tree_menu_path = Some("/tmp/project/src/main.rs".to_string());
    let delete = super::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::FileTreeAction(
            crate::app::message::project::FileTreeAction::Delete,
        ),
    );
    assert!(delete.is_some());
    assert!(app.error_message.is_none());

    app.file_tree_menu_path = Some("/tmp/project/src/main.rs".to_string());
    let add_to_chat = super::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::FileTreeAction(
            crate::app::message::project::FileTreeAction::AddToChat,
        ),
    );
    assert!(add_to_chat.is_some());
    assert!(!app.show_file_search);
}

#[test]
fn paste_without_clipboard_or_project_does_not_mutate_clipboard() {
    let mut app = app();
    app.file_tree_menu_path = Some("/tmp/project/src".to_string());
    let _ = super::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::FileTreeAction(
            crate::app::message::project::FileTreeAction::Paste,
        ),
    );
    assert!(app.file_tree_clipboard.is_none());

    app.file_tree_menu_path = Some("/tmp/project/src".to_string());
    app.file_tree_clipboard = Some(crate::app::state::FileTreeClipboard {
        mode: crate::app::state::FileTreeClipboardMode::Copy,
        src_paths: vec!["/tmp/source.txt".to_string()],
    });
    let _ = super::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::FileTreeAction(
            crate::app::message::project::FileTreeAction::Paste,
        ),
    );
    assert_eq!(
        app.file_tree_clipboard
            .as_ref()
            .and_then(|value| value.src_paths.first().map(String::as_str)),
        Some("/tmp/source.txt")
    );
}

#[test]
fn unrelated_project_message_is_not_handled() {
    let mut app = app();

    let task =
        super::handle(&mut app, crate::app::message::project::ProjectMessage::RecentOverlayClosed);

    assert!(task.is_none());
}

#[test]
fn paste_completed_cut_success_clears_clipboard() {
    let mut app = app();
    app.file_tree_clipboard = Some(crate::app::state::FileTreeClipboard {
        mode: crate::app::state::FileTreeClipboardMode::Cut,
        src_paths: vec!["/tmp/a.txt".to_string()],
    });

    let clipboard = app.file_tree_clipboard.clone().unwrap();
    let _ = super::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::FileTreePasteCompleted {
            project_path: "/tmp".into(),
            clipboard,
            completed_sources: vec!["/tmp/a.txt".into()],
            result: Ok(()),
        },
    );

    assert!(app.file_tree_clipboard.is_none());
}

#[test]
fn paste_completed_copy_success_preserves_clipboard() {
    let mut app = app();
    app.file_tree_clipboard = Some(crate::app::state::FileTreeClipboard {
        mode: crate::app::state::FileTreeClipboardMode::Copy,
        src_paths: vec!["/tmp/a.txt".to_string()],
    });

    let _ = super::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::FileTreePasteCompleted {
            project_path: "/tmp".into(),
            clipboard: crate::app::state::FileTreeClipboard {
                mode: crate::app::state::FileTreeClipboardMode::Copy,
                src_paths: vec!["/tmp/a.txt".into()],
            },
            completed_sources: vec![],
            result: Ok(()),
        },
    );

    assert_eq!(
        app.file_tree_clipboard
            .as_ref()
            .and_then(|value| value.src_paths.first().map(String::as_str)),
        Some("/tmp/a.txt")
    );
}

// 失败与边界：覆盖非法、缺失、过期或冲突场景的既定处理规则。
// 重点确认现有状态与错误反馈不会被意外的部分更新污染。
#[test]
fn paste_and_delete_failures_show_error_toasts() {
    let mut app = app();

    let _ = super::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::FileTreePasteCompleted {
            project_path: "/tmp".into(),
            clipboard: crate::app::state::FileTreeClipboard {
                mode: crate::app::state::FileTreeClipboardMode::Copy,
                src_paths: vec!["/tmp/a.txt".into()],
            },
            completed_sources: vec![],
            result: Err("denied".to_string()),
        },
    );
    assert!(app.active_toast.as_ref().is_some_and(|toast| toast.message.contains("denied")));

    let _ = super::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::FileTreeDeleteCompleted {
            project_path: "/tmp".into(),
            deleted_paths: vec![],
            result: Err("missing".into()),
        },
    );
    assert!(app.active_toast.as_ref().is_some_and(|toast| toast.message.contains("missing")));
}

#[test]
fn file_tree_move_drop_release_consumes_tree_source_once() {
    use crate::app::message::ProjectMessage as P;
    let mut app = app();
    app.project_path = Some("/project".into());
    let _ = super::handle(
        &mut app,
        P::FileTreeInternalDragStart {
            project_path: "/project".into(),
            source_path: "/project/a.txt".into(),
        },
    );
    let _ = super::handle(&mut app, P::FileTreeDropTargetChanged(Some("/project/dst".into())));
    let _ = super::handle(&mut app, P::FileTreeDragEnd);
    assert!(app.file_tree_operation_in_progress, "tree release must submit a move");
    assert!(app.file_tree_drag_source.is_none());
    assert!(app.pending_drop_file_paths.is_empty());
    let _ = super::handle(&mut app, P::FileTreeDragEnd);
    assert!(app.pending_drop_file_paths.is_empty());
}

#[test]
fn file_tree_move_drop_completion_preserves_dirty_content_and_directory_boundaries() {
    use crate::app::message::ProjectMessage as P;
    let mut app = app();
    app.project_path = Some("/project".into());
    app.lsp_disabled = true;
    let mut tab = preview_tab("/project/src/nested/a.txt");
    tab.content = "unsaved".into();
    tab.editor = crate::app::components::editor::Editor::new("unsaved", "txt");
    tab.is_dirty = true;
    app.preview_tabs = vec![tab, preview_tab("/project/src-other/a.txt")];
    app.active_preview_path = Some("/project/src/nested/a.txt".into());
    app.file_tree_expanded = vec!["src".into(), "src/nested".into(), "src-other".into()];
    app.file_tree_operation_in_progress = true;
    let _ = super::handle(
        &mut app,
        P::FileTreeMoveCompleted {
            project_path: "/project".into(),
            source_path: "/project/src".into(),
            target_path: "/project/dst/src".into(),
            result: Ok(()),
        },
    );
    assert_eq!(app.preview_tabs[0].path, "/project/dst/src/nested/a.txt");
    assert_eq!(app.preview_tabs[0].content, "unsaved");
    assert_eq!(app.preview_tabs[0].editor.content(), "unsaved");
    assert!(app.preview_tabs[0].is_dirty);
    assert_eq!(app.preview_tabs[1].path, "/project/src-other/a.txt");
    assert_eq!(app.active_preview_path.as_deref(), Some("/project/dst/src/nested/a.txt"));
    assert_eq!(app.file_tree_expanded, ["dst/src", "dst/src/nested", "src-other"]);
    assert!(app.file_tree_expanded_set.contains("dst/src/nested"));
}

#[test]
fn file_tree_move_drop_failure_and_old_project_leave_current_editor_and_drag_untouched() {
    use crate::app::message::ProjectMessage as P;
    for result in [Ok(()), Err("destination exists".into())] {
        let mut app = app();
        app.project_path = Some("/other".into());
        let mut tab = preview_tab("/other/edit.txt");
        tab.content = "unsaved".into();
        tab.editor = crate::app::components::editor::Editor::new("unsaved", "txt");
        tab.is_dirty = true;
        app.preview_tabs.push(tab);
        app.file_tree_drag_source = Some(("/other".into(), "/other/edit.txt".into()));
        app.project_preview_tabs.insert("/project".into(), vec![preview_tab("/project/a.txt")]);
        let success = result.is_ok();
        let _ = super::handle(
            &mut app,
            P::FileTreeMoveCompleted {
                project_path: "/project".into(),
                source_path: "/project/a.txt".into(),
                target_path: "/project/dst/a.txt".into(),
                result,
            },
        );
        assert_eq!(app.preview_tabs[0].path, "/other/edit.txt");
        assert_eq!(app.preview_tabs[0].content, "unsaved");
        assert_eq!(app.preview_tabs[0].editor.content(), "unsaved");
        assert!(app.preview_tabs[0].is_dirty);
        assert!(app.file_tree_drag_source.is_some());
        assert_eq!(
            app.project_preview_tabs["/project"][0].path,
            if success { "/project/dst/a.txt" } else { "/project/a.txt" }
        );
    }
}

#[test]
fn file_tree_move_drop_rejects_invalid_targets_and_consumes_release() {
    use crate::app::message::ProjectMessage as P;
    for target in [
        "/project/src",
        "/project/src/nested",
        "/project",
        "/outside",
        "/project/../outside",
        "/project/file.txt",
    ] {
        let mut app = app();
        app.project_path = Some("/project".into());
        app.files = vec!["/project/file.txt".into()];
        let _ = super::handle(
            &mut app,
            P::FileTreeInternalDragStart {
                project_path: "/project".into(),
                source_path: "/project/src".into(),
            },
        );
        let _ = super::handle(
            &mut app,
            P::FileTreeDropRequested {
                project_path: "/project".into(),
                source_path: "/project/src".into(),
                target_directory: target.into(),
            },
        );
        assert!(!app.file_tree_operation_in_progress, "{target}");
        assert!(app.file_tree_drag_source.is_none());
        assert!(app.pending_drop_file_paths.is_empty());
    }
}

#[path = "file_tree_move_drop_http_tests.rs"]
mod http_tests;

#[test]
fn file_tree_move_drop_unicode_conflict_preserves_unsaved_editor_and_active_path() {
    use crate::app::message::ProjectMessage as P;
    let mut app = app();
    app.project_path = Some("/project".into());
    app.lsp_disabled = true;
    let mut tab = preview_tab("/project/中文/草稿.txt");
    tab.content = "未保存的内容".into();
    tab.editor = crate::app::components::editor::Editor::new("未保存的内容", "txt");
    tab.is_dirty = true;
    app.preview_tabs.push(tab);
    app.active_preview_path = Some("/project/中文/草稿.txt".into());
    app.file_tree_operation_in_progress = true;
    let _ = super::handle(
        &mut app,
        P::FileTreeMoveCompleted {
            project_path: "/project".into(),
            source_path: "/project/中文".into(),
            target_path: "/project/目标/中文".into(),
            result: Err("destination exists".into()),
        },
    );
    assert_eq!(app.preview_tabs[0].path, "/project/中文/草稿.txt");
    assert_eq!(app.preview_tabs[0].content, "未保存的内容");
    assert_eq!(app.preview_tabs[0].editor.content(), "未保存的内容");
    assert!(app.preview_tabs[0].is_dirty);
    assert_eq!(app.active_preview_path.as_deref(), Some("/project/中文/草稿.txt"));
    assert!(!app.file_tree_operation_in_progress);
}

#[test]
fn file_tree_move_drop_target_release_uses_current_source_and_is_consumed_once() {
    use crate::app::message::ProjectMessage as P;
    let mut app = app();
    app.project_path = Some("/project".into());
    // The target message is constructed before the source gesture begins.
    let release = P::FileTreeDropTargetReleased {
        project_path: "/project".into(),
        target_directory: "/project/dst".into(),
    };
    let _ = super::handle(&mut app, release.clone());
    assert!(!app.file_tree_operation_in_progress);
    let _ = super::handle(
        &mut app,
        P::FileTreeInternalDragStart {
            project_path: "/project".into(),
            source_path: "/project/a.txt".into(),
        },
    );
    let _ = super::handle(&mut app, release.clone());
    assert!(app.file_tree_operation_in_progress);
    assert!(app.file_tree_drag_source.is_none());
    let _ = super::handle(&mut app, release);
    let _ = super::handle(&mut app, P::FileTreeDragEnd);
    assert!(app.pending_drop_file_paths.is_empty());
}

#[test]
fn modified_tree_clicks_select_files_and_directories_without_opening_them() {
    use crate::app::message::project::ProjectMessage;
    use iced::keyboard::Modifiers;
    let mut app = app();
    app.project_path = Some("/project".to_string());
    app.set_file_index("/project", vec!["/project/src/a.rs".into(), "/project/README.md".into()]);
    app.file_tree_expanded.clear();
    app.file_tree_expanded_set.clear();
    for (path, directory) in [("/project/src", true), ("/project/README.md", false)] {
        let _ = super::handle(
            &mut app,
            ProjectMessage::FileTreeSelected {
                project_path: "/project".into(),
                path: path.into(),
                directory,
                modifiers: Modifiers::COMMAND,
            },
        );
    }
    assert!(app.file_tree_selection.contains("/project", "/project/src"));
    assert!(app.file_tree_selection.contains("/project", "/project/README.md"));
    assert!(!app.is_file_tree_dir_expanded("src"));
    assert!(app.active_preview_path.is_none());
    let _ = super::handle(
        &mut app,
        ProjectMessage::FileTreeSelected {
            project_path: "/project".into(),
            path: "/project/src".into(),
            directory: true,
            modifiers: Modifiers::default(),
        },
    );
    assert!(app.is_file_tree_dir_expanded("src"));
    assert_eq!(app.file_tree_selection.paths.len(), 1);
}

#[test]
fn tree_selection_rejects_old_project_and_invisible_rows() {
    use crate::app::message::project::ProjectMessage;
    let mut app = app();
    app.project_path = Some("/project".into());
    app.set_file_index("/project", vec!["/project/src/a.rs".into()]);
    app.file_tree_expanded.clear();
    app.file_tree_expanded_set.clear();
    for (root, path) in [("/old", "/old/a"), ("/project", "/project/src/a.rs")] {
        let _ = super::handle(
            &mut app,
            ProjectMessage::FileTreeSelected {
                project_path: root.into(),
                path: path.into(),
                directory: false,
                modifiers: iced::keyboard::Modifiers::COMMAND,
            },
        );
    }
    assert!(app.file_tree_selection.paths.is_empty());
}
