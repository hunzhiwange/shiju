use super::*;
use crate::app::state::ToastKind;

fn app() -> App {
    App::new().0
}

fn preview_tab(path: &str, content: &str) -> PreviewTab {
    PreviewTab {
        path: path.to_string(),
        title: std::path::Path::new(path)
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or(path)
            .to_string(),
        content: content.to_string(),
        is_dirty: false,
        truncated: false,
        auto_save_revision: 0,
        editor: crate::app::components::editor::Editor::new(content, "txt"),
        scroll_id: iced::widget::Id::unique(),
        #[cfg(not(target_arch = "wasm32"))]
        lsp_server_key: None,
        #[cfg(not(target_arch = "wasm32"))]
        lsp_uri: None,
        #[cfg(not(target_arch = "wasm32"))]
        lsp_language_id: None,
    }
}

fn has_scheduled_action(task: Task<Message>) -> bool {
    task.units() > 0
}

async fn collect_task_messages(task: Task<Message>) -> Vec<Message> {
    use iced_runtime::futures::futures::StreamExt;

    let Some(mut stream) = iced_runtime::task::into_stream(task.collect()) else {
        return Vec::new();
    };
    while let Some(action) = stream.next().await {
        if let iced_runtime::Action::Output(messages) = action {
            return messages;
        }
    }
    Vec::new()
}

#[test]
fn gateway_target_uses_project_relative_path_for_file_inside_project() {
    let project = tempfile::tempdir().expect("temporary project should be created");
    let project_path = project.path().to_string_lossy().to_string();
    let file_path = project.path().join("src").join("main.rs").to_string_lossy().to_string();
    let target = gateway_preview_target(Some(&project_path), &file_path)
        .expect("path inside project should resolve");

    assert_eq!(target, (project_path, "src/main.rs".to_string()));
}

#[test]
fn gateway_target_joins_relative_path_to_project_and_normalizes_separators() {
    let target = gateway_preview_target(Some("/workspace/project"), "src\\main.rs")
        .expect("relative project path should resolve");

    assert_eq!(target, ("/workspace/project".to_string(), "src/main.rs".to_string()));
}

#[test]
fn gateway_target_falls_back_to_parent_for_absolute_path_outside_project() {
    let project = tempfile::tempdir().expect("temporary project should be created");
    let outside = tempfile::tempdir().expect("temporary outside directory should be created");
    let project_path = project.path().to_string_lossy().to_string();
    let file_path = outside.path().join("notes.txt").to_string_lossy().to_string();
    let target = gateway_preview_target(Some(&project_path), &file_path)
        .expect("absolute path should resolve from its parent");

    assert_eq!(target, (outside.path().to_string_lossy().to_string(), "notes.txt".to_string()));
}

#[test]
fn gateway_target_rejects_path_without_directory_or_file_name() {
    let no_directory = gateway_preview_target(None, "notes.txt")
        .expect_err("bare path without a project should be rejected");
    let temp = tempfile::tempdir().expect("temporary directory should be created");
    let path_without_name = temp.path().join("..").to_string_lossy().to_string();
    let no_file_name = gateway_preview_target(None, &path_without_name)
        .expect_err("path without a file name should be rejected");

    assert!(no_directory.contains("无法确定预览文件目录"));
    assert!(no_file_name.contains("无法确定预览文件名"));
}

#[test]
fn push_trace_entry_deduplicates_adjacent_entries() {
    let entry = ("/tmp/a.rs".to_string(), 3, 5);
    let mut entries = vec![entry.clone()];

    push_trace_entry(&mut entries, entry.clone());

    assert_eq!(entries, vec![entry]);
}

#[test]
fn push_trace_entry_keeps_only_latest_history_limit() {
    let mut entries = Vec::new();
    for line in 0..=TRACE_HISTORY_LIMIT {
        push_trace_entry(&mut entries, ("/tmp/a.rs".to_string(), line, 0));
    }

    assert_eq!(entries.len(), TRACE_HISTORY_LIMIT);
    assert_eq!(entries.first().map(|entry| entry.1), Some(1));
    assert_eq!(entries.last().map(|entry| entry.1), Some(TRACE_HISTORY_LIMIT));
}

