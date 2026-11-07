// 验证编辑器设置、预览状态与输入事件的消息处理行为。
// 测试夹具围绕目标分支构造可控状态；跨模块行为由对应集成测试负责。

use super::editor::{EditorMessage, update};
use crate::app::components::editor::Editor;
use crate::app::{App, PreviewTab};
use iced::{Theme, widget::Id};
use iced_code_editor::{ArrowDirection, Message as CodeEditorMessage, i18n::Language};

fn test_app() -> App {
    let themes = tempfile::tempdir().unwrap();
    crate::app::theme_catalog::set_test_themes_root(Some(themes.path().to_path_buf()));
    let app = App::new().0;
    crate::app::theme_catalog::set_test_themes_root(None);
    app
}

fn preview_tab(path: &str, content: &str) -> PreviewTab {
    PreviewTab {
        path: path.to_string(),
        title: path.rsplit('/').next().unwrap_or(path).to_string(),
        content: content.to_string(),
        is_dirty: false,
        truncated: false,
        read_only: false,
        auto_save_revision: 0,
        editor: Editor::new(content, "text"),
        scroll_id: Id::unique(),
        #[cfg(not(target_arch = "wasm32"))]
        lsp_server_key: None,
        #[cfg(not(target_arch = "wasm32"))]
        lsp_uri: None,
        #[cfg(not(target_arch = "wasm32"))]
        lsp_language_id: None,
    }
}

fn commit_metrics_input(app: &mut App, message: EditorMessage) -> iced::Task<crate::app::Message> {
    let _ = update(app, message);
    update(app, EditorMessage::CommitMetrics)
}

fn app_with_active_preview(content: &str) -> App {
    let mut app = test_app();
    let path = "/tmp/editor-test.txt";
    app.active_preview_path = Some(path.to_string());
    app.preview_tabs = vec![preview_tab(path, content)];
    app
}

// 编译接线：引用父模块符号，确保测试片段仍由真实模块包含。
// 该冒烟用例只承担接线职责，具体行为由同领域测试覆盖。
#[test]
fn test_module_is_wired() {
    let module = module_path!();

    assert!(module.ends_with("editor_tests"));
}

#[test]
fn toggle_settings_flips_preview_settings_panel() {
    let mut app = test_app();
    let original = app.show_preview_settings;

    let _ = update(&mut app, EditorMessage::ToggleSettings);
    assert_eq!(app.show_preview_settings, !original);

    let _ = update(&mut app, EditorMessage::ToggleSettings);
    assert_eq!(app.show_preview_settings, original);
}

#[test]
fn font_size_changed_updates_auto_line_height() {
    let mut app = test_app();
    app.auto_adjust_line_height = true;
    app.preview_tabs = vec![preview_tab("/tmp/a.txt", "a")];

    let _ = commit_metrics_input(&mut app, EditorMessage::FontSizePreview(20.0));

    assert_eq!(app.current_font_size, 20.0);
    assert_eq!(app.current_line_height, 28.0);
    assert_eq!(app.preview_tabs[0].editor.inner.font_size(), 20.0);
    assert_eq!(app.preview_tabs[0].editor.inner.line_height(), 28.0);
    assert_eq!(app.git_copy_modal_code_editor.font_size(), 20.0);
    assert_eq!(app.git_copy_modal_code_editor.line_height(), 28.0);
}

#[test]
fn font_size_changed_without_auto_line_height_preserves_height_and_clamps_modal_font() {
    let mut app = test_app();
    app.auto_adjust_line_height = false;
    app.current_line_height = 24.0;
    app.preview_tabs = vec![preview_tab("/tmp/a.txt", "a")];
    let preview_line_height = app.preview_tabs[0].editor.inner.line_height();
    let modal_line_height = app.git_copy_modal_code_editor.line_height();

    let _ = commit_metrics_input(&mut app, EditorMessage::FontSizePreview(42.0));

    assert_eq!(app.current_font_size, 42.0);
    assert_eq!(app.current_line_height, 24.0);
    assert_eq!(app.preview_tabs[0].editor.inner.font_size(), 42.0);
    assert_eq!(app.preview_tabs[0].editor.inner.line_height(), preview_line_height);
    assert_eq!(app.git_copy_modal_code_editor.font_size(), 30.0);
    assert_eq!(app.git_copy_modal_code_editor.line_height(), modal_line_height);
}

