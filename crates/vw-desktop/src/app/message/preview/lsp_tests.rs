// 验证预览标签的 LSP 绑定、同步、分离与结果过滤。
// 测试夹具围绕目标分支构造可控状态；跨模块行为由对应集成测试负责。

// 编译接线：引用父模块符号，确保测试片段仍由真实模块包含。
// 该冒烟用例只承担接线职责，具体行为由同领域测试覆盖。
#[test]
fn test_module_is_wired() {
    let module = module_path!();

    assert!(module.ends_with("lsp_tests"));
}

fn app() -> crate::app::App {
    crate::app::App::new().0
}

fn preview_tab(path: &str, content: &str) -> crate::app::PreviewTab {
    crate::app::PreviewTab {
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
        lsp_server_key: None,
        lsp_uri: None,
        lsp_language_id: None,
    }
}

fn install_lsp_events(
    app: &mut crate::app::App,
    events: impl IntoIterator<Item = crate::app::lsp::LspEvent>,
) {
    let (sender, receiver) = std::sync::mpsc::channel();
    for event in events {
        sender.send(event).expect("test receiver should remain connected");
    }
    drop(sender);
    app.lsp_events = Some(receiver);
}

#[test]
fn lsp_results_rebind_overlay_to_active_preview() {
    let active_path = "/tmp/active.rs";
    let mut app = app();
    app.active_preview_path = Some(active_path.to_string());
    app.preview_tabs.push(preview_tab(active_path, "fn main() {}"));
    app.lsp_overlay_path = Some("/tmp/previous.rs".to_string());
    install_lsp_events(&mut app, [crate::app::lsp::LspEvent::Hover { text: "hover".to_string() }]);

    let _ = super::lsp::drain_lsp_events(&mut app);

    assert_eq!(app.lsp_overlay_path.as_deref(), Some(active_path));

    app.lsp_overlay_path = Some("/tmp/previous.rs".to_string());
    install_lsp_events(
        &mut app,
        [crate::app::lsp::LspEvent::Completion { items: vec!["main".to_string()] }],
    );

    let _ = super::lsp::drain_lsp_events(&mut app);

    assert_eq!(app.lsp_overlay_path.as_deref(), Some(active_path));
}

#[test]
fn detach_lsp_for_tab_clears_all_document_metadata() {
    let mut tab = preview_tab("/tmp/main.rs", "fn main() {}");
    tab.lsp_server_key = Some("rust-analyzer");
    tab.lsp_uri = Some("file:///tmp/main.rs".to_string());
    tab.lsp_language_id = Some("rust".to_string());

    super::lsp::detach_lsp_for_tab(&mut tab);

    assert!(tab.lsp_server_key.is_none());
    assert!(tab.lsp_uri.is_none());
    assert!(tab.lsp_language_id.is_none());
    assert!(!tab.editor.has_lsp_attached());
}

#[test]
fn sync_lsp_for_path_rejects_disabled_lsp_and_detaches_matching_tab() {
    let mut app = app();
    app.lsp_disabled = true;
    let mut tab = preview_tab("/tmp/main.rs", "fn main() {}");
    tab.lsp_server_key = Some("rust-analyzer");
    tab.lsp_uri = Some("file:///tmp/main.rs".to_string());
    tab.lsp_language_id = Some("rust".to_string());
    app.preview_tabs.push(tab);

    let synced = super::lsp::sync_lsp_for_path(&mut app, "/tmp/main.rs");

    assert!(!synced);
    assert!(app.preview_tabs[0].lsp_server_key.is_none());
    assert!(app.preview_tabs[0].lsp_uri.is_none());
    assert!(app.preview_tabs[0].lsp_language_id.is_none());
}

#[test]
fn sync_lsp_for_path_rejects_missing_tab_without_changing_status() {
    let mut app = app();
    app.lsp_disabled = false;
    app.lsp_status = Some("unchanged".to_string());

    let synced = super::lsp::sync_lsp_for_path(&mut app, "/tmp/missing.rs");

    assert!(!synced);
    assert_eq!(app.lsp_status.as_deref(), Some("unchanged"));
}

#[test]
fn lsp_workspace_root_prefers_project_containing_preview_file() {
    let project = std::env::current_dir().unwrap().join("nested-project");
    let file = project.join("src").join("main.rs");
    let project_path = project.to_string_lossy();

    let root_uri = super::lsp::lsp_root_uri_for_preview_path(Some(&project_path), &file);

    assert_eq!(root_uri, Some(crate::app::path_to_file_uri(&project)));
}

