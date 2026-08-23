// 验证项目顶层更新函数向各领域处理器正确分派消息。
// 测试夹具围绕目标分支构造可控状态；跨模块行为由对应集成测试负责。

use super::{ProjectMessage, update};
use crate::app::{App, state::FindInFolderTab};

fn find_tab(id: &str) -> FindInFolderTab {
    FindInFolderTab {
        id: id.to_string(),
        title: "查找".to_string(),
        scope_path: "/tmp/project".to_string(),
        query_input: String::new(),
        replace_input: String::new(),
        query_editor: iced::widget::text_editor::Content::new(),
        replace_editor: iced::widget::text_editor::Content::new(),
        query: String::new(),
        replace_text: String::new(),
        case_sensitive: false,
        whole_word: false,
        use_regex: false,
        running: false,
        error: None,
        limit_reached: false,
        matches: Vec::new(),
        results_page: 0,
    }
}

// 消息与任务边界：确认输入消息产生预期状态或返回任务。
// 场景外状态必须保持不变，避免无关消息触发隐藏副作用。
#[test]
fn update_dispatches_basic_message() {
    let (mut app, _task) = App::new();

    let _task = update(&mut app, ProjectMessage::ProjectPathChanged("/tmp/project".to_string()));

    assert_eq!(app.project_path_input, "/tmp/project");
}

// 消息与任务边界：确认输入消息产生预期状态或返回任务。
// 场景外状态必须保持不变，避免无关消息触发隐藏副作用。
#[test]
fn update_dispatches_find_message() {
    let (mut app, _task) = App::new();
    app.find_results_tabs.push(find_tab("find-tab"));

    let _task = update(
        &mut app,
        ProjectMessage::FileTreeFindCaseSensitiveToggled("find-tab".to_string(), true),
    );

    assert!(app.find_results_tabs[0].case_sensitive);
}

// 消息与任务边界：确认输入消息产生预期状态或返回任务。
// 场景外状态必须保持不变，避免无关消息触发隐藏副作用。
#[test]
fn update_dispatches_file_tree_message() {
    let (mut app, _task) = App::new();

    let _task = update(
        &mut app,
        ProjectMessage::FileTreeRightClicked(
            "/tmp/project/src/main.rs".to_string(),
            "tree".to_string(),
            12.0,
            24.0,
        ),
    );

    assert_eq!(app.file_tree_menu_path.as_deref(), Some("/tmp/project/src/main.rs"));
    assert_eq!(app.file_tree_menu_source.as_deref(), Some("tree"));
    assert_eq!(app.file_tree_menu_anchor, Some(iced::Point::new(12.0, 24.0)));
}

// 消息与任务边界：确认输入消息产生预期状态或返回任务。
// 场景外状态必须保持不变，避免无关消息触发隐藏副作用。
#[test]
fn update_dispatches_session_message() {
    let (mut app, _task) = App::new();

    let _task =
        update(&mut app, ProjectMessage::SessionRightClicked("session-id".to_string(), 8.0, 16.0));

    assert_eq!(app.session_menu_id.as_deref(), Some("session-id"));
    assert_eq!(app.session_menu_anchor, Some(iced::Point::new(8.0, 16.0)));
}
