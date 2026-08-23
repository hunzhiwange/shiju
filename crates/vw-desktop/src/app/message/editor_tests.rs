// 验证编辑器设置、预览状态与输入事件的消息处理行为。
// 测试夹具围绕目标分支构造可控状态；跨模块行为由对应集成测试负责。

use super::editor::{EditorMessage, update};
use crate::app::components::editor::Editor;
use crate::app::{App, PreviewTab};
use iced::{Theme, widget::Id};
use iced_code_editor::{ArrowDirection, Message as CodeEditorMessage, i18n::Language};

fn test_app() -> App {
    App::new().0
}

fn preview_tab(path: &str, content: &str) -> PreviewTab {
    PreviewTab {
        path: path.to_string(),
        title: path.rsplit('/').next().unwrap_or(path).to_string(),
        content: content.to_string(),
        is_dirty: false,
        truncated: false,
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

    let _ = update(&mut app, EditorMessage::FontSizeChanged(20.0));

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

    let _ = update(&mut app, EditorMessage::FontSizeChanged(42.0));

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

    let _ = update(&mut app, EditorMessage::LineHeightChanged(5.0));

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