#[test]
fn current_preview_location_requires_active_matching_tab() {
    let mut app = app();
    assert!(current_preview_location(&app).is_none());

    app.active_preview_path = Some("/tmp/missing.rs".to_string());
    assert!(current_preview_location(&app).is_none());

    app.preview_tabs.push(preview_tab("/tmp/missing.rs", "fn main() {}"));
    assert_eq!(current_preview_location(&app), Some(("/tmp/missing.rs".to_string(), 0, 0)));
}

#[test]
fn preview_is_dirty_reflects_tab_state() {
    let mut tab = preview_tab("/tmp/a.txt", "a");
    assert!(!preview_is_dirty(&tab));

    tab.is_dirty = true;
    assert!(preview_is_dirty(&tab));
}

#[test]
fn record_trace_before_jump_records_current_and_clears_forward_history() {
    let mut app = app();
    app.preview_tabs.push(preview_tab("/tmp/current.rs", "current"));
    app.active_preview_path = Some("/tmp/current.rs".to_string());
    app.preview_trace_forward.push(("/tmp/future.rs".to_string(), 8, 2));

    record_trace_before_jump(&mut app, &("/tmp/target.rs".to_string(), 4, 1));

    assert_eq!(app.preview_trace_back, vec![("/tmp/current.rs".to_string(), 0, 0)]);
    assert!(app.preview_trace_forward.is_empty());
}

#[test]
fn record_trace_before_jump_ignores_navigation_and_same_location() {
    let mut app = app();
    app.preview_tabs.push(preview_tab("/tmp/current.rs", "current"));
    app.active_preview_path = Some("/tmp/current.rs".to_string());
    let current = ("/tmp/current.rs".to_string(), 0, 0);

    record_trace_before_jump(&mut app, &current);
    assert!(app.preview_trace_back.is_empty());

    app.preview_trace_navigating = true;
    record_trace_before_jump(&mut app, &("/tmp/other.rs".to_string(), 1, 1));
    assert!(app.preview_trace_back.is_empty());
}

#[test]
fn navigate_trace_back_skips_current_and_prepares_previous_target() {
    let mut app = app();
    app.preview_tabs.push(preview_tab("/tmp/current.rs", "current"));
    app.active_preview_path = Some("/tmp/current.rs".to_string());
    app.preview_trace_back =
        vec![("/tmp/previous.rs".to_string(), 7, 3), ("/tmp/current.rs".to_string(), 0, 0)];

    let task = navigate_trace_back(&mut app);

    assert_eq!(app.pending_preview_goto, Some(("/tmp/previous.rs".to_string(), 7, 3)));
    assert_eq!(app.preview_trace_forward, vec![("/tmp/current.rs".to_string(), 0, 0)]);
    assert!(app.preview_trace_navigating);
    assert!(has_scheduled_action(task));
}

#[test]
fn navigate_trace_forward_prepares_next_target_and_records_current() {
    let mut app = app();
    app.preview_tabs.push(preview_tab("/tmp/current.rs", "current"));
    app.active_preview_path = Some("/tmp/current.rs".to_string());
    app.preview_trace_forward = vec![("/tmp/next.rs".to_string(), 9, 4)];

    let task = navigate_trace_forward(&mut app);

    assert_eq!(app.pending_preview_goto, Some(("/tmp/next.rs".to_string(), 9, 4)));
    assert_eq!(app.preview_trace_back, vec![("/tmp/current.rs".to_string(), 0, 0)]);
    assert!(app.preview_trace_navigating);
    assert!(has_scheduled_action(task));
}

#[test]
fn navigate_trace_with_empty_history_stays_idle() {
    let mut app = app();

    assert!(!has_scheduled_action(navigate_trace_back(&mut app)));
    assert!(!has_scheduled_action(navigate_trace_forward(&mut app)));
    assert!(!app.preview_trace_navigating);
    assert!(app.pending_preview_goto.is_none());
}