#[test]
fn line_height_changed_disables_auto_adjustment() {
    let mut app = test_app();
    app.auto_adjust_line_height = true;
    app.preview_tabs = vec![preview_tab("/tmp/a.txt", "a")];

    let _ = commit_metrics_input(&mut app, EditorMessage::LineHeightPreview(5.0));

    assert_eq!(app.current_line_height, 5.0);
    assert!(!app.auto_adjust_line_height);
    assert_eq!(app.preview_tabs[0].editor.inner.line_height(), 10.0);
    assert_eq!(app.git_copy_modal_code_editor.line_height(), 10.0);
}

#[test]
fn toggle_auto_line_height_recomputes_when_enabled() {
    let mut app = test_app();
    app.current_font_size = 15.0;
    app.current_line_height = 19.0;
    app.auto_adjust_line_height = false;
    app.preview_tabs = vec![preview_tab("/tmp/a.txt", "a")];

    let _ = update(&mut app, EditorMessage::ToggleAutoLineHeight(true));

    assert!(app.auto_adjust_line_height);
    assert_eq!(app.current_line_height, 21.0);
    assert_eq!(app.preview_tabs[0].editor.inner.line_height(), 21.0);
    assert_eq!(app.git_copy_modal_code_editor.line_height(), 21.0);
}

// 状态转换：覆盖交互前后的字段同步、选择关系与清理规则。
// 连续操作仍需保持模块不变量，并清除已经失效的瞬态反馈。
#[test]
fn toggle_auto_line_height_off_keeps_current_height() {
    let mut app = test_app();
    app.current_font_size = 18.0;
    app.current_line_height = 25.0;
    app.preview_tabs = vec![preview_tab("/tmp/a.txt", "a")];
    let preview_line_height = app.preview_tabs[0].editor.inner.line_height();
    let modal_line_height = app.git_copy_modal_code_editor.line_height();

    let _ = update(&mut app, EditorMessage::ToggleAutoLineHeight(false));

    assert!(!app.auto_adjust_line_height);
    assert_eq!(app.current_line_height, 25.0);
    assert_eq!(app.preview_tabs[0].editor.inner.line_height(), preview_line_height);
    assert_eq!(app.git_copy_modal_code_editor.line_height(), modal_line_height);
}

#[test]
fn language_changed_updates_current_language() {
    let mut app = test_app();

    let _ = update(&mut app, EditorMessage::LanguageChanged(Language::English));

    assert_eq!(app.current_language, Language::English);
}

#[test]
fn language_changed_propagates_to_every_preview_editor() {
    let mut app = test_app();
    app.preview_tabs = vec![preview_tab("/tmp/a.txt", "a"), preview_tab("/tmp/b.txt", "b")];

    let _ = update(&mut app, EditorMessage::LanguageChanged(Language::English));

    assert!(app.preview_tabs.iter().all(|tab| tab.editor.inner.language() == Language::English));
    assert_eq!(app.git_copy_modal_code_editor.language(), Language::English);
}

#[test]
fn theme_changed_disables_follow_system_theme() {
    let mut app = test_app();
    app.editor_follow_system_theme = true;

    let _ = update(&mut app, EditorMessage::ThemeChanged(Theme::Dark));

    assert_eq!(app.editor_theme, Theme::Dark);
    assert!(!app.editor_follow_system_theme);
}

#[test]
fn toggle_follow_system_theme_updates_flag() {
    let mut app = test_app();
    app.editor_follow_system_theme = false;

    let _ = update(&mut app, EditorMessage::ToggleFollowSystemTheme(true));

    assert!(app.editor_follow_system_theme);
}

