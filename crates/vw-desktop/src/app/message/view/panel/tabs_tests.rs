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

fn assert_scroll_offset(actual: f32, expected: f32) {
    assert!((actual - expected).abs() < f32::EPSILON);
}

// 数据边界：覆盖规范化、默认值以及相关状态字段的既定规则。
// 这里固定可观察结果，避免依赖处理过程中的临时实现细节。
#[test]
fn home_apps_bar_scroll_messages_update_and_clamp_offsets() {
    let mut app = test_app();

    let _ = update(&mut app, ViewMessage::HomeAppsBarScrollChanged(0.6));
    assert_scroll_offset(app.home_apps_bar_scroll_x, 0.6);

    let _ = update(&mut app, ViewMessage::HomeAppsBarPrev);
    assert_scroll_offset(app.home_apps_bar_scroll_x, 0.35);

    let _ = update(&mut app, ViewMessage::HomeAppsBarNext);
    assert_scroll_offset(app.home_apps_bar_scroll_x, 0.6);

    app.home_apps_bar_scroll_x = 0.1;
    let _ = update(&mut app, ViewMessage::HomeAppsBarPrev);
    assert_scroll_offset(app.home_apps_bar_scroll_x, 0.0);

    app.home_apps_bar_scroll_x = 0.9;
    let _ = update(&mut app, ViewMessage::HomeAppsBarNext);
    assert_scroll_offset(app.home_apps_bar_scroll_x, 1.0);
}

#[test]
fn tab_hovered_tracks_and_clears_hover() {
    let mut app = test_app();

    let _ = update(&mut app, ViewMessage::TabHovered(Some("project".to_string())));
    assert_eq!(app.hovered_tab_id.as_deref(), Some("project"));

    let _ = update(&mut app, ViewMessage::TabHovered(None));
    assert_eq!(app.hovered_tab_id, None);
}

#[test]
fn go_home_archives_project_preview_and_clears_project_context() {
    let mut app = test_app();
    app.screen = Screen::Project;
    app.project_path = Some("/project-a".to_string());
    app.project_id = Some("project-a".to_string());
    app.active_preview_path = Some("/project-a/src/main.rs".to_string());
    app.preview_tab_menu_path = Some("/project-a/src/main.rs".to_string());
    app.preview_tab_menu_pos = Some(iced::Point::new(12.0, 24.0));

    let _ = update(&mut app, ViewMessage::GoHome);

    assert_eq!(app.screen, Screen::Home);
    assert_eq!(app.active_tab_id.as_deref(), Some("home"));
    assert_eq!(app.project_path, None);
    assert_eq!(app.project_id, None);
    assert_eq!(app.active_preview_path, None);
    assert_eq!(
        app.project_preview_active_path.get("/project-a"),
        Some(&Some("/project-a/src/main.rs".to_string()))
    );
    assert!(app.project_preview_tabs.contains_key("/project-a"));
    assert_eq!(app.preview_tab_menu_path, None);
    assert_eq!(app.preview_tab_menu_pos, None);
}

#[test]
fn tab_selected_home_archives_previous_project_preview() {
    let mut app = test_app();
    app.screen = Screen::Project;
    app.project_path = Some("/project-a".to_string());
    app.active_preview_path = Some("/project-a/README.md".to_string());
    app.preview_tab_menu_path = Some("/project-a/README.md".to_string());

    let _ = update(&mut app, ViewMessage::TabSelected("home".to_string()));

    assert_eq!(app.screen, Screen::Home);
    assert_eq!(app.active_tab_id.as_deref(), Some("home"));
    assert_eq!(app.project_path, None);
    assert_eq!(app.active_preview_path, None);
    assert_eq!(
        app.project_preview_active_path.get("/project-a"),
        Some(&Some("/project-a/README.md".to_string()))
    );
    assert_eq!(app.preview_tab_menu_path, None);
}