#[test]
fn navigate_trace_without_current_location_opens_target_without_recording_opposite_history() {
    let mut app = app();
    app.preview_trace_back = vec![("/tmp/previous.rs".to_string(), 6, 2)];

    let task = navigate_trace_back(&mut app);

    assert_eq!(app.pending_preview_goto, Some(("/tmp/previous.rs".to_string(), 6, 2)));
    assert!(app.preview_trace_forward.is_empty());
    assert!(app.preview_trace_navigating);
    assert!(has_scheduled_action(task));
}

#[test]
fn open_existing_tab_focuses_preview_and_preserves_single_tab() {
    let mut app = app();
    app.workspace_mode = crate::app::state::WorkspaceMode::Code;
    app.preview_tabs.push(preview_tab("/tmp/a.txt", "a"));
    app.show_file_manager = true;
    app.file_manager_show_changes = false;
    app.screen = Screen::Project;

    let task = update(&mut app, PreviewMessage::Open("/tmp/a.txt".to_string()));

    assert_eq!(app.preview_tabs.len(), 1);
    assert_eq!(app.active_preview_path.as_deref(), Some("/tmp/a.txt"));
    assert!(matches!(app.focus_area, FocusArea::Preview));
    assert!(app.show_diff);
    assert!(!has_scheduled_action(task));
}

#[test]
fn open_existing_tab_outside_project_creates_one_selected_top_level_tab() {
    let mut app = app();
    app.preview_tabs.push(preview_tab("/tmp/a.txt", "a"));
    app.show_file_manager = true;
    app.file_manager_show_changes = false;
    app.screen = Screen::Home;

    let _ = update(&mut app, PreviewMessage::Open("/tmp/a.txt".to_string()));
    let _ = update(&mut app, PreviewMessage::Open("/tmp/a.txt".to_string()));

    let preview_tabs =
        app.open_tabs.iter().filter(|tab| tab.id == "preview:/tmp/a.txt").collect::<Vec<_>>();
    assert_eq!(preview_tabs.len(), 1);
    assert_eq!(preview_tabs[0].title, "a.txt");
    assert_eq!(preview_tabs[0].screen, Screen::Preview);
    assert_eq!(app.active_tab_id.as_deref(), Some("preview:/tmp/a.txt"));
    assert_eq!(app.screen, Screen::Preview);
}

#[test]
fn open_new_instant_preview_creates_tab_without_loading_task() {
    let mut app = app();
    app.workspace_mode = crate::app::state::WorkspaceMode::Code;
    app.screen = Screen::Project;
    app.show_file_manager = false;
    app.file_manager_show_changes = true;

    let task = update(&mut app, PreviewMessage::Open("/tmp/image.png".to_string()));

    assert_eq!(app.preview_tabs.len(), 1);
    assert_eq!(app.preview_tabs[0].path, "/tmp/image.png");
    assert_eq!(app.preview_tabs[0].title, "image.png");
    assert_eq!(app.active_preview_path.as_deref(), Some("/tmp/image.png"));
    assert!(app.show_file_manager);
    assert!(!app.file_manager_show_changes);
    assert!(app.show_diff);
    assert!(!has_scheduled_action(task));
}

#[test]
fn open_new_text_preview_sets_loading_content_and_schedules_gateway_read() {
    let mut app = app();
    app.screen = Screen::Project;
    app.project_path = Some("/workspace/project".to_string());

    let task = update(&mut app, PreviewMessage::Open("src/lib.rs".to_string()));

    assert_eq!(app.preview_tabs.len(), 1);
    assert_eq!(app.preview_tabs[0].content, "加载中…");
    assert!(!app.preview_tabs[0].truncated);
    assert_eq!(app.active_preview_path.as_deref(), Some("src/lib.rs"));
    assert!(has_scheduled_action(task));
}

#[tokio::test(flavor = "multi_thread")]
async fn open_file_in_work_mode_reveals_workspace_files() {
    let mut app = app();
    app.screen = Screen::Project;
    app.workspace_mode = crate::app::state::WorkspaceMode::Work;
    app.work_resources_visible = false;
    app.work_resource_mode = crate::app::state::WorkResourceMode::Browser;
    app.work_chat_fullscreen = true;
    app.work_chat_half_fullscreen = true;

    let task = update(&mut app, PreviewMessage::Open("/tmp/image.png".to_string()));

    assert!(app.work_resources_visible);
    assert_eq!(app.work_resource_mode, crate::app::state::WorkResourceMode::WorkspaceFiles);
    assert!(!app.work_chat_fullscreen);
    assert!(!app.work_chat_half_fullscreen);
    assert!(collect_task_messages(task).await.iter().any(|message| {
        matches!(
            message,
            Message::View(ViewMessage::WorkResourceModeSelected(WorkResourceMode::WorkspaceFiles))
        )
    }));
}

