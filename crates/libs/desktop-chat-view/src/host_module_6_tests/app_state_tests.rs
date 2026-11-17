//! 验证聊天应用状态的标签回退、会话标题查找与近期复制判定。
//! 活动会话可从近期列表或项目缓存解析，复制状态同时要求哈希匹配且时间仍有效。

use super::label_fixture::LabelFixture as App;
use super::label_fixture::{current_branch_label, current_project_path_label, get_session_title};

fn session_info(id: &str, title: &str) -> session_info::Info {
    session_info::Info {
        id: id.to_string(),
        slug: id.to_string(),
        project_id: "project".to_string(),
        directory: "/tmp/project".to_string(),
        parent_id: None,
        summary: None,
        share: None,
        title: title.to_string(),
        version: "1".to_string(),
        time: session_info::TimeInfo { created: 1, updated: 1, compacting: None, archived: None },
        permission: None,
        fork_execution_started: None,
        revert: None,
    }
}

#[test]
fn app_state_labels_fall_back_when_values_are_missing() {
    let app = App::new().0;

    assert_eq!(get_session_title(&app), "暂无");
    assert_eq!(current_project_path_label(&app), "未打开项目");
    assert_eq!(current_branch_label(&app), "-");
}

#[test]
fn app_state_labels_return_current_values_when_present() {
    let mut app = App::new().0;
    app.project_path = Some("/work/project".to_string());
    app.selected_branch = Some("main".to_string());
    app.sessions = vec![session_info("stale", "Stale"), session_info("active", "Active")];
    app.active_session_id = Some("active".to_string());

    assert_eq!(get_session_title(&app), "Active");
    assert_eq!(current_project_path_label(&app), "/work/project");
    assert_eq!(current_branch_label(&app), "main");
}

#[test]
fn session_title_falls_back_when_active_session_is_missing() {
    let mut app = App::new().0;
    app.sessions = vec![session_info("other", "Other")];
    app.active_session_id = Some("missing".to_string());

    assert_eq!(get_session_title(&app), "暂无");
}

#[test]
fn session_title_uses_project_session_when_active_session_is_not_in_recent_list() {
    let mut app = App::new().0;
    app.active_session_id = Some("plan-child".to_string());
    app.sessions = vec![session_info("plan-child", "   ")];
    app.project_sessions
        .insert("/tmp/project".to_string(), vec![session_info("plan-child", "规划 · task-1")]);

    assert_eq!(get_session_title(&app), "规划 · task-1");
}

#[test]
fn session_title_keeps_new_session_fallback_when_all_matching_titles_are_blank() {
    let mut app = App::new().0;
    app.active_session_id = Some("active".to_string());
    app.sessions = vec![session_info("active", "   ")];
    app.project_sessions
        .insert("/tmp/project".to_string(), vec![session_info("active", "")]);

    assert_eq!(get_session_title(&app), "新会话");
}
