//! 宿主保留：加载会话优先于项目会话元数据的选择契约；纯令牌与角色聚合迁 desktop-usage。
//! 用量数据快照的测试，覆盖活动会话元数据、令牌合计和消息角色计数。
//! fixture 验证最新步骤优先，并在会话或元数据缺失时采用明确回退。
use crate::app::App;
use crate::app::models::{ChatSessionStep, TokenUsage};
use serde_json::json;
use session_info::{Info, TimeInfo};

fn session_info(id: &str, title: &str, created: u64, updated: u64) -> Info {
    Info {
        id: id.to_string(),
        slug: id.to_string(),
        project_id: "project".to_string(),
        directory: "/tmp/project".to_string(),
        parent_id: None,
        summary: None,
        share: None,
        title: title.to_string(),
        version: "0.0.0".to_string(),
        time: TimeInfo { created, updated, compacting: None, archived: None },
        permission: None,
        fork_execution_started: None,
        revert: None,
    }
}

fn step(index: u32, usage: TokenUsage) -> ChatSessionStep {
    ChatSessionStep {
        index,
        turn_id: None,
        started_ms: 10,
        finished_ms: Some(20),
        start_snapshot_path: None,
        finish_snapshot_path: None,
        usage,
        cost_usd: None,
        finish_reason: None,
        model: None,
        tool_timings: Vec::new(),
    }
}

#[test]
fn from_app_prefers_loaded_session_metadata_and_latest_step_usage() {
    let (mut app, _task) = App::new();
    app.active_session_id = Some("session-1".to_string());
    app.sessions = vec![session_info("session-1", "Loaded", 100, 200)];
    app.project_sessions
        .insert("/project".to_string(), vec![session_info("session-1", "Project", 300, 400)]);
    app.active_session_view_state.calls = vec![json!({ "kind": "llm" })];
    app.active_session_view_state.steps = vec![
        step(1, TokenUsage::default()),
        step(
            2,
            TokenUsage {
                input_tokens: 11,
                output_tokens: 7,
                cached_tokens: 3,
                reasoning_tokens: 2,
            },
        ),
    ];

    let usage = super::from_app(&app);
    let session = usage.session.as_ref().expect("active session should be projected");

    assert_eq!(usage.session_title, "Loaded");
    assert_eq!((session.created_ms, session.updated_ms), (100, 200));
    assert_eq!(usage.call_count, 1);
    assert_eq!(usage.last_step_total_tokens, 23);
}



#[test]
fn from_app_falls_back_to_project_metadata_for_active_session() {
    let (mut app, _task) = App::new();
    app.active_session_id = Some("project-session".to_string());
    app.project_sessions.insert(
        "/project".to_string(),
        vec![session_info("project-session", "Project Session", 500, 600)],
    );

    let usage = super::from_app(&app);
    let session = usage.session.as_ref().expect("project session should be projected");

    assert_eq!(usage.session_title, "Project Session");
    assert_eq!((session.created_ms, session.updated_ms), (500, 600));
}