#[test]
fn select_empty_path_clears_active_preview() {
    let mut app = app();
    app.active_preview_path = Some("/tmp/a.txt".to_string());

    let task = update(&mut app, PreviewMessage::Select(String::new()));

    assert!(app.active_preview_path.is_none());
    assert!(!has_scheduled_action(task));
}

#[test]
fn select_path_sets_active_preview_and_focus() {
    let mut app = app();

    let task = update(&mut app, PreviewMessage::Select("/tmp/a.txt".to_string()));

    assert_eq!(app.active_preview_path.as_deref(), Some("/tmp/a.txt"));
    assert!(matches!(app.focus_area, FocusArea::Preview));
    assert!(!has_scheduled_action(task));
}

#[test]
fn close_active_preview_selects_last_remaining_tab() {
    let mut app = app();
    app.preview_tabs.push(preview_tab("/tmp/a.txt", "a"));
    app.preview_tabs.push(preview_tab("/tmp/b.txt", "b"));
    app.active_preview_path = Some("/tmp/a.txt".to_string());

    let _ = update(&mut app, PreviewMessage::Close("/tmp/a.txt".to_string()));

    assert_eq!(app.preview_tabs.len(), 1);
    assert_eq!(app.preview_tabs[0].path, "/tmp/b.txt");
    assert_eq!(app.active_preview_path.as_deref(), Some("/tmp/b.txt"));
}

#[test]
fn close_non_active_or_missing_preview_preserves_active_path() {
    let mut app = app();
    app.preview_tabs.push(preview_tab("/tmp/a.txt", "a"));
    app.preview_tabs.push(preview_tab("/tmp/b.txt", "b"));
    app.active_preview_path = Some("/tmp/a.txt".to_string());

    let _ = update(&mut app, PreviewMessage::Close("/tmp/b.txt".to_string()));
    let _ = update(&mut app, PreviewMessage::Close("/tmp/missing.txt".to_string()));

    assert_eq!(
        app.preview_tabs.iter().map(|tab| tab.path.as_str()).collect::<Vec<_>>(),
        vec!["/tmp/a.txt"]
    );
    assert_eq!(app.active_preview_path.as_deref(), Some("/tmp/a.txt"));
}

#[test]
fn close_only_active_preview_clears_active_path() {
    let mut app = app();
    app.preview_tabs.push(preview_tab("/tmp/a.txt", "a"));
    app.active_preview_path = Some("/tmp/a.txt".to_string());

    let _ = update(&mut app, PreviewMessage::Close("/tmp/a.txt".to_string()));

    assert!(app.preview_tabs.is_empty());
    assert!(app.active_preview_path.is_none());
}

#[test]
fn open_loaded_updates_existing_tab_but_ignores_missing_path() {
    let mut app = app();
    app.preview_tabs.push(preview_tab("/tmp/a.rs", "old"));

    let _ = update(
        &mut app,
        PreviewMessage::OpenLoaded {
            path: "/tmp/a.rs".to_string(),
            content: "new".to_string(),
            truncated: true,
        },
    );
    let _ = update(
        &mut app,
        PreviewMessage::OpenLoaded {
            path: "/tmp/missing.rs".to_string(),
            content: "ignored".to_string(),
            truncated: false,
        },
    );

    let tab = &app.preview_tabs[0];
    assert_eq!(tab.content, "new");
    assert!(!tab.is_dirty);
    assert!(tab.truncated);
    assert_eq!(app.preview_tabs.len(), 1);
}