// 状态转换：覆盖交互前后的字段同步、选择关系与清理规则。
// 连续操作仍需保持模块不变量，并清除已经失效的瞬态反馈。
#[test]
fn tab_selected_project_restores_cached_preview_state() {
    let mut app = test_app();
    app.open_tabs.push(tab("project-b", Screen::Project, Some("/project-b")));
    app.project_preview_tabs.insert("/project-b".to_string(), Vec::new());
    app.project_preview_active_path
        .insert("/project-b".to_string(), Some("/project-b/src/lib.rs".to_string()));

    let _ = update(&mut app, ViewMessage::TabSelected("project-b".to_string()));

    assert_eq!(app.screen, Screen::Project);
    assert_eq!(app.active_tab_id.as_deref(), Some("project-b"));
    assert_eq!(app.project_path.as_deref(), Some("/project-b"));
    assert_eq!(app.active_preview_path.as_deref(), Some("/project-b/src/lib.rs"));
    assert!(!app.project_preview_tabs.contains_key("/project-b"));
    assert!(!app.project_preview_active_path.contains_key("/project-b"));
}

#[test]
fn tab_selected_unknown_id_preserves_current_project_preview_state() {
    let mut app = test_app();
    app.screen = Screen::Project;
    app.project_path = Some("/project-a".to_string());
    app.active_preview_path = Some("/project-a/src/main.rs".to_string());

    let _ = update(&mut app, ViewMessage::TabSelected("missing".to_string()));

    assert_eq!(app.screen, Screen::Project);
    assert_eq!(app.active_tab_id.as_deref(), Some("missing"));
    assert_eq!(app.project_path.as_deref(), Some("/project-a"));
    assert_eq!(app.active_preview_path.as_deref(), Some("/project-a/src/main.rs"));
    assert!(!app.project_preview_tabs.contains_key("/project-a"));
    assert!(!app.project_preview_active_path.contains_key("/project-a"));
}

#[test]
fn tab_selected_same_project_changes_screen_without_losing_preview_state() {
    let mut app = test_app();
    app.open_tabs.push(tab("preview", Screen::Preview, Some("/project-a")));
    app.screen = Screen::Project;
    app.project_path = Some("/project-a".to_string());
    app.active_preview_path = Some("/project-a/src/main.rs".to_string());

    let _ = update(&mut app, ViewMessage::TabSelected("preview".to_string()));

    assert_eq!(app.screen, Screen::Preview);
    assert_eq!(app.active_tab_id.as_deref(), Some("preview"));
    assert_eq!(app.project_path.as_deref(), Some("/project-a"));
    assert_eq!(app.active_preview_path.as_deref(), Some("/project-a/src/main.rs"));
    assert!(!app.project_preview_tabs.contains_key("/project-a"));
    assert!(!app.project_preview_active_path.contains_key("/project-a"));
}

#[test]
fn tab_selected_knowledge_initializes_knowledge_without_project_switch() {
    let mut app = test_app();
    app.open_tabs.push(tab("knowledge", Screen::Knowledge, None));

    let _load_task = update(&mut app, ViewMessage::TabSelected("knowledge".to_string()));

    assert_eq!(app.active_tab_id.as_deref(), Some("knowledge"));
    assert_eq!(app.screen, Screen::Knowledge);
    assert_eq!(app.project_path, None);
    assert!(app.knowledge.loading_status);
}

#[test]
fn tab_closed_inactive_removes_only_requested_tab() {
    let mut app = test_app();
    app.open_tabs.push(tab("apps", Screen::Apps, None));

    let _ = update(&mut app, ViewMessage::TabClosed("apps".to_string()));

    assert_eq!(app.open_tabs.len(), 1);
    assert_eq!(app.open_tabs[0].id, "home");
    assert_eq!(app.active_tab_id.as_deref(), Some("home"));
    assert_eq!(app.screen, Screen::Home);
}

// 失败与边界：覆盖非法、缺失、过期或冲突场景的既定处理规则。
// 重点确认现有状态与错误反馈不会被意外的部分更新污染。
#[test]
fn tab_closed_missing_id_preserves_tab_selection() {
    let mut app = test_app();

    let _ = update(&mut app, ViewMessage::TabClosed("missing".to_string()));

    assert_eq!(app.open_tabs.len(), 1);
    assert_eq!(app.open_tabs[0].id, "home");
    assert_eq!(app.active_tab_id.as_deref(), Some("home"));
    assert_eq!(app.screen, Screen::Home);
}

