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

fn preview_tab(path: &str) -> crate::app::PreviewTab {
    crate::app::PreviewTab {
        path: path.to_string(),
        title: path.to_string(),
        content: String::new(),
        is_dirty: false,
        truncated: false,
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

        assert_eq!(app.error_message.as_deref(), Some("文件名不合法"));
        assert_eq!(app.file_tree_rename_path.as_deref(), Some("/tmp/a.txt"));
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
fn delete_action_removes_file_from_local_tree_before_gateway_completes() {
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
    );

    assert!(task.is_some());
    assert_eq!(app.files, vec!["/tmp/project/src/lib.rs".to_string()]);
}

#[test]
fn delete_action_removes_directory_subtree_from_local_tree_before_gateway_completes() {
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

    let task = super::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::FileTreeAction(
            crate::app::message::project::FileTreeAction::Delete,
        ),
    );

    assert!(task.is_some());
    assert_eq!(
        app.files,
        vec!["/tmp/project/src-old/keep.rs".to_string(), "/tmp/project/README.md".to_string(),]
    );
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
    assert_eq!(clipboard.src_path, "/tmp/a.txt");

    app.file_tree_menu_path = Some("/tmp/b.txt".to_string());
    let _ = super::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::FileTreeAction(
            crate::app::message::project::FileTreeAction::Cut,
        ),
    );
    let clipboard = app.file_tree_clipboard.as_ref().expect("clipboard");
    assert!(matches!(clipboard.mode, crate::app::state::FileTreeClipboardMode::Cut));
    assert_eq!(clipboard.src_path, "/tmp/b.txt");
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
        src_path: "/tmp/source.txt".to_string(),
    });
    let _ = super::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::FileTreeAction(
            crate::app::message::project::FileTreeAction::Paste,
        ),
    );
    assert_eq!(
        app.file_tree_clipboard.as_ref().map(|value| value.src_path.as_str()),
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
        src_path: "/tmp/a.txt".to_string(),
    });

    let _ = super::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::FileTreePasteCompleted {
            clear_clipboard: true,
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
        src_path: "/tmp/a.txt".to_string(),
    });

    let _ = super::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::FileTreePasteCompleted {
            clear_clipboard: false,
            result: Ok(()),
        },
    );

    assert_eq!(
        app.file_tree_clipboard.as_ref().map(|value| value.src_path.as_str()),
        Some("/tmp/a.txt")
    );
}

// 失败与边界：覆盖非法、缺失、过期或冲突场景的既定处理规则。
// 重点确认现有状态与错误反馈不会被意外的部分更新污染。
#[test]
fn paste_and_delete_failures_set_error_message() {
    let mut app = app();

    let _ = super::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::FileTreePasteCompleted {
            clear_clipboard: false,
            result: Err("denied".to_string()),
        },
    );
    assert_eq!(app.error_message.as_deref(), Some("粘贴失败: denied"));

    let _ = super::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::FileTreeDeleteCompleted(Err(
            "missing".to_string()
        )),
    );
    assert_eq!(app.error_message.as_deref(), Some("删除失败: missing"));
}