#[test]
fn editor_actions_without_active_preview_are_noops() {
    let mut app = test_app();
    app.active_preview_path = None;

    for message in [
        EditorMessage::OpenSearch,
        EditorMessage::OpenReplace,
        EditorMessage::CloseSearch,
        EditorMessage::Undo,
        EditorMessage::Redo,
        EditorMessage::Copy,
        EditorMessage::Cut,
        EditorMessage::ClipboardContentReceived(Some("text".to_string())),
        EditorMessage::ClipboardContentReceived(None),
        EditorMessage::Delete,
        EditorMessage::FontChanged("Monospace".to_string()),
    ] {
        let _ = update(&mut app, message);
    }

    assert_eq!(app.active_preview_path, None);
}

// 失败与边界：覆盖非法、缺失、过期或冲突场景的既定处理规则。
// 重点确认现有状态与错误反馈不会被意外的部分更新污染。
#[test]
fn editor_actions_with_missing_active_tab_leave_other_tabs_unchanged() {
    let mut app = test_app();
    app.active_preview_path = Some("/tmp/missing.txt".to_string());
    app.preview_tabs = vec![preview_tab("/tmp/other.txt", "keep")];

    for message in [
        EditorMessage::OpenSearch,
        EditorMessage::OpenReplace,
        EditorMessage::CloseSearch,
        EditorMessage::Undo,
        EditorMessage::Redo,
        EditorMessage::Copy,
        EditorMessage::Cut,
        EditorMessage::ClipboardContentReceived(Some("ignored".to_string())),
        EditorMessage::Delete,
    ] {
        let _ = update(&mut app, message);
    }

    assert_eq!(app.preview_tabs[0].editor.content(), "keep");
}

#[test]
fn clipboard_content_undo_and_redo_mutate_only_the_active_editor() {
    let mut app = app_with_active_preview("tail");
    app.preview_tabs.push(preview_tab("/tmp/other.txt", "other"));

    let _ = update(&mut app, EditorMessage::ClipboardContentReceived(Some("head ".to_string())));
    assert_eq!(app.preview_tabs[0].editor.content(), "head tail");
    assert_eq!(app.preview_tabs[1].editor.content(), "other");

    let _ = update(&mut app, EditorMessage::Undo);
    assert_eq!(app.preview_tabs[0].editor.content(), "tail");

    let _ = update(&mut app, EditorMessage::Redo);
    assert_eq!(app.preview_tabs[0].editor.content(), "head tail");
}

#[test]
fn cut_and_delete_remove_the_active_editor_selection() {
    for message in [EditorMessage::Cut, EditorMessage::Delete] {
        let mut app = app_with_active_preview("abcd");
        let editor = &mut app.preview_tabs[0].editor.inner;
        let _ = editor.update(&CodeEditorMessage::GotoPosition(0, 0));
        let _ = editor.update(&CodeEditorMessage::ArrowKey(ArrowDirection::Right, true));
        let _ = editor.update(&CodeEditorMessage::ArrowKey(ArrowDirection::Right, true));

        let _ = update(&mut app, message);

        assert_eq!(app.preview_tabs[0].editor.content(), "cd");
    }
}

#[test]
fn copy_preserves_the_active_editor_content() {
    let mut app = app_with_active_preview("copy me");
    let editor = &mut app.preview_tabs[0].editor.inner;
    let _ = editor.update(&CodeEditorMessage::GotoPosition(0, 0));
    let _ = editor.update(&CodeEditorMessage::ArrowKey(ArrowDirection::Right, true));

    let _ = update(&mut app, EditorMessage::Copy);

    assert_eq!(app.preview_tabs[0].editor.content(), "copy me");
}

#[test]
fn paste_request_and_empty_clipboard_result_preserve_editor_content() {
    let mut app = app_with_active_preview("keep");

    let _ = update(&mut app, EditorMessage::Paste);
    let _ = update(&mut app, EditorMessage::ClipboardContentReceived(None));

    assert_eq!(app.preview_tabs[0].editor.content(), "keep");
}