#[test]
fn open_loaded_matching_pending_target_clears_navigation_and_schedules_goto() {
    let mut app = app();
    app.preview_tabs.push(preview_tab("/tmp/a.txt", "old"));
    app.pending_preview_goto = Some(("/tmp/a.txt".to_string(), 4, 7));
    app.preview_trace_navigating = true;

    let task = update(
        &mut app,
        PreviewMessage::OpenLoaded {
            path: "/tmp/a.txt".to_string(),
            content: "new".to_string(),
            truncated: false,
        },
    );

    assert!(app.pending_preview_goto.is_none());
    assert!(!app.preview_trace_navigating);
    assert!(has_scheduled_action(task));
}

#[tokio::test(flavor = "multi_thread")]
async fn save_file_emits_save_for_active_path_and_none_without_active_path() {
    let mut app = app();
    app.active_preview_path = Some("/tmp/a.txt".to_string());

    let messages = collect_task_messages(update(&mut app, PreviewMessage::SaveFile)).await;
    assert!(matches!(
        messages.as_slice(),
        [Message::Preview(PreviewMessage::SaveFilePath { path, notify: false })]
            if path == "/tmp/a.txt"
    ));

    app.active_preview_path = None;
    assert!(!has_scheduled_action(update(&mut app, PreviewMessage::SaveFile)));
}

#[test]
fn save_file_path_rejects_missing_or_clean_tab_without_io() {
    let mut app = app();
    app.preview_tabs.push(preview_tab("/tmp/a.txt", "a"));
    app.show_preview_context_menu = true;

    assert!(!has_scheduled_action(update(
        &mut app,
        PreviewMessage::SaveFilePath { path: "/tmp/missing.txt".to_string(), notify: false },
    )));
    assert!(app.show_preview_context_menu);

    assert!(!has_scheduled_action(update(
        &mut app,
        PreviewMessage::SaveFilePath { path: "/tmp/a.txt".to_string(), notify: false },
    )));
    assert!(!app.show_preview_context_menu);
}

#[test]
fn save_file_path_dirty_tab_schedules_persist_and_closes_context_menu() {
    let mut app = app();
    let mut tab = preview_tab("/tmp/a.txt", "old");
    tab.is_dirty = true;
    app.preview_tabs.push(tab);
    app.show_preview_context_menu = true;

    let task = update(
        &mut app,
        PreviewMessage::SaveFilePath { path: "/tmp/a.txt".to_string(), notify: true },
    );

    assert!(!app.show_preview_context_menu);
    assert!(has_scheduled_action(task));
}

#[test]
fn save_file_finished_success_updates_matching_tab() {
    let mut app = app();
    let mut tab = preview_tab("/tmp/a.txt", "old");
    tab.editor = crate::app::components::editor::Editor::new("saved", "text");
    tab.is_dirty = true;
    app.preview_tabs.push(tab);

    let task = update(
        &mut app,
        PreviewMessage::SaveFileFinished {
            path: "/tmp/a.txt".to_string(),
            content: "saved".to_string(),
            notify: false,
            result: Ok(()),
        },
    );

    assert_eq!(app.preview_tabs[0].content, "saved");
    assert!(!app.preview_tabs[0].is_dirty);
    assert!(!app.preview_tabs[0].editor.inner.is_modified());
    assert!(app.active_toast.is_none());
    assert!(!has_scheduled_action(task));
}

#[test]
fn save_file_finished_keeps_newer_editor_content_dirty() {
    let mut app = app();
    let mut tab = preview_tab("/tmp/a.txt", "old");
    let _ = tab.editor.inner.update(&iced_code_editor::Message::Enter);
    tab.is_dirty = true;
    app.preview_tabs.push(tab);

    let _ = update(
        &mut app,
        PreviewMessage::SaveFileFinished {
            path: "/tmp/a.txt".to_string(),
            content: "old".to_string(),
            notify: false,
            result: Ok(()),
        },
    );

    assert!(app.preview_tabs[0].is_dirty);
    assert!(app.preview_tabs[0].editor.inner.is_modified());
}