#[test]
fn lsp_workspace_root_keeps_active_project_for_external_definition() {
    let project = std::env::current_dir().unwrap().join("active-project");
    let external_file = std::env::temp_dir().join("dependency").join("lib.rs");
    let project_path = project.to_string_lossy();

    let root_uri = super::lsp::lsp_root_uri_for_preview_path(Some(&project_path), &external_file);

    assert_eq!(root_uri, Some(crate::app::path_to_file_uri(&project)));
}

#[test]
fn lsp_workspace_root_uses_file_parent_without_active_project() {
    let file = std::env::current_dir().unwrap().join("standalone").join("main.rs");
    let expected_root = file.parent().unwrap();

    let root_uri = super::lsp::lsp_root_uri_for_preview_path(None, &file);

    assert_eq!(root_uri, Some(crate::app::path_to_file_uri(expected_root)));
}

#[test]
fn sync_lsp_for_path_rejects_unknown_language_and_detaches_metadata() {
    let mut app = app();
    app.lsp_disabled = false;
    let mut tab = preview_tab("/tmp/data.unknown-language", "content");
    tab.lsp_server_key = Some("rust-analyzer");
    tab.lsp_uri = Some("file:///tmp/data.unknown-language".to_string());
    tab.lsp_language_id = Some("rust".to_string());
    app.preview_tabs.push(tab);

    let synced = super::lsp::sync_lsp_for_path(&mut app, "/tmp/data.unknown-language");

    assert!(!synced);
    assert!(app.preview_tabs[0].lsp_server_key.is_none());
    assert!(app.preview_tabs[0].lsp_uri.is_none());
    assert!(app.preview_tabs[0].lsp_language_id.is_none());
}

#[test]
fn sync_lsp_for_path_rejects_known_language_without_manager_after_detach() {
    let mut app = app();
    app.lsp_disabled = false;
    app.lsp_manager = None;
    let mut tab = preview_tab("/tmp/main.rs", "fn main() {}");
    tab.lsp_server_key = Some("rust-analyzer");
    tab.lsp_uri = Some("file:///tmp/main.rs".to_string());
    tab.lsp_language_id = Some("rust".to_string());
    app.preview_tabs.push(tab);

    let synced = super::lsp::sync_lsp_for_path(&mut app, "/tmp/main.rs");

    assert!(!synced);
    assert!(app.preview_tabs[0].lsp_server_key.is_none());
    assert!(app.preview_tabs[0].lsp_uri.is_none());
    assert!(app.preview_tabs[0].lsp_language_id.is_none());
}

#[test]
fn enable_lsp_without_active_preview_only_marks_lsp_enabled() {
    let mut app = app();
    app.lsp_disabled = true;
    app.lsp_manager = None;
    app.active_preview_path = None;

    super::lsp::enable_lsp(&mut app);

    assert!(!app.lsp_disabled);
    assert_eq!(app.lsp_status.as_deref(), Some("LSP 已启用"));
}

#[test]
fn enable_lsp_with_active_unknown_language_marks_enabled_and_detaches_tab() {
    let mut app = app();
    app.lsp_disabled = true;
    app.lsp_manager = None;
    app.active_preview_path = Some("/tmp/data.unknown-language".to_string());
    let mut tab = preview_tab("/tmp/data.unknown-language", "content");
    tab.lsp_server_key = Some("rust-analyzer");
    tab.lsp_uri = Some("file:///tmp/data.unknown-language".to_string());
    tab.lsp_language_id = Some("rust".to_string());
    app.preview_tabs.push(tab);

    super::lsp::enable_lsp(&mut app);

    assert!(!app.lsp_disabled);
    assert_eq!(app.lsp_status.as_deref(), Some("LSP 已启用"));
    assert!(app.preview_tabs[0].lsp_server_key.is_none());
}

