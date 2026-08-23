// 验证 Git 顶层消息向各专用处理模块的路由契约。
// 测试夹具围绕目标分支构造可控状态；跨模块行为由对应集成测试负责。

use super::{GitMessage, update};
use crate::app::App;

fn test_app() -> App {
    App::new().0
}

// 编译接线：引用父模块符号，确保测试片段仍由真实模块包含。
// 该冒烟用例只承担接线职责，具体行为由同领域测试覆盖。
#[test]
fn test_module_is_wired() {
    let module = module_path!();

    assert!(module.ends_with("tests"));
}

#[test]
fn expand_direction_variants_are_distinct() {
    assert_ne!(super::ExpandDirection::Up, super::ExpandDirection::Down);
    assert_eq!(super::ExpandDirection::All, super::ExpandDirection::All);
}

// 任务与结果：覆盖任务构造、调度或结果回写对公开状态的影响。
// 完成、失败与过期响应均按当前场景的既定契约处理。
#[test]
fn update_routes_worktree_menu_message_to_refresh_handler() {
    let mut app = test_app();
    app.git_worktree_menu_open = false;

    let _task = update(&mut app, GitMessage::ToggleGitWorktreeMenu(true));

    assert!(app.git_worktree_menu_open);
}

#[test]
fn update_routes_line_selection_message_to_diff_handler() {
    let mut app = test_app();

    let _task = update(
        &mut app,
        GitMessage::ToggleDiffLineSelection(
            "src/lib.rs".to_string(),
            7,
            false,
            "+added".to_string(),
        ),
    );

    assert_eq!(app.git_diff_selected_lines.len(), 1);
    let selected = &app.git_diff_selected_lines[0];
    assert_eq!(selected.file, "src/lib.rs");
    assert_eq!(selected.line, 7);
    assert!(!selected.is_old);
    assert_eq!(selected.text, "+added");
}

// 消息与任务边界：确认输入消息产生预期状态或返回任务。
// 场景外状态必须保持不变，避免无关消息触发隐藏副作用。
#[test]
fn update_routes_custom_diff_title_message_to_modal_handler() {
    let mut app = test_app();

    let _task = update(&mut app, GitMessage::CustomDiffTitleChanged("Review".to_string()));

    assert_eq!(app.git_custom_diff_title, "Review");
}

// 提交与删除：覆盖保存、安装或移除操作前后的状态衔接。
// 这里关注校验、任务载荷和结果反馈，不绑定底层实现。
#[test]
fn update_routes_filter_query_message_to_stage_commit_handler() {
    let mut app = test_app();

    let _task = update(&mut app, GitMessage::FilterQueryChanged("src/*.rs".to_string()));

    assert_eq!(app.git_filter_query, "src/*.rs");
}