// 失败与边界：覆盖非法、缺失、过期或冲突场景的既定处理规则。
// 重点确认现有状态与错误反馈不会被意外的部分更新污染。
#[test]
fn search_dialog_actions_preserve_editor_content() {
    let mut app = app_with_active_preview("find me");

    let _ = update(&mut app, EditorMessage::OpenSearch);
    assert_eq!(app.preview_tabs[0].editor.content(), "find me");

    let _ = update(&mut app, EditorMessage::OpenReplace);
    assert_eq!(app.preview_tabs[0].editor.content(), "find me");

    let _ = update(&mut app, EditorMessage::CloseSearch);
    assert_eq!(app.preview_tabs[0].editor.content(), "find me");
}

#[cfg(not(target_arch = "wasm32"))]
async fn metric_deadline(task: iced::Task<crate::app::Message>) -> EditorMessage {
    use crate::app::Message;
    use iced_runtime::futures::futures::StreamExt;

    let mut actions = iced_runtime::task::into_stream(task).expect("slider must schedule a commit");
    let Some(iced_runtime::Action::Output(Message::Editor(
        message @ EditorMessage::CommitMetricsDue(_),
    ))) = actions.next().await
    else {
        panic!("expected a scheduled metrics commit");
    };
    message
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn editor_metrics_preview_commits_on_latest_deadline() {
    let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
    let mut app = app_with_active_preview("hello");
    app.auto_adjust_line_height = true;
    let previous = app.preview_tabs[0].editor.inner.font_size();
    let first = update(&mut app, EditorMessage::FontSizePreview(18.0));
    let last = update(&mut app, EditorMessage::FontSizePreview(20.0));
    let (stale, latest) =
        runtime.block_on(async { tokio::join!(metric_deadline(first), metric_deadline(last)) });

    assert!(iced_runtime::task::into_stream(update(&mut app, stale)).is_none());
    assert_eq!(app.preview_tabs[0].editor.inner.font_size(), previous);

    let _ = update(&mut app, latest);
    assert_eq!(app.preview_tabs[0].editor.inner.font_size(), 20.0);
    assert_eq!(app.preview_tabs[0].editor.inner.line_height(), 28.0);
    assert!(app.auto_adjust_line_height);
    assert!(
        iced_runtime::task::into_stream(update(&mut app, EditorMessage::CommitMetrics)).is_none()
    );
}

#[test]
fn editor_metrics_preview_preserves_latest_input_when_toggling_auto_or_closing() {
    let mut app = app_with_active_preview("hello");
    app.show_preview_settings = true;
    app.auto_adjust_line_height = true;

    let _ = update(&mut app, EditorMessage::FontSizePreview(20.0));
    let _ = update(&mut app, EditorMessage::LineHeightPreview(33.0));
    assert!(!super::editor::metrics::displayed(&app).auto_line_height);
    let _ = update(&mut app, EditorMessage::ToggleAutoLineHeight(true));
    assert_eq!(app.current_font_size, 20.0);
    assert_eq!(app.current_line_height, 28.0);
    assert!(app.auto_adjust_line_height);

    let _ = update(&mut app, EditorMessage::FontSizePreview(25.0));
    let _ = update(&mut app, EditorMessage::ToggleAutoLineHeight(false));
    assert_eq!(app.current_line_height, 35.0);
    assert_eq!(app.preview_tabs[0].editor.inner.line_height(), 35.0);
    assert!(!app.auto_adjust_line_height);

    let _ = update(&mut app, EditorMessage::LineHeightPreview(39.0));
    let _ = update(&mut app, EditorMessage::ToggleSettings);
    assert!(!app.show_preview_settings);
    assert_eq!(app.current_line_height, 39.0);
    assert_eq!(app.preview_tabs[0].editor.inner.line_height(), 39.0);
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn editor_metrics_preview_commits_latest_once() {
    use crate::app::message::preview::{self, PreviewMessage};
    use crate::app::{Message, config};
    use iced_runtime::futures::futures::StreamExt;
    use serde_json::json;
    use std::time::{Duration, Instant};

    const TIMEOUT: Duration = Duration::from_secs(5);

    let mut evidence = Vec::new();
    for (sample, tabs, history) in
        [("empty", 0, false), ("multi-tab", 8, false), ("history", 8, true)]
    {
        config::with_editor_settings_gateway(|mut app, mut requests, responses| {
            let runtime =
                tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
            app.auto_adjust_line_height = true;
            for index in 0..tabs {
                app.preview_tabs
                    .push(preview_tab(&format!("/tmp/editor-{index}.rs"), "fn main() {}"));
            }
            let history_content = "历史版本的只读内容 abcdefghijklmnopqrstuvwxyz\n".repeat(20_000);
            if history {
                let mut tab = preview_tab("/tmp/history.rs", &history_content);
                tab.read_only = true;
                app.preview_tabs.push(tab);
            }
            app.active_preview_path = app.preview_tabs.first().map(|tab| tab.path.clone());
            let target = app.preview_tabs.last().map(|tab| tab.path.clone());
            let read_metrics = |app: &App| {
                app.preview_tabs
                    .iter()
                    .map(|tab| &tab.editor.inner)
                    .chain(std::iter::once(&app.git_copy_modal_code_editor))
                    .map(|editor| (editor.font_size(), editor.line_height()))
                    .collect::<Vec<_>>()
            };

            let original = read_metrics(&app);
            let mut previous = original.clone();
            let mut save = None;
            let mut update_times = Vec::new();
            let mut input_layouts = 0;
            let mut debounce_tasks = Vec::new();
            for message in [
                EditorMessage::FontSizePreview(18.0),
                EditorMessage::LineHeightPreview(28.0),
                EditorMessage::FontSizePreview(20.0),
                EditorMessage::LineHeightPreview(31.0),
                EditorMessage::FontSizePreview(24.0),
                EditorMessage::LineHeightPreview(34.0),
            ] {
                let start = Instant::now();
                let task = update(&mut app, message);
                update_times.push(start.elapsed());
                let current = read_metrics(&app);
                input_layouts += usize::from(current != previous);
                previous = current;
                if save.is_none() && app.editor_config_persistence.wait_for_idle().is_some() {
                    save = Some(task);
                } else {
                    debounce_tasks.push(task);
                }
            }
            let displayed = super::editor::metrics::displayed(&app);
            assert_eq!(
                (displayed.font_size, displayed.line_height, displayed.auto_line_height),
                (24.0, 34.0, false)
            );
            let original_until_release = read_metrics(&app) == original;
            let save_before_release = save.is_some();
            let start = Instant::now();
            let release = update(&mut app, EditorMessage::CommitMetrics);
            let release_time = start.elapsed();
            let release_layouts = usize::from(read_metrics(&app) != previous);
            if save.is_none() && app.editor_config_persistence.wait_for_idle().is_some() {
                save = Some(release);
            }
            assert_eq!(read_metrics(&app), vec![(24.0, 34.0); original.len()]);
            assert!(!app.auto_adjust_line_height);
            let deadlines = debounce_tasks.split_off(debounce_tasks.len() - 2);
            let deadlines = runtime.block_on(async {
                let messages = deadlines.into_iter().map(metric_deadline);
                iced_runtime::futures::futures::future::join_all(messages).await
            });
            for deadline in deadlines {
                let stale = update(&mut app, deadline);
                assert!(iced_runtime::task::into_stream(stale).is_none());
            }
            assert!(
                iced_runtime::task::into_stream(update(&mut app, EditorMessage::CommitMetrics))
                    .is_none()
            );
            assert_eq!(read_metrics(&app), vec![(24.0, 34.0); original.len()]);

            let mut patch_count = 0;
            let mut last_settings = None;
            let mut max_patch_time = Duration::ZERO;
            while let Some(task) = save.take() {
                let start = Instant::now();
                let mut actions = iced_runtime::task::into_stream(task).expect("save task");
                let result = runtime.block_on(async {
                    let capture = tokio::task::spawn_blocking(move || {
                        let request = requests.recv_timeout(TIMEOUT).expect("PATCH request");
                        (requests, request)
                    });
                    tokio::pin!(capture);
                    let (returned_requests, request) = tokio::select! {
                        request = &mut capture => request.unwrap(),
                        _ = actions.next() => panic!("save completed before the PATCH arrived"),
                    };
                    assert!(request.starts_with("PATCH /v1/global/config "));
                    let patch: serde_json::Value =
                        serde_json::from_str(request.split_once("\r\n\r\n").unwrap().1).unwrap();
                    last_settings = Some(patch["app_ui"]["system_settings"].clone());
                    patch_count += 1;

                    if let Some(path) = &target {
                        let _ = preview::update(&mut app, PreviewMessage::Select(path.clone()));
                        assert_eq!(app.active_preview_path.as_ref(), Some(path));
                    }
                    tokio::time::sleep(Duration::from_millis(100)).await;
                    responses.send(Some("{}".to_string())).unwrap();
                    let action = tokio::time::timeout(TIMEOUT, actions.next()).await.unwrap();
                    (returned_requests, action)
                });
                max_patch_time = max_patch_time.max(start.elapsed());
                requests = result.0;
                let Some(iced_runtime::Action::Output(Message::Editor(
                    EditorMessage::ConfigSaved(Ok(())),
                ))) = result.1
                else {
                    panic!("expected successful editor save");
                };
                let next = update(&mut app, EditorMessage::ConfigSaved(Ok(())));
                if app.editor_config_persistence.wait_for_idle().is_some() {
                    save = Some(next);
                }
            }
            assert_eq!(
                last_settings,
                Some(json!({
                    "editor_font_size": 24.0,
                    "editor_line_height": 34.0,
                    "editor_auto_line_height": false,
                }))
            );
            if history {
                assert!(app.preview_tabs.last().unwrap().read_only);
                assert_eq!(app.preview_tabs.last().unwrap().editor.content(), history_content);
            }
            eprintln!(
                "{sample}: max input update {:?}, release {release_time:?}, input/release layouts {input_layouts}/{release_layouts}, PATCHes {patch_count}, max PATCH {max_patch_time:?}",
                update_times.into_iter().max().unwrap(),
            );
            evidence.push((
                original_until_release,
                save_before_release,
                input_layouts,
                release_layouts,
                patch_count,
            ));
        });
    }
    assert_eq!(evidence, vec![(true, false, 0, 1, 1); 3]);
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn editor_settings_save_ignores_gateway_delay_and_persists_latest() {
    use crate::app::{Message, config};
    use iced_runtime::futures::futures::StreamExt;
    use serde_json::json;
    use std::time::Duration;

    const CLEANUP_TIMEOUT: Duration = Duration::from_secs(5);

    fn run_task(
        runtime: &tokio::runtime::Runtime,
        task: iced::Task<Message>,
    ) -> tokio::task::JoinHandle<Message> {
        let mut actions = iced_runtime::task::into_stream(task)
            .expect("editor update must return a background save task");
        runtime.spawn(async move {
            while let Some(action) = actions.next().await {
                if let iced_runtime::Action::Output(message) = action {
                    return message;
                }
            }
            panic!("save task did not report completion");
        })
    }

    fn finish(
        runtime: &tokio::runtime::Runtime,
        task: tokio::task::JoinHandle<Message>,
        app: &mut App,
    ) -> iced::Task<Message> {
        let message = runtime
            .block_on(async { tokio::time::timeout(CLEANUP_TIMEOUT, task).await })
            .expect("save task must finish")
            .unwrap();
        let Message::Editor(message) = message else {
            panic!("save task must report an editor message");
        };
        update(app, message)
    }

    fn patch(request: &str) -> serde_json::Value {
        assert!(request.starts_with("PATCH /v1/global/config "));
        let body: serde_json::Value =
            serde_json::from_str(request.split_once("\r\n\r\n").unwrap().1).unwrap();
        let settings = body["app_ui"]["system_settings"].clone();
        assert_eq!(body, json!({"app_ui": {"system_settings": settings}}));
        settings
    }

    config::with_editor_settings_gateway(|mut app, requests, responses| {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(1)
            .enable_all()
            .build()
            .unwrap();
        app.auto_adjust_line_height = true;
        let first = run_task(
            &runtime,
            commit_metrics_input(&mut app, EditorMessage::FontSizePreview(20.0)),
        );
        let first_patch = patch(&requests.recv_timeout(CLEANUP_TIMEOUT).unwrap());
        assert_eq!(
            first_patch,
            json!({
                "editor_font_size": 20.0,
                "editor_line_height": 28.0,
                "editor_auto_line_height": true
            })
        );

        // All these updates finish before the first HTTP response is released.
        let _ = update(&mut app, EditorMessage::FontSizePreview(24.0));
        assert!(
            iced_runtime::task::into_stream(commit_metrics_input(
                &mut app,
                EditorMessage::LineHeightPreview(27.0)
            ))
            .is_none()
        );
        for message in
            [EditorMessage::ThemeChanged(Theme::Dark), EditorMessage::ToggleFollowSystemTheme(true)]
        {
            assert!(iced_runtime::task::into_stream(update(&mut app, message)).is_none());
        }
        assert_eq!(app.current_font_size, 24.0);
        assert_eq!(app.current_line_height, 27.0);
        assert!(app.editor_follow_system_theme);

        let mut drain = app.editor_config_persistence.wait_for_idle().unwrap();
        assert!(matches!(drain.try_recv(), Err(tokio::sync::oneshot::error::TryRecvError::Empty)));
        responses.send(Some("{}".to_string())).unwrap();
        let second = run_task(&runtime, finish(&runtime, first, &mut app));
        let second_patch = patch(&requests.recv_timeout(CLEANUP_TIMEOUT).unwrap());
        assert_eq!(
            second_patch,
            json!({
                "editor_font_size": 24.0,
                "editor_line_height": 27.0,
                "editor_auto_line_height": false,
                "editor_theme": "Dark",
                "editor_follow_system_theme": true
            })
        );
        assert!(matches!(drain.try_recv(), Err(tokio::sync::oneshot::error::TryRecvError::Empty)));
        responses.send(Some("{}".to_string())).unwrap();
        assert!(iced_runtime::task::into_stream(finish(&runtime, second, &mut app)).is_none());
        assert_eq!(drain.try_recv(), Ok(()));

        // The loopback fixture merges the captured patches, then answers a real settings reload.
        let mut persisted = json!({"terminal_font_size": 19.0});
        persisted.as_object_mut().unwrap().extend(first_patch.as_object().unwrap().clone());
        persisted.as_object_mut().unwrap().extend(second_patch.as_object().unwrap().clone());
        let reload = runtime.spawn(config::load_system_settings_config_async());
        assert!(
            requests.recv_timeout(CLEANUP_TIMEOUT).unwrap().starts_with("GET /v1/global/config ")
        );
        responses
            .send(Some(json!({"app_ui": {"system_settings": persisted}}).to_string()))
            .unwrap();
        let loaded = runtime.block_on(reload).unwrap().unwrap();
        assert_eq!(loaded.editor_font_size, 24.0);
        assert_eq!(loaded.editor_line_height, 27.0);
        assert!(!loaded.editor_auto_line_height);
        assert_eq!(loaded.editor_theme, "Dark");
        assert!(loaded.editor_follow_system_theme);
        assert_eq!(loaded.terminal_font_size, 19.0);

        let failed =
            run_task(&runtime, update(&mut app, EditorMessage::ToggleFollowSystemTheme(false)));
        assert_eq!(
            patch(&requests.recv_timeout(CLEANUP_TIMEOUT).unwrap()),
            json!({"editor_follow_system_theme": false})
        );
        responses.send(None).unwrap();
        let _ = finish(&runtime, failed, &mut app);
        assert!(!app.editor_follow_system_theme);
        let toast = app.active_toast.as_ref().expect("save failure must show a toast");
        assert!(toast.message.contains("编辑器配置保存失败"));

        let retry =
            run_task(&runtime, update(&mut app, EditorMessage::ToggleFollowSystemTheme(true)));
        assert_eq!(
            patch(&requests.recv_timeout(CLEANUP_TIMEOUT).unwrap()),
            json!({"editor_follow_system_theme": true})
        );
        responses.send(Some("{}".to_string())).unwrap();
        let _ = finish(&runtime, retry, &mut app);
    });
}