#[test]
fn clear_lsp_hover_clears_hover_state_and_path_when_completion_hidden() {
    let mut app = app();
    app.lsp_overlay.show_hover("hover".to_string());
    app.lsp_overlay_path = Some("/tmp/main.rs".to_string());
    app.lsp_hover_anchor =
        Some(("/tmp/main.rs".to_string(), iced_code_editor::LspPosition { line: 1, character: 2 }));
    app.lsp_hover_pending = Some(crate::app::LspHoverPending {
        path: "/tmp/main.rs".to_string(),
        position: iced_code_editor::LspPosition { line: 1, character: 2 },
        point: iced::Point::new(10.0, 20.0),
        ready_at: std::time::Instant::now(),
    });
    app.lsp_hover_hide_deadline = Some(std::time::Instant::now());

    super::lsp::clear_lsp_hover(&mut app);

    assert!(!app.lsp_overlay.hover_visible);
    assert!(app.lsp_hover_anchor.is_none());
    assert!(app.lsp_hover_pending.is_none());
    assert!(app.lsp_hover_hide_deadline.is_none());
    assert!(app.lsp_overlay_path.is_none());
}

#[test]
fn clear_lsp_hover_preserves_path_while_completion_is_visible() {
    let mut app = app();
    app.lsp_overlay.show_hover("hover".to_string());
    app.lsp_overlay.completion_visible = true;
    app.lsp_overlay_path = Some("/tmp/main.rs".to_string());

    super::lsp::clear_lsp_hover(&mut app);

    assert!(!app.lsp_overlay.hover_visible);
    assert_eq!(app.lsp_overlay_path.as_deref(), Some("/tmp/main.rs"));
}

// 数据边界：覆盖规范化、默认值以及相关状态字段的既定规则。
// 这里固定可观察结果，避免依赖处理过程中的临时实现细节。
#[test]
fn clear_lsp_completion_clears_items_and_preserves_hover_path() {
    let mut app = app();
    app.lsp_overlay.all_completions = vec!["Alpha".to_string(), "Beta".to_string()];
    app.lsp_overlay.completion_items = vec!["Alpha".to_string()];
    app.lsp_overlay.completion_filter = "a".to_string();
    app.lsp_overlay.completion_visible = true;
    app.lsp_overlay.completion_selected = 9;
    app.lsp_overlay.hover_visible = true;
    app.lsp_overlay_path = Some("/tmp/main.rs".to_string());

    super::lsp::clear_lsp_completion(&mut app, true);

    assert!(app.lsp_overlay.all_completions.is_empty());
    assert!(app.lsp_overlay.completion_items.is_empty());
    assert!(app.lsp_overlay.completion_filter.is_empty());
    assert!(!app.lsp_overlay.completion_visible);
    assert!(app.lsp_overlay.completion_suppressed);
    assert_eq!(app.lsp_overlay.completion_selected, 0);
    assert_eq!(app.lsp_overlay_path.as_deref(), Some("/tmp/main.rs"));
}

#[test]
fn clear_lsp_completion_clears_path_when_hover_is_hidden() {
    let mut app = app();
    app.lsp_overlay.completion_visible = true;
    app.lsp_overlay.completion_suppressed = true;
    app.lsp_overlay.hover_visible = false;
    app.lsp_overlay_path = Some("/tmp/main.rs".to_string());

    super::lsp::clear_lsp_completion(&mut app, false);

    assert!(!app.lsp_overlay.completion_suppressed);
    assert!(app.lsp_overlay_path.is_none());
}

#[test]
fn filter_completions_matches_case_insensitively_and_clamps_selection() {
    let mut app = app();
    app.lsp_overlay.all_completions =
        vec!["ReadFile".to_string(), "write_file".to_string(), "Close".to_string()];
    app.lsp_overlay.completion_filter = "FILE".to_string();
    app.lsp_overlay.completion_selected = 8;

    super::lsp::filter_completions(&mut app);

    assert_eq!(app.lsp_overlay.completion_filter, "file");
    assert_eq!(
        app.lsp_overlay.completion_items,
        vec!["ReadFile".to_string(), "write_file".to_string()]
    );
    assert!(app.lsp_overlay.completion_visible);
    assert_eq!(app.lsp_overlay.completion_selected, 1);
}

#[test]
fn filter_completions_hides_empty_result() {
    let mut app = app();
    app.lsp_overlay.all_completions = vec!["Alpha".to_string()];
    app.lsp_overlay.completion_filter = "zzz".to_string();
    app.lsp_overlay.completion_selected = 4;

    super::lsp::filter_completions(&mut app);

    assert!(app.lsp_overlay.completion_items.is_empty());
    assert!(!app.lsp_overlay.completion_visible);
    assert_eq!(app.lsp_overlay.completion_selected, 0);
}