#[test]
fn tab_closed_active_project_falls_back_home_and_archives_preview() {
    let mut app = test_app();
    app.open_tabs.push(tab("project-a", Screen::Project, Some("/project-a")));
    app.active_tab_id = Some("project-a".to_string());
    app.screen = Screen::Project;
    app.project_path = Some("/project-a".to_string());
    app.active_preview_path = Some("/project-a/src/main.rs".to_string());
    app.preview_tab_menu_path = Some("/project-a/src/main.rs".to_string());

    let _ = update(&mut app, ViewMessage::TabClosed("project-a".to_string()));

    assert_eq!(app.open_tabs.len(), 1);
    assert_eq!(app.active_tab_id.as_deref(), Some("home"));
    assert_eq!(app.screen, Screen::Home);
    assert_eq!(app.project_path, None);
    assert_eq!(app.active_preview_path, None);
    assert_eq!(
        app.project_preview_active_path.get("/project-a"),
        Some(&Some("/project-a/src/main.rs".to_string()))
    );
    assert_eq!(app.preview_tab_menu_path, None);
}

#[test]
fn tab_closed_active_tab_on_same_project_preserves_preview_state() {
    let mut app = test_app();
    app.open_tabs = vec![
        tab("project-summary", Screen::Project, Some("/project-a")),
        tab("project-preview", Screen::Preview, Some("/project-a")),
    ];
    app.active_tab_id = Some("project-preview".to_string());
    app.screen = Screen::Preview;
    app.project_path = Some("/project-a".to_string());
    app.active_preview_path = Some("/project-a/src/main.rs".to_string());

    let _ = update(&mut app, ViewMessage::TabClosed("project-preview".to_string()));

    assert_eq!(app.open_tabs.len(), 1);
    assert_eq!(app.active_tab_id.as_deref(), Some("project-summary"));
    assert_eq!(app.screen, Screen::Project);
    assert_eq!(app.project_path.as_deref(), Some("/project-a"));
    assert_eq!(app.active_preview_path.as_deref(), Some("/project-a/src/main.rs"));
    assert!(!app.project_preview_tabs.contains_key("/project-a"));
    assert!(!app.project_preview_active_path.contains_key("/project-a"));
}

#[test]
fn tab_closed_active_project_switches_to_last_project_and_restores_preview() {
    let mut app = test_app();
    app.open_tabs = vec![
        tab("project-b", Screen::Project, Some("/project-b")),
        tab("project-a", Screen::Project, Some("/project-a")),
    ];
    app.active_tab_id = Some("project-a".to_string());
    app.screen = Screen::Project;
    app.project_path = Some("/project-a".to_string());
    app.active_preview_path = Some("/project-a/src/main.rs".to_string());
    app.project_preview_active_path
        .insert("/project-b".to_string(), Some("/project-b/src/lib.rs".to_string()));

    let _ = update(&mut app, ViewMessage::TabClosed("project-a".to_string()));

    assert_eq!(app.open_tabs.len(), 1);
    assert_eq!(app.active_tab_id.as_deref(), Some("project-b"));
    assert_eq!(app.screen, Screen::Project);
    assert_eq!(app.project_path.as_deref(), Some("/project-b"));
    assert_eq!(app.active_preview_path.as_deref(), Some("/project-b/src/lib.rs"));
    assert_eq!(
        app.project_preview_active_path.get("/project-a"),
        Some(&Some("/project-a/src/main.rs".to_string()))
    );
}

// 状态转换：覆盖交互前后的字段同步、选择关系与清理规则。
// 连续操作仍需保持模块不变量，并清除已经失效的瞬态反馈。
#[test]
fn tab_closed_last_tab_falls_back_to_unselected_home_screen() {
    let mut app = test_app();
    app.open_tabs = vec![tab("apps", Screen::Apps, None)];
    app.active_tab_id = Some("apps".to_string());
    app.screen = Screen::Apps;

    let _ = update(&mut app, ViewMessage::TabClosed("apps".to_string()));

    assert!(app.open_tabs.is_empty());
    assert_eq!(app.active_tab_id, None);
    assert_eq!(app.screen, Screen::Home);
}
