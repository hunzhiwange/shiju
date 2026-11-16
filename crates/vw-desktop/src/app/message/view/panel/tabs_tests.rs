// 验证主面板标签选择、滚动偏移与项目预览恢复。
// 测试夹具围绕目标分支构造可控状态；跨模块行为由对应集成测试负责。

use super::update;
use crate::app::message::ViewMessage;
use crate::app::state::AppTab;
use crate::app::{App, Screen};

fn test_app() -> App {
    App::new().0
}

fn tab(id: &str, screen: Screen, project_path: Option<&str>) -> AppTab {
    AppTab {
        id: id.to_string(),
        title: id.to_string(),
        screen,
        project_path: project_path.map(str::to_owned),
    }
}

// 数据边界：覆盖规范化、默认值以及相关状态字段的既定规则。
// 这里固定可观察结果，避免依赖处理过程中的临时实现细节。

#[test]
fn go_home_archives_project_preview_and_clears_project_context() {
    let mut app = test_app();
    app.navigation_tabs.screen = Screen::Project;
    app.navigation_tabs.project_path = Some("/project-a".to_string());
    app.workspace_state.project_id = Some("project-a".to_string());
    app.navigation_tabs.active_preview_path = Some("/project-a/src/main.rs".to_string());
    app.navigation_tabs.preview_tab_menu_path = Some("/project-a/src/main.rs".to_string());
    app.navigation_tabs.preview_tab_menu_pos = Some(iced::Point::new(12.0, 24.0));

    let _ = update(&mut app, ViewMessage::GoHome);

    assert_eq!(app.navigation_tabs.screen, Screen::Home);
    assert_eq!(app.navigation_tabs.active_tab_id.as_deref(), Some("home"));
    assert_eq!(app.navigation_tabs.project_path, None);
    assert_eq!(app.workspace_state.project_id, None);
    assert_eq!(app.navigation_tabs.active_preview_path, None);
    assert_eq!(
        app.navigation_tabs.project_preview_active_path.get("/project-a"),
        Some(&Some("/project-a/src/main.rs".to_string()))
    );
    assert!(app.navigation_tabs.project_preview_tabs.contains_key("/project-a"));
    assert_eq!(app.navigation_tabs.preview_tab_menu_path, None);
    assert_eq!(app.navigation_tabs.preview_tab_menu_pos, None);
}

// 状态转换：覆盖交互前后的字段同步、选择关系与清理规则。
// 连续操作仍需保持模块不变量，并清除已经失效的瞬态反馈。

#[test]
fn tab_selected_knowledge_initializes_knowledge_without_project_switch() {
    let mut app = test_app();
    app.navigation_tabs.open_tabs.push(tab("knowledge", Screen::Knowledge, None));

    let _load_task = update(&mut app, ViewMessage::TabSelected("knowledge".to_string()));

    assert_eq!(app.navigation_tabs.active_tab_id.as_deref(), Some("knowledge"));
    assert_eq!(app.navigation_tabs.screen, Screen::Knowledge);
    assert_eq!(app.navigation_tabs.project_path, None);
    assert!(app.knowledge.loading_status);
}

// 失败与边界：覆盖非法、缺失、过期或冲突场景的既定处理规则。
// 重点确认现有状态与错误反馈不会被意外的部分更新污染。

// 状态转换：覆盖交互前后的字段同步、选择关系与清理规则。
// 连续操作仍需保持模块不变量，并清除已经失效的瞬态反馈。