#[test]
fn filter_completions_with_empty_filter_restores_all_items_and_keeps_valid_selection() {
    let mut app = app();
    app.lsp_overlay.all_completions = vec!["Alpha".to_string(), "Beta".to_string()];
    app.lsp_overlay.completion_items = vec!["stale".to_string()];
    app.lsp_overlay.completion_filter = String::new();
    app.lsp_overlay.completion_selected = 1;

    super::lsp::filter_completions(&mut app);

    assert_eq!(app.lsp_overlay.completion_items, app.lsp_overlay.all_completions);
    assert!(app.lsp_overlay.completion_visible);
    assert_eq!(app.lsp_overlay.completion_selected, 1);
}

#[test]
fn find_word_start_handles_boundaries_unicode_and_separators() {
    assert_eq!(super::lsp::find_word_start("hello_world", 11), 0);
    assert_eq!(super::lsp::find_word_start("let value", 9), 4);
    assert_eq!(super::lsp::find_word_start("变量_name", 7), 0);
    assert_eq!(super::lsp::find_word_start("value", 0), 0);
    assert_eq!(super::lsp::find_word_start("value", 99), 99);
}

#[test]
fn apply_completion_without_matching_active_preview_preserves_editor_content() {
    let mut app = app();
    app.preview_tabs.push(preview_tab("/tmp/main.rs", "let value = 1;"));
    let original = app.preview_tabs[0].editor.content();

    super::lsp::apply_completion(&mut app, "replacement");
    assert_eq!(app.preview_tabs[0].editor.content(), original);

    app.active_preview_path = Some("/tmp/missing.rs".to_string());
    super::lsp::apply_completion(&mut app, "replacement");

    assert_eq!(app.preview_tabs[0].editor.content(), original);
}

#[test]
fn file_uri_to_path_decodes_spaces_and_rejects_non_file_uri() {
    assert_eq!(
        super::lsp::file_uri_to_path("file:///tmp/my%20file.rs").as_deref(),
        Some("/tmp/my file.rs")
    );
    assert_eq!(super::lsp::file_uri_to_path("https://example.com/main.rs"), None);
}

#[test]
fn process_lsp_hover_timers_clears_visible_hover_without_path() {
    let mut app = app();
    app.lsp_overlay.show_hover("hover".to_string());
    app.lsp_overlay_path = None;

    super::lsp::process_lsp_hover_timers(&mut app);

    assert!(!app.lsp_overlay.hover_visible);
    assert!(app.lsp_hover_anchor.is_none());
}

#[test]
fn process_lsp_hover_timers_retains_request_before_deadline() {
    let mut app = app();
    app.lsp_hover_pending = Some(crate::app::LspHoverPending {
        path: "/tmp/main.rs".to_string(),
        position: iced_code_editor::LspPosition { line: 2, character: 3 },
        point: iced::Point::new(8.0, 9.0),
        ready_at: std::time::Instant::now() + std::time::Duration::from_secs(60),
    });

    super::lsp::process_lsp_hover_timers(&mut app);

    let pending = app.lsp_hover_pending.expect("request should remain pending");
    assert_eq!(pending.path, "/tmp/main.rs");
    assert_eq!(pending.position, iced_code_editor::LspPosition { line: 2, character: 3 });
}

#[test]
fn process_lsp_hover_timers_clears_anchor_when_ready_tab_is_missing() {
    let mut app = app();
    let position = iced_code_editor::LspPosition { line: 2, character: 3 };
    app.lsp_hover_anchor = Some(("/tmp/main.rs".to_string(), position));
    app.lsp_hover_pending = Some(crate::app::LspHoverPending {
        path: "/tmp/main.rs".to_string(),
        position,
        point: iced::Point::new(8.0, 9.0),
        ready_at: std::time::Instant::now() - std::time::Duration::from_secs(1),
    });

    super::lsp::process_lsp_hover_timers(&mut app);

    assert!(app.lsp_hover_pending.is_none());
    assert!(app.lsp_hover_anchor.is_none());
    assert!(app.lsp_overlay_path.is_none());
}

