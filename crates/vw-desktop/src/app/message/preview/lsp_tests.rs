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
        read_only: false,
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

// 宿主契约：核对 App 字段投影及 drain_lsp_events 的消息/任务接线。
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

// 宿主契约：核对 App 字段投影及 detach_lsp_for_tab 的消息/任务接线。
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




















// 数据边界：覆盖规范化、默认值以及相关状态字段的既定规则。
// 这里固定可观察结果，避免依赖处理过程中的临时实现细节。


























// 失败与边界：覆盖非法、缺失、过期或冲突场景的既定处理规则。
// 重点确认现有状态与错误反馈不会被意外的部分更新污染。










// 宿主契约：核对 App 字段投影及 drain_lsp_events 的消息/任务接线。
#[test]
fn drain_lsp_events_without_receiver_returns_none_and_preserves_state() {
    let mut app = app();
    app.lsp_status = Some("unchanged".to_string());
    app.lsp_events = None;

    let _ = super::lsp::drain_lsp_events(&mut app);

    assert_eq!(app.lsp_status.as_deref(), Some("unchanged"));
    assert!(app.lsp_events.is_none());
}

// 宿主契约：核对 App 字段投影及 drain_lsp_events 的消息/任务接线。
#[test]
fn drain_lsp_events_retains_empty_receiver_when_channel_is_open() {
    let mut app = app();
    let (_sender, receiver) = std::sync::mpsc::channel();
    app.lsp_events = Some(receiver);

    let _ = super::lsp::drain_lsp_events(&mut app);

    assert!(app.lsp_events.is_some());
}

// 宿主契约：核对 App 字段投影及 drain_lsp_events 的消息/任务接线。
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

// 宿主契约：核对 App 字段投影及 drain_lsp_events 的消息/任务接线。
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
    assert_eq!(app.lsp_status.as_deref(), Some("rust-analyzer 就绪"));
}

// 宿主契约：核对 App 字段投影及 drain_lsp_events 的消息/任务接线。
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

// 宿主契约：核对 App 字段投影及 drain_lsp_events 的消息/任务接线。
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



// 宿主契约：核对 App 字段投影及 tick 的消息/任务接线。
#[test]
fn tick_when_disabled_only_advances_spinner() {
    let mut app = app();
    app.lsp_disabled = true;
    let initial = app.spinner_frame;

    let _ = super::lsp::tick(&mut app);

    assert_eq!(app.spinner_frame, initial.wrapping_add(1));
}

// 宿主契约：核对 App 字段投影及 tick 的消息/任务接线。
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
// 宿主契约：核对 App 字段投影及 completion_scroll_task, focus_hover_overlay 的消息/任务接线。
#[test]
fn focus_and_completion_scroll_tasks_are_constructible() {
    let _focus = super::lsp::focus_hover_overlay();
    let _scroll_top = super::lsp::completion_scroll_task(0);
    let _scroll_offset = super::lsp::completion_scroll_task(7);
}
