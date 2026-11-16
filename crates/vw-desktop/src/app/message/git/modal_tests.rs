// 验证自定义差异与聊天文本差异弹窗的编辑和显隐状态。
// 测试夹具围绕目标分支构造可控状态；跨模块行为由对应集成测试负责。

use super::{GitMessage, update};
use crate::app::App;

fn test_app() -> App {
    App::new().0
}

// 编译接线：引用父模块符号，确保测试片段仍由真实模块包含。
// 该冒烟用例只承担接线职责，具体行为由同领域测试覆盖。

#[test]
fn custom_diff_result_hides_inputs_and_shows_diff_view() {
    let mut app = test_app();
    app.navigation_tabs.active_preview_path = Some("/tmp/a.rs".to_string());
    app.navigation_panels.show_diff = false;

    let _ = update(
        &mut app,
        GitMessage::OpenCustomDiffResult {
            title: "Patch".to_string(),
            before: "old".to_string(),
            after: "new".to_string(),
        },
    );

    assert!(app.git.show_git_custom_diff_modal);
    assert!(app.git.git_custom_diff_hide_inputs);
    assert_eq!(app.git.git_custom_diff_title, "Patch");
    assert_eq!(app.git.git_custom_diff_before_editor.text(), "old");
    assert_eq!(app.git.git_custom_diff_after_editor.text(), "new");
    assert_eq!(app.navigation_tabs.active_preview_path, None);
    assert!(app.navigation_panels.show_diff);
}

#[test]
fn chat_text_diff_open_and_close_update_panel_state() {
    let mut app = test_app();
    app.git.chat_text_diff = Some(crate::app::state::ChatTextDiff {
        title: "stale".to_string(),
        file: "stale.rs".to_string(),
        before: "old".to_string(),
        after: "new".to_string(),
    });
    app.navigation_tabs.active_preview_path = Some("preview.rs".to_string());

    let _ = update(
        &mut app,
        GitMessage::OpenChatTextDiff {
            title: "Chat patch".to_string(),
            file: "main.rs".to_string(),
            before: "a".to_string(),
            after: "b".to_string(),
        },
    );

    assert!(app.git.chat_text_diff.is_none());
    assert!(app.git.show_git_custom_diff_modal);
    assert!(app.git.git_custom_diff_hide_inputs);
    assert_eq!(app.git.git_custom_diff_title, "Chat patch");
    assert_eq!(app.git.git_custom_diff_before_editor.text(), "a");
    assert_eq!(app.git.git_custom_diff_after_editor.text(), "b");
    assert!(app.navigation_panels.show_diff);
    assert!(app.file_manager.file_manager_show_changes);
    assert!(app.navigation_tabs.active_preview_path.is_none());

    let _ = update(&mut app, GitMessage::CloseChatTextDiff);
    assert!(app.git.chat_text_diff.is_none());
    assert!(!app.git.show_git_custom_diff_modal);
    assert!(!app.git.git_custom_diff_hide_inputs);
}

// 状态转换：覆盖交互前后的字段同步、选择关系与清理规则。
// 连续操作仍需保持模块不变量，并清除已经失效的瞬态反馈。

// 数据边界：覆盖规范化、默认值以及相关状态字段的既定规则。
// 这里固定可观察结果，避免依赖处理过程中的临时实现细节。

// 行为契约：使用代表性场景固定该分支的可观察结果。
// 断言聚焦输入、状态与返回任务，不复述内部控制流程。