#[test]
fn process_lsp_hover_timers_hides_expired_noninteractive_hover() {
    let mut app = app();
    app.lsp_overlay.show_hover("hover".to_string());
    app.lsp_overlay.hover_interactive = false;
    app.lsp_overlay_path = Some("/tmp/main.rs".to_string());
    app.lsp_hover_hide_deadline =
        Some(std::time::Instant::now() - std::time::Duration::from_secs(1));

    super::lsp::process_lsp_hover_timers(&mut app);

    assert!(!app.lsp_overlay.hover_visible);
    assert!(app.lsp_hover_hide_deadline.is_none());
    assert!(app.lsp_overlay_path.is_none());
}

#[test]
fn process_lsp_hover_timers_preserves_expired_interactive_hover() {
    let mut app = app();
    app.lsp_overlay.show_hover("hover".to_string());
    app.lsp_overlay.hover_interactive = true;
    app.lsp_overlay_path = Some("/tmp/main.rs".to_string());
    app.lsp_hover_hide_deadline =
        Some(std::time::Instant::now() - std::time::Duration::from_secs(1));

    super::lsp::process_lsp_hover_timers(&mut app);

    assert!(app.lsp_overlay.hover_visible);
    assert!(app.lsp_overlay.hover_interactive);
    assert!(app.lsp_hover_hide_deadline.is_some());
    assert_eq!(app.lsp_overlay_path.as_deref(), Some("/tmp/main.rs"));
}

#[test]
fn handle_lsp_hover_from_mouse_ignores_missing_active_preview() {
    let mut app = app();
    app.active_preview_path = None;

    super::lsp::handle_lsp_hover_from_mouse(&mut app, iced::Point::new(10.0, 20.0));

    assert!(app.lsp_hover_pending.is_none());
    assert!(app.lsp_hover_anchor.is_none());
}

// 失败与边界：覆盖非法、缺失、过期或冲突场景的既定处理规则。
// 重点确认现有状态与错误反馈不会被意外的部分更新污染。
#[test]
fn handle_lsp_hover_from_mouse_ignores_context_menu() {
    let mut app = app();
    app.show_preview_context_menu = true;
    app.active_preview_path = Some("/tmp/main.rs".to_string());
    app.lsp_hover_hide_deadline = Some(std::time::Instant::now());

    super::lsp::handle_lsp_hover_from_mouse(&mut app, iced::Point::new(10.0, 20.0));

    assert!(app.lsp_hover_hide_deadline.is_some());
    assert!(app.lsp_hover_pending.is_none());
}

#[test]
fn handle_lsp_hover_from_mouse_releases_stale_interactive_overlay() {
    let mut app = app();
    app.active_preview_path = Some("/tmp/main.rs".to_string());
    app.lsp_overlay.show_hover("hover".to_string());
    app.lsp_overlay.hover_interactive = true;
    app.lsp_overlay_path = Some("/tmp/other.rs".to_string());
    app.lsp_hover_pending = Some(crate::app::LspHoverPending {
        path: "/tmp/other.rs".to_string(),
        position: iced_code_editor::LspPosition { line: 1, character: 1 },
        point: iced::Point::new(1.0, 1.0),
        ready_at: std::time::Instant::now(),
    });

    super::lsp::handle_lsp_hover_from_mouse(&mut app, iced::Point::new(10.0, 20.0));

    assert!(!app.lsp_overlay.hover_interactive);
    assert!(app.lsp_hover_pending.is_none());
    assert!(app.lsp_hover_anchor.is_none());
}

#[test]
fn handle_lsp_hover_from_mouse_keeps_visible_hover_for_same_path_without_anchor() {
    let mut app = app();
    app.active_preview_path = Some("/tmp/main.rs".to_string());
    app.lsp_overlay.show_hover("hover".to_string());
    app.lsp_overlay_path = Some("/tmp/main.rs".to_string());

    super::lsp::handle_lsp_hover_from_mouse(&mut app, iced::Point::new(10.0, 20.0));

    assert!(app.lsp_overlay.hover_visible);
    assert!(app.lsp_hover_hide_deadline.is_none());
}

#[test]
fn handle_lsp_hover_from_mouse_schedules_hide_for_other_visible_hover_without_anchor() {
    let mut app = app();
    app.active_preview_path = Some("/tmp/main.rs".to_string());
    app.lsp_overlay.show_hover("hover".to_string());
    app.lsp_overlay_path = Some("/tmp/other.rs".to_string());

    super::lsp::handle_lsp_hover_from_mouse(&mut app, iced::Point::new(10.0, 20.0));

    assert!(app.lsp_overlay.hover_visible);
    assert!(app.lsp_hover_hide_deadline.is_some());
}

