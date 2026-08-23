//! 验证聊天应用状态的标签回退、会话标题查找与近期复制判定。
//! 活动会话可从近期列表或项目缓存解析，复制状态同时要求哈希匹配且时间仍有效。

use super::app_state::{
    current_branch_label, current_project_path_label, get_session_title, is_recent_copy,
};
use crate::app::App;
use web_time::{Duration as WebDuration, SystemTime as WebSystemTime};

fn session_info(id: &str, title: &str) -> vw_shared::session::info::Info {
    vw_shared::session::info::Info {
        id: id.to_string(),
        slug: id.to_string(),
        project_id: "project".to_string(),
        directory: "/tmp/project".to_string(),
        parent_id: None,
        summary: None,
        share: None,
        title: title.to_string(),
        version: "1".to_string(),
        time: vw_shared::session::info::TimeInfo {
            created: 1,
            updated: 1,
            compacting: None,
            archived: None,
        },
        permission: None,
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
fn is_recent_copy_requires_matching_hash_and_fresh_timestamp() {
    let mut app = App::new().0;
    app.last_copied_code_hash = Some(42);
    app.last_copy_time = Some(WebSystemTime::now() - WebDuration::from_millis(500));

    assert!(is_recent_copy(&app, 42));
    assert!(!is_recent_copy(&app, 7));

    app.last_copy_time = Some(WebSystemTime::now() - WebDuration::from_millis(2_000));
    assert!(!is_recent_copy(&app, 42));
}

#[test]
fn is_recent_copy_rejects_missing_or_unusable_copy_state() {
    let mut app = App::new().0;

    assert!(!is_recent_copy(&app, 42));

    app.last_copied_code_hash = Some(42);
    assert!(!is_recent_copy(&app, 42));

    app.last_copy_time = Some(WebSystemTime::now() + WebDuration::from_millis(60_000));
    assert!(!is_recent_copy(&app, 42));
}
