// 验证 Git 差异高亮、行选择与范围选择的状态转换。
// 测试夹具围绕目标分支构造可控状态；跨模块行为由对应集成测试负责。

use super::{GitMessage, update};
use crate::app::{App, Message};
use iced_runtime::Action;
use iced_runtime::futures::futures::StreamExt;

async fn collect_task_messages(task: iced::Task<Message>) -> Vec<Message> {
    let Some(mut stream) = iced_runtime::task::into_stream(task.collect()) else {
        return Vec::new();
    };

    while let Some(action) = stream.next().await {
        if let Action::Output(messages) = action {
            return messages;
        }
    }

    Vec::new()
}

fn test_app() -> App {
    App::new().0
}

#[test]
fn preview_rejects_unsupported_file_without_external_io() {
    let mut app = test_app();
    app.git.git_diff_file_menu =
        Some(crate::app::state::GitDiffFileMenuState { file: "archive.zip".to_string() });

    let _ = update(&mut app, GitMessage::PreviewDiffFile("archive.zip".to_string()));

    assert!(app.git.git_diff_file_menu.is_none());
    assert_eq!(app.input_feedback.error_message.as_deref(), Some("不支持展开该文件：压缩包 文件无法打开"));
}

#[tokio::test(flavor = "multi_thread")]
async fn preview_and_copy_file_join_project_path_when_available() {
    let mut app = test_app();
    app.navigation_tabs.project_path = Some("/workspace/repo".to_string());
    app.git.git_diff_file_menu =
        Some(crate::app::state::GitDiffFileMenuState { file: "src/lib.rs".to_string() });

    let preview =
        collect_task_messages(update(&mut app, GitMessage::PreviewDiffFile("src/lib.rs".into())))
            .await;
    assert!(app.git.git_diff_file_menu.is_none());
    assert!(matches!(
        preview.as_slice(),
        [Message::Preview(crate::app::message::PreviewMessage::Open(path))]
            if path == "/workspace/repo/src/lib.rs"
    ));

    let copied = collect_task_messages(update(
        &mut app,
        GitMessage::CopyDiffFile { file: "src/lib.rs".into(), deleted_content: None },
    ))
    .await;
    assert!(matches!(
        copied.as_slice(),
        [Message::CopyFile(path)] if path == "/workspace/repo/src/lib.rs"
    ));
}

// 状态转换：覆盖交互前后的字段同步、选择关系与清理规则。
// 连续操作仍需保持模块不变量，并清除已经失效的瞬态反馈。

#[test]
fn fullscreen_toggles_reset_competing_chat_fullscreen_state() {
    let mut app = test_app();
    app.chat_interaction.chat_panel_fullscreen = true;
    app.chat_interaction.chat_panel_half_fullscreen = true;

    let _ = update(&mut app, GitMessage::ToggleFullscreen);

    assert!(app.git.git_diff_fullscreen);
    assert!(!app.git.git_diff_half_fullscreen);
    assert!(!app.chat_interaction.chat_panel_fullscreen);
    assert!(!app.chat_interaction.chat_panel_half_fullscreen);
    assert!(app.navigation_panels.show_diff);
    assert!(app.fullscreen_layout_settling);
}

#[test]
fn fullscreen_toggle_exits_without_reopening_diff() {
    let mut app = test_app();
    app.git.git_diff_fullscreen = true;
    app.navigation_panels.show_diff = false;

    let _ = update(&mut app, GitMessage::ToggleFullscreen);

    assert!(!app.git.git_diff_fullscreen);
    assert!(!app.git.git_diff_half_fullscreen);
    assert!(!app.navigation_panels.show_diff);
    assert!(app.fullscreen_layout_settling);
}

#[test]
fn half_fullscreen_enters_and_exits_while_disabling_fullscreen() {
    let mut app = test_app();
    app.git.git_diff_fullscreen = true;
    app.chat_interaction.chat_panel_fullscreen = true;
    app.chat_interaction.chat_panel_half_fullscreen = true;

    let _ = update(&mut app, GitMessage::ToggleHalfFullscreen);

    assert!(app.git.git_diff_half_fullscreen);
    assert!(!app.git.git_diff_fullscreen);
    assert!(!app.chat_interaction.chat_panel_fullscreen);
    assert!(!app.chat_interaction.chat_panel_half_fullscreen);
    assert!(app.navigation_panels.show_diff);
    assert!(app.fullscreen_layout_settling);

    let _ = update(&mut app, GitMessage::ToggleHalfFullscreen);
    assert!(!app.git.git_diff_half_fullscreen);
    assert!(!app.git.git_diff_fullscreen);
}

#[test]
fn focus_file_resets_preview_state_and_opens_diff() {
    let mut app = test_app();
    app.navigation_tabs.active_preview_path = Some("preview.rs".to_string());
    app.navigation_panels.show_diff = false;
    app.git.git_diff_file_menu =
        Some(crate::app::state::GitDiffFileMenuState { file: "old.rs".to_string() });

    let _ = update(&mut app, GitMessage::FocusFile("focused.rs".to_string()));

    assert_eq!(app.git.git_focused_file.as_deref(), Some("focused.rs"));
    assert!(app.is_diff_file_expanded("focused.rs"));
    assert!(app.navigation_tabs.active_preview_path.is_none());
    assert!(app.navigation_panels.show_diff);
    assert!(app.git.git_diff_file_menu.is_none());
}

// 状态转换：覆盖交互前后的字段同步、选择关系与清理规则。
// 连续操作仍需保持模块不变量，并清除已经失效的瞬态反馈。