#[test]
fn handle_lsp_hover_from_mouse_keeps_pending_deadline_for_same_symbol() {
    let path = "/tmp/main.rs";
    let mut app = app();
    app.active_preview_path = Some(path.to_string());
    app.preview_tabs.push(preview_tab(path, "fn main() {}"));
    let point = iced::Point::new(100.0, 10.0);

    super::lsp::handle_lsp_hover_from_mouse(&mut app, point);
    let first_ready_at = app
        .lsp_hover_pending
        .as_ref()
        .expect("first hover should schedule a request")
        .ready_at;

    super::lsp::handle_lsp_hover_from_mouse(&mut app, point);

    assert_eq!(
        app.lsp_hover_pending
            .as_ref()
            .expect("same-symbol hover should keep the pending request")
            .ready_at,
        first_ready_at
    );
}

#[test]
fn drain_lsp_events_without_receiver_returns_none_and_preserves_state() {
    let mut app = app();
    app.lsp_status = Some("unchanged".to_string());
    app.lsp_events = None;

    let _ = super::lsp::drain_lsp_events(&mut app);

    assert_eq!(app.lsp_status.as_deref(), Some("unchanged"));
    assert!(app.lsp_events.is_none());
}

#[test]
fn drain_lsp_events_retains_empty_receiver_when_channel_is_open() {
    let mut app = app();
    let (_sender, receiver) = std::sync::mpsc::channel();
    app.lsp_events = Some(receiver);

    let _ = super::lsp::drain_lsp_events(&mut app);

    assert!(app.lsp_events.is_some());
}

#[test]
fn drain_lsp_events_handles_hover_completion_progress_and_log() {
    let mut app = app();
    app.active_preview_path = Some("/tmp/main.rs".to_string());
    install_lsp_events(
        &mut app,
        [
            crate::app::lsp::LspEvent::Hover { text: "documentation".to_string() },
            crate::app::lsp::LspEvent::Completion {
                items: vec!["Alpha".to_string(), "Beta".to_string()],
            },
            crate::app::lsp::LspEvent::Progress {
                token: "index".to_string(),
                server_key: "rust-analyzer".to_string(),
                title: "Indexing".to_string(),
                message: Some("workspace".to_string()),
                percentage: Some(25),
                done: false,
            },
            crate::app::lsp::LspEvent::Log {
                server_key: "rust-analyzer".to_string(),
                message: "ready".to_string(),
            },
        ],
    );

    let _ = super::lsp::drain_lsp_events(&mut app);

    assert!(app.lsp_overlay.hover_visible);
    assert_eq!(app.lsp_overlay.completion_items, vec!["Alpha", "Beta"]);
    assert_eq!(app.lsp_overlay_path.as_deref(), Some("/tmp/main.rs"));
    let progress = &app.lsp_progress["rust-analyzer"]["index"];
    assert_eq!(progress.title, "Indexing");
    assert_eq!(progress.message.as_deref(), Some("workspace"));
    assert_eq!(progress.percentage, Some(25));
    assert_eq!(app.lsp_status.as_deref(), Some("rust-analyzer Indexing"));
    assert_eq!(app.notifications.last().map(|item| item.message.as_str()), Some("LSP 日志: ready"));
    assert!(app.lsp_events.is_none());
}

#[test]
fn drain_lsp_events_clears_blank_hover_and_completed_progress() {
    let mut app = app();
    app.lsp_overlay.show_hover("stale".to_string());
    app.lsp_overlay_path = Some("/tmp/main.rs".to_string());
    app.lsp_progress.entry("rust-analyzer".to_string()).or_default().insert(
        "index".to_string(),
        crate::app::LspProgress {
            title: "Indexing".to_string(),
            message: None,
            percentage: Some(90),
        },
    );
    install_lsp_events(
        &mut app,
        [
            crate::app::lsp::LspEvent::Hover { text: " \n ".to_string() },
            crate::app::lsp::LspEvent::Progress {
                token: "index".to_string(),
                server_key: "rust-analyzer".to_string(),
                title: "Indexing".to_string(),
                message: None,
                percentage: Some(100),
                done: true,
            },
        ],
    );

    let _ = super::lsp::drain_lsp_events(&mut app);

    assert!(!app.lsp_overlay.hover_visible);
    assert!(app.lsp_overlay_path.is_none());
    assert!(!app.lsp_progress.contains_key("rust-analyzer"));
    assert_eq!(app.lsp_status.as_deref(), Some("rust-analyzer Ready"));
}