#[test]
fn save_file_finished_reports_success_and_failure_toasts() {
    let mut app = app();

    let _ = update(
        &mut app,
        PreviewMessage::SaveFileFinished {
            path: "/tmp/missing.txt".to_string(),
            content: "saved".to_string(),
            notify: true,
            result: Ok(()),
        },
    );
    let success = app.active_toast.as_ref().expect("success should show a toast");
    assert_eq!(success.kind, ToastKind::Success);
    assert_eq!(success.message, "已保存");

    let _ = update(
        &mut app,
        PreviewMessage::SaveFileFinished {
            path: "/tmp/missing.txt".to_string(),
            content: String::new(),
            notify: false,
            result: Err("gateway unavailable".to_string()),
        },
    );
    let failure = app.active_toast.as_ref().expect("failure should show a toast");
    assert_eq!(failure.kind, ToastKind::Error);
    assert_eq!(failure.message, "保存失败: gateway unavailable");
}

#[tokio::test(flavor = "multi_thread")]
async fn auto_save_delay_emits_save_only_for_matching_dirty_revision() {
    let mut app = app();
    let mut tab = preview_tab("/tmp/a.txt", "a");
    tab.is_dirty = true;
    tab.auto_save_revision = 4;
    app.preview_tabs.push(tab);
    app.preview_auto_save_mode = PreviewAutoSaveMode::AfterDelay;

    let messages = collect_task_messages(update(
        &mut app,
        PreviewMessage::AutoSaveDelayElapsed { path: "/tmp/a.txt".to_string(), revision: 4 },
    ))
    .await;

    assert!(matches!(
        messages.as_slice(),
        [Message::Preview(PreviewMessage::SaveFilePath { path, notify: false })]
            if path == "/tmp/a.txt"
    ));

    assert!(!has_scheduled_action(update(
        &mut app,
        PreviewMessage::AutoSaveDelayElapsed { path: "/tmp/a.txt".to_string(), revision: 3 },
    )));
    app.preview_tabs[0].is_dirty = false;
    assert!(!has_scheduled_action(update(
        &mut app,
        PreviewMessage::AutoSaveDelayElapsed { path: "/tmp/a.txt".to_string(), revision: 4 },
    )));
    app.preview_auto_save_mode = PreviewAutoSaveMode::Off;
    assert!(!has_scheduled_action(update(
        &mut app,
        PreviewMessage::AutoSaveDelayElapsed { path: "/tmp/a.txt".to_string(), revision: 4 },
    )));
}

#[test]
fn auto_save_delay_rejects_missing_target_without_scheduling_io() {
    let mut app = app();
    app.preview_auto_save_mode = PreviewAutoSaveMode::AfterDelay;

    let task = update(
        &mut app,
        PreviewMessage::AutoSaveDelayElapsed { path: "/tmp/missing.txt".to_string(), revision: 1 },
    );

    assert!(!has_scheduled_action(task));
}

#[test]
fn auto_save_mode_changed_updates_state_and_returns_save_task() {
    let mut app = app();
    app.preview_auto_save_mode = PreviewAutoSaveMode::Off;

    let task =
        update(&mut app, PreviewMessage::AutoSaveModeChanged(PreviewAutoSaveMode::OnFocusChange));

    assert_eq!(app.preview_auto_save_mode, PreviewAutoSaveMode::OnFocusChange);
    assert!(has_scheduled_action(task));
}

#[tokio::test(flavor = "multi_thread")]
async fn window_unfocused_saves_only_active_dirty_tab_in_supported_mode() {
    let mut app = app();
    let mut tab = preview_tab("/tmp/a.txt", "a");
    tab.is_dirty = true;
    app.preview_tabs.push(tab);
    app.active_preview_path = Some("/tmp/a.txt".to_string());
    app.preview_auto_save_mode = PreviewAutoSaveMode::OnWindowChange;

    let messages = collect_task_messages(update(&mut app, PreviewMessage::WindowUnfocused)).await;
    assert!(matches!(
        messages.as_slice(),
        [Message::Preview(PreviewMessage::SaveFilePath { path, notify: false })]
            if path == "/tmp/a.txt"
    ));

    app.preview_auto_save_mode = PreviewAutoSaveMode::AfterDelay;
    assert!(!has_scheduled_action(update(&mut app, PreviewMessage::WindowUnfocused)));
    app.preview_auto_save_mode = PreviewAutoSaveMode::OnFocusChange;
    app.preview_tabs[0].is_dirty = false;
    assert!(!has_scheduled_action(update(&mut app, PreviewMessage::WindowUnfocused)));
    app.active_preview_path = None;
    assert!(!has_scheduled_action(update(&mut app, PreviewMessage::WindowUnfocused)));
}

