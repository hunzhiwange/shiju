//! 验证聊天标题栏的会话状态、工作区标记、返回操作与主题样式。
//! fixture 组合近期会话、项目缓存和运行态队列，覆盖父会话回退查找与空标识拒绝。

use super::{App, Message, message, return_to_parent_session_pressed};
use session_info::{Info, TimeInfo};

fn test_app() -> App {
    App::new().0
}

fn session(id: &str, directory: &str) -> Info {
    Info {
        id: id.to_string(),
        slug: format!("{id}-slug"),
        project_id: "project".to_string(),
        directory: directory.to_string(),
        parent_id: None,
        summary: None,
        share: None,
        title: format!("{id} title"),
        version: "1".to_string(),
        time: TimeInfo { created: 1, updated: 2, compacting: None, archived: None },
        permission: None,
        fork_execution_started: None,
        revert: None,
    }
}

#[test]
fn return_to_parent_session_pressed_emits_open_parent_when_parent_id_present() {
    let mut app = test_app();
    let parent = session("parent-session", "/workspace/main");
    let mut child = session("child", "/workspace/main");
    child.parent_id = Some(parent.id.clone());
    app.navigation_tabs.project_path = Some("/workspace/main".to_string());
    app.active_session_id = Some(parent.id.clone());
    app.sessions = vec![parent, child];

    let _open_task = crate::app::message::project::update(
        &mut app,
        message::ProjectMessage::OpenProjectSessionPressed(
            "/workspace/main".to_string(),
            "child".to_string(),
        ),
    );
    assert_eq!(app.active_session_id.as_deref(), Some("child"));

    match return_to_parent_session_pressed(&app) {
        Some(Message::Project(message::ProjectMessage::OpenProjectSessionPressed(path, id))) => {
            assert_eq!(path, "/workspace/main");
            assert_eq!(id, "parent-session");
        }
        other => panic!("expected OpenProjectSessionPressed, got {other:?}"),
    }
}