#[test]
fn drain_lsp_events_records_definition_with_decoded_file_path() {
    let mut app = app();
    install_lsp_events(
        &mut app,
        [crate::app::lsp::LspEvent::Definition {
            uri: "file:///tmp/my%20file.rs".to_string(),
            range: iced_code_editor::LspRange {
                start: iced_code_editor::LspPosition { line: 4, character: 7 },
                end: iced_code_editor::LspPosition { line: 4, character: 12 },
            },
        }],
    );

    let _ = super::lsp::drain_lsp_events(&mut app);

    assert_eq!(app.pending_preview_goto, Some(("/tmp/my file.rs".to_string(), 4, 7)));
}

#[test]
fn drain_lsp_events_ignores_definition_with_non_file_uri() {
    let mut app = app();
    install_lsp_events(
        &mut app,
        [crate::app::lsp::LspEvent::Definition {
            uri: "https://example.com/main.rs".to_string(),
            range: iced_code_editor::LspRange {
                start: iced_code_editor::LspPosition { line: 1, character: 2 },
                end: iced_code_editor::LspPosition { line: 1, character: 3 },
            },
        }],
    );

    let _ = super::lsp::drain_lsp_events(&mut app);

    assert!(app.pending_preview_goto.is_none());
}

#[test]
fn disable_lsp_detaches_state_and_marks_status() {
    let mut app = app();
    app.lsp_disabled = false;
    app.lsp_overlay.show_hover("hover".to_string());
    app.lsp_overlay.all_completions = vec!["item".to_string()];
    app.lsp_overlay.completion_items = vec!["item".to_string()];
    app.lsp_overlay.completion_visible = true;
    app.lsp_overlay_path = Some("/tmp/main.rs".to_string());
    app.lsp_progress.entry("rust-analyzer".to_string()).or_default().insert(
        "token".to_string(),
        crate::app::LspProgress { title: "index".to_string(), message: None, percentage: Some(10) },
    );
    let mut tab = preview_tab("/tmp/main.rs", "fn main() {}");
    tab.lsp_server_key = Some("rust-analyzer");
    tab.lsp_uri = Some("file:///tmp/main.rs".to_string());
    tab.lsp_language_id = Some("rust".to_string());
    app.preview_tabs.push(tab);

    super::lsp::disable_lsp(&mut app);

    assert!(app.lsp_disabled);
    assert!(!app.lsp_overlay.hover_visible);
    assert!(!app.lsp_overlay.completion_visible);
    assert!(app.lsp_progress.is_empty());
    assert_eq!(app.lsp_status.as_deref(), Some("LSP 已禁用"));
    assert!(app.preview_tabs[0].lsp_server_key.is_none());
}

#[test]
fn tick_when_disabled_only_advances_spinner() {
    let mut app = app();
    app.lsp_disabled = true;
    let initial = app.spinner_frame;

    let _ = super::lsp::tick(&mut app);

    assert_eq!(app.spinner_frame, initial.wrapping_add(1));
}

#[test]
fn tick_when_enabled_processes_lsp_events() {
    let mut app = app();
    app.lsp_disabled = false;
    app.active_preview_path = Some("/tmp/main.rs".to_string());
    install_lsp_events(
        &mut app,
        [crate::app::lsp::LspEvent::Hover { text: "hover docs".to_string() }],
    );
    let initial = app.spinner_frame;

    let _ = super::lsp::tick(&mut app);

    assert_eq!(app.spinner_frame, initial.wrapping_add(1));
    assert!(app.lsp_overlay.hover_visible);
    assert_eq!(app.lsp_overlay_path.as_deref(), Some("/tmp/main.rs"));
}

// 任务与结果：覆盖任务构造、调度或结果回写对公开状态的影响。
// 完成、失败与过期响应均按当前场景的既定契约处理。
#[test]
fn focus_and_completion_scroll_tasks_are_constructible() {
    let _focus = super::lsp::focus_hover_overlay();
    let _scroll_top = super::lsp::completion_scroll_task(0);
    let _scroll_offset = super::lsp::completion_scroll_task(7);
}