#[test]
fn path_segment_clicked_lists_directories_first_and_hides_dot_entries() {
    let temp = tempfile::tempdir().expect("temporary preview directory should be created");
    std::fs::create_dir(temp.path().join("z_dir")).expect("directory fixture should be created");
    std::fs::write(temp.path().join("B.txt"), "b").expect("file fixture should be created");
    std::fs::write(temp.path().join("a.txt"), "a").expect("file fixture should be created");
    std::fs::write(temp.path().join(".hidden"), "hidden")
        .expect("hidden fixture should be created");
    let mut app = app();
    app.show_preview_context_menu = true;
    app.preview_tab_menu_path = Some("old".to_string());

    let _ = update(
        &mut app,
        PreviewMessage::PathSegmentClicked(
            temp.path().to_string_lossy().to_string(),
            Some((12.0, 34.0)),
        ),
    );

    let (root, x, y, items) = app.preview_nav_popup.as_ref().expect("popup should be opened");
    assert_eq!(root, &temp.path().to_string_lossy().to_string());
    assert_eq!((*x, *y), (12.0, 34.0));
    assert_eq!(
        items.as_slice(),
        &[("z_dir".to_string(), true), ("a.txt".to_string(), false), ("B.txt".to_string(), false),]
    );
    assert!(!app.show_preview_context_menu);
    assert!(app.preview_tab_menu_path.is_none());
}

#[test]
fn path_segment_clicked_for_file_lists_parent_at_cursor_position() {
    let temp = tempfile::tempdir().expect("temporary preview directory should be created");
    let file = temp.path().join("a.txt");
    std::fs::write(&file, "a").expect("file fixture should be created");
    let mut app = app();
    app.cursor_position = iced::Point::new(21.0, 43.0);

    let _ = update(
        &mut app,
        PreviewMessage::PathSegmentClicked(file.to_string_lossy().to_string(), None),
    );

    let (root, x, y, items) = app.preview_nav_popup.as_ref().expect("popup should be opened");
    assert_eq!(root, &temp.path().to_string_lossy().to_string());
    assert_eq!((*x, *y), (21.0, 43.0));
    assert_eq!(items, &vec![("a.txt".to_string(), false)]);
}

#[test]
fn invalid_path_segment_dismisses_existing_popup_without_opening_another() {
    let mut app = app();
    app.preview_nav_popup = Some(("old".to_string(), 1.0, 2.0, Vec::new()));

    let _ = update(
        &mut app,
        PreviewMessage::PathSegmentClicked("/definitely/missing/vw-preview-path".to_string(), None),
    );

    assert!(app.preview_nav_popup.is_none());
}

#[test]
fn close_nav_popup_clears_popup() {
    let mut app = app();
    app.preview_nav_popup = Some(("/tmp".to_string(), 1.0, 2.0, Vec::new()));

    let _ = update(&mut app, PreviewMessage::CloseNavPopup);

    assert!(app.preview_nav_popup.is_none());
}

#[test]
fn tab_right_click_replaces_other_preview_popups() {
    let mut app = app();
    app.show_preview_context_menu = true;
    app.preview_nav_popup = Some(("/tmp".to_string(), 1.0, 2.0, Vec::new()));

    let _ = update(&mut app, PreviewMessage::TabRightClicked("/tmp/a.txt".to_string(), 10.0, 20.0));

    assert!(!app.show_preview_context_menu);
    assert!(app.preview_nav_popup.is_none());
    assert_eq!(app.preview_tab_menu_path.as_deref(), Some("/tmp/a.txt"));
    assert_eq!(app.preview_tab_menu_pos, Some(iced::Point::new(10.0, 20.0)));
}

#[test]
fn tab_menu_close_clears_path_and_position() {
    let mut app = app();
    app.preview_tab_menu_path = Some("/tmp/a.txt".to_string());
    app.preview_tab_menu_pos = Some(iced::Point::new(1.0, 2.0));

    let _ = update(&mut app, PreviewMessage::TabMenuClose);

    assert!(app.preview_tab_menu_path.is_none());
    assert!(app.preview_tab_menu_pos.is_none());
}

#[test]
fn tab_menu_close_left_removes_tabs_before_target() {
    let mut app = app();
    app.preview_tabs = vec![
        preview_tab("/tmp/a.txt", "a"),
        preview_tab("/tmp/b.txt", "b"),
        preview_tab("/tmp/c.txt", "c"),
    ];
    app.active_preview_path = Some("/tmp/a.txt".to_string());

    let _ = update(&mut app, PreviewMessage::TabMenuCloseLeft("/tmp/b.txt".to_string()));

    assert_eq!(
        app.preview_tabs.iter().map(|tab| tab.path.as_str()).collect::<Vec<_>>(),
        vec!["/tmp/b.txt", "/tmp/c.txt"]
    );
    assert_eq!(app.active_preview_path.as_deref(), Some("/tmp/b.txt"));
}

#[test]
fn tab_menu_close_right_removes_tabs_after_target() {
    let mut app = app();
    app.preview_tabs = vec![
        preview_tab("/tmp/a.txt", "a"),
        preview_tab("/tmp/b.txt", "b"),
        preview_tab("/tmp/c.txt", "c"),
    ];
    app.active_preview_path = Some("/tmp/c.txt".to_string());

    let _ = update(&mut app, PreviewMessage::TabMenuCloseRight("/tmp/b.txt".to_string()));

    assert_eq!(
        app.preview_tabs.iter().map(|tab| tab.path.as_str()).collect::<Vec<_>>(),
        vec!["/tmp/a.txt", "/tmp/b.txt"]
    );
    assert_eq!(app.active_preview_path.as_deref(), Some("/tmp/b.txt"));
}

#[test]
fn tab_menu_close_missing_target_only_dismisses_menu() {
    let mut app = app();
    app.preview_tabs.push(preview_tab("/tmp/a.txt", "a"));
    app.active_preview_path = Some("/tmp/a.txt".to_string());
    app.preview_tab_menu_path = Some("/tmp/missing.txt".to_string());
    app.preview_tab_menu_pos = Some(iced::Point::new(1.0, 2.0));

    let _ = update(&mut app, PreviewMessage::TabMenuCloseLeft("/tmp/missing.txt".to_string()));

    assert_eq!(app.preview_tabs.len(), 1);
    assert_eq!(app.active_preview_path.as_deref(), Some("/tmp/a.txt"));
    assert!(app.preview_tab_menu_path.is_none());
    assert!(app.preview_tab_menu_pos.is_none());
}

#[test]
fn tab_menu_close_right_missing_target_only_dismisses_menu() {
    let mut app = app();
    app.preview_tabs.push(preview_tab("/tmp/a.txt", "a"));
    app.active_preview_path = Some("/tmp/a.txt".to_string());
    app.preview_tab_menu_path = Some("/tmp/missing.txt".to_string());
    app.preview_tab_menu_pos = Some(iced::Point::new(1.0, 2.0));

    let _ = update(&mut app, PreviewMessage::TabMenuCloseRight("/tmp/missing.txt".to_string()));

    assert_eq!(app.preview_tabs.len(), 1);
    assert_eq!(app.active_preview_path.as_deref(), Some("/tmp/a.txt"));
    assert!(app.preview_tab_menu_path.is_none());
    assert!(app.preview_tab_menu_pos.is_none());
}

#[test]
fn tab_menu_close_all_clears_tabs_active_path_and_menu() {
    let mut app = app();
    app.preview_tabs = vec![preview_tab("/tmp/a.txt", "a"), preview_tab("/tmp/b.txt", "b")];
    app.active_preview_path = Some("/tmp/a.txt".to_string());
    app.preview_tab_menu_path = Some("/tmp/a.txt".to_string());
    app.preview_tab_menu_pos = Some(iced::Point::new(1.0, 2.0));

    let _ = update(&mut app, PreviewMessage::TabMenuCloseAll);

    assert!(app.preview_tabs.is_empty());
    assert!(app.active_preview_path.is_none());
    assert!(app.preview_tab_menu_path.is_none());
    assert!(app.preview_tab_menu_pos.is_none());
}
