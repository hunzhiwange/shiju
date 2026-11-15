use super::*;
use std::collections::HashSet;
use iced::{Element, Point};
use session_info::{Info, Summary, TimeInfo};
use crate::app::state::{AcpHistoryReplayMode, AgentRequest, ChatSendBehavior, QueueItem, SessionRuntimeState};
use domain::session_tree_projection;
fn test_app() -> crate::app::App {
    crate::app::App::new().0
}

fn keep_element(element: Element<'_, Message>) {
    std::hint::black_box(element);
}

fn session(id: &str, title: &str, directory: &str, archived: bool) -> Info {
    Info {
        id: id.to_string(),
        slug: format!("{id}-slug"),
        project_id: "project".to_string(),
        directory: directory.to_string(),
        parent_id: None,
        summary: Some(Summary { additions: 3, deletions: 1, files: 2, diffs: None }),
        share: None,
        title: title.to_string(),
        version: "1".to_string(),
        time: TimeInfo {
            created: 1,
            updated: 2,
            compacting: None,
            archived: archived.then_some(3),
        },
        permission: None,
        fork_execution_started: None,
        revert: None,
    }
}

fn child_session(id: &str, parent_id: &str, updated: u64) -> Info {
    let mut info = session(id, id, "/tmp/project", false);
    info.parent_id = Some(parent_id.to_string());
    info.time.updated = updated;
    info
}

fn agent_request(id: u64, session_id: &str) -> std::sync::Arc<AgentRequest> {
    std::sync::Arc::new(AgentRequest {
        id,
        session: session_id.to_string(),
        query: "执行计划".to_string(),
        root: None,
        model: None,
        reasoning_effort: None,
        acp_test: false,
        acp_model_source: Default::default(),
        acp_agent: None,
        acp_allowed_tools: None,
        agent: None,
        allowed_tools: None,
        require_agent_tool: false,
        acp_force_new_session: false,
        acp_history_mode: AcpHistoryReplayMode::Discard,
        acp_recent_count: 0,
        full_access_enabled: Some(false),
        resume_history_only: false,
        workflow_mode_enabled: true,
        goal_mode_enabled: false,
        history: Vec::new(),
    })
}

fn queue_item(query: &str) -> QueueItem {
    QueueItem {
        created_ms: 1,
        query: query.to_string(),
        attachments: Vec::new(),
        root: None,
        model: None,
        reasoning_effort: None,
        acp_test: false,
        acp_model_source: Default::default(),
        acp_agent: None,
        acp_allowed_tools: None,
        agent: None,
        allowed_tools: None,
        require_agent_tool: false,
        acp_force_new_session: false,
        acp_history_mode: AcpHistoryReplayMode::Discard,
        acp_recent_count: 3,
        full_access_enabled: true,
        send_behavior: ChatSendBehavior::Queue,
        request_history_override: None,
        resume_history_only: false,
        workflow_mode_enabled: false,
        goal_mode_enabled: false,
    }
}

#[test]
fn session_items_build_loading_empty_and_load_button_states() {
    let app = test_app();
    keep_element(session_items_list(&app, "/tmp/project", None, "Project", 10, false));
    keep_element(session_items_list(&app, "/tmp/project", Some(&Vec::new()), "Project", 10, false));
    keep_element(session_items_list(&app, "/tmp/project", None, "Project", 10, true));
}

#[test]
fn session_tree_projects_only_main_tasks_regardless_of_expansion() {
    let mut older_root = session("older-root", "Older root", "/tmp/project", false);
    older_root.time.updated = 10;
    let mut newer_root = session("newer-root", "Newer root", "/tmp/project", false);
    newer_root.time.updated = 50;
    let child = child_session("child", "newer-root", 40);
    let sibling = child_session("sibling", "newer-root", 20);
    let grandchild = child_session("grandchild", "child", 30);
    let sessions = vec![older_root, sibling, grandchild, newer_root, child];

    let collapsed = session_tree_projection(&sessions, 1, &HashSet::new());
    assert_eq!(collapsed.total_roots, 2);
    assert_eq!(
        collapsed.rows.iter().map(|row| row.session.id.as_str()).collect::<Vec<_>>(),
        vec!["newer-root"]
    );

    let parent_expanded = session_tree_projection(&sessions, 1, &HashSet::new());
    assert_eq!(parent_expanded.total_roots, 2);
    assert_eq!(
        parent_expanded.rows.iter().map(|row| row.session.id.as_str()).collect::<Vec<_>>(),
        vec!["newer-root"]
    );

    let recursively_expanded = session_tree_projection(&sessions, 1, &HashSet::new());
    assert_eq!(
        recursively_expanded.rows.iter().map(|row| row.session.id.as_str()).collect::<Vec<_>>(),
        vec!["newer-root"]
    );

    let app = test_app();
    keep_element(session_items_list(&app, "/tmp/project", Some(&sessions), "Project", 1, false));
}

#[test]
fn session_items_filters_archived_and_shows_load_more() {
    let app = test_app();
    let sessions = vec![
        session("s1", "可见会话一", "/tmp/project", false),
        session("s2", "已归档会话", "/tmp/project", true),
        session("s3", "可见会话二", "/tmp/project/worktree-a", false),
    ];

    keep_element(session_items_list(&app, "/tmp/project", Some(&sessions), "Project", 1, false));
    keep_element(session_items_list(&app, "/tmp/project", Some(&sessions), "Project", 0, false));
}

#[test]
fn session_items_reports_empty_when_only_archived_sessions_exist() {
    let app = test_app();
    let sessions = vec![
        session("s1", "归档一", "/tmp/project", true),
        session("s2", "归档二", "/tmp/project/worktree-a", true),
    ];

    keep_element(session_items_list(&app, "/tmp/project", Some(&sessions), "Project", 10, false));
}

#[test]
fn session_items_builds_runtime_status_variants() {
    let mut app = test_app();
    app.active_session_id = Some("running".to_string());
    app.session_menu_id = Some("queued".to_string());
    app.session_menu_anchor = Some(Point::new(4.0, 6.0));

    let mut running = SessionRuntimeState::default();
    running.request_lifecycle.is_requesting = true;
    app.session_runtime_states.insert("running".to_string(), running);

    let mut queued = SessionRuntimeState::default();
    queued.request_lifecycle.queue.push(queue_item("继续"));
    app.session_runtime_states.insert("queued".to_string(), queued);

    let mut unseen = SessionRuntimeState::default();
    unseen.has_unseen_success = true;
    app.session_runtime_states.insert("unseen".to_string(), unseen);

    let sessions = vec![
        session("running", "运行中会话", "/tmp/project", false),
        session("queued", "排队会话", "/tmp/project/worktree-b", false),
        session("unseen", "未读成功会话", "/tmp/project", false),
        session("idle", "空闲会话", "", false),
    ];

    keep_element(session_items_list(&app, "/tmp/project", Some(&sessions), "Project", 10, false));
}

#[test]
fn session_items_builds_missing_summary_menu_default_anchor_and_no_load_more() {
    let mut app = test_app();
    app.session_menu_id = Some("summaryless".to_string());
    app.session_menu_anchor = None;

    let mut summaryless = session("summaryless", "没有摘要的会话", "/", false);
    summaryless.summary = None;
    let sessions =
        vec![summaryless, session("normal", "普通会话", "/tmp/project/worktree-d", false)];

    keep_element(session_items_list(&app, "/tmp/project", Some(&sessions), "Project", 10, false));
}

#[test]
fn project_panel_builds_empty_and_selected_project_states() {
    let mut app = test_app();
    keep_element(project_sessions_panel(&app, None));

    app.recent_projects.push("/tmp/project".to_string());
    keep_element(project_sessions_panel(&app, None));

    app.recent_projects_edits.push("自定义项目名".to_string());
    app.project_tools_menu_path = Some("/tmp/project".to_string());
    app.project_session_has_vertical_scrollbar.insert("/tmp/project".to_string(), true);
    app.project_session_load_counts.insert("/tmp/project".to_string(), 1);
    app.project_sessions.insert(
        "/tmp/project".to_string(),
        vec![
            session("s1", "第一条会话", "/tmp/project", false),
            session("s2", "第二条会话", "/tmp/project/worktree-c", false),
        ],
    );

    keep_element(project_sessions_panel(&app, Some("/tmp/project".to_string())));

    app.project_sessions_loading.insert("/tmp/loading".to_string());
    keep_element(project_sessions_panel(&app, Some("/tmp/loading".to_string())));
}

#[test]
fn project_panel_uses_path_fallback_for_unknown_or_blank_recent_names() {
    let mut app = test_app();
    app.recent_projects = vec!["/tmp/project".to_string()];
    app.recent_projects_edits = vec!["   ".to_string()];
    app.project_path = Some("/tmp/project".to_string());

    keep_element(project_sessions_panel(&app, Some("/tmp/project".to_string())));
    keep_element(project_sessions_panel(&app, Some("/tmp/unknown-project".to_string())));
}
#[test]
fn plan_run_live_children_integrate_with_session_visibility() {
    const REQUEST_ID: u64 = 42;
    const DIRECTORY: &str = "/workspace/project";
    const MAIN_ID: &str = "main-session";
    const PLANNING_ID: &str = "task-1-plan";
    const BATCH_ONE_ID: &str = "task-1-sub-1";
    const BATCH_TWO_ID: &str = "task-1-sub-2";

    let (mut app, _) = crate::app::App::new();
    app.project_path = Some(DIRECTORY.to_string());
    app.active_session_id = Some(MAIN_ID.to_string());

    let mut main = session(MAIN_ID, "主会话", DIRECTORY, false);
    main.time.updated = 100;
    let mut ordinary_child = child_session("ordinary-child", MAIN_ID, 90);
    ordinary_child.directory = DIRECTORY.to_string();
    ordinary_child.title = "普通子会话".to_string();
    let mut ordinary_grandchild = child_session("ordinary-grandchild", "ordinary-child", 80);
    ordinary_grandchild.directory = DIRECTORY.to_string();
    ordinary_grandchild.title = "普通孙会话".to_string();
    app.sessions = vec![ordinary_grandchild, ordinary_child, main];
    app.chat = vec![crate::app::models::ChatMessage {
        role: crate::app::models::ChatRole::User,
        content: "主会话任务".to_string(),
        model: None,
        think_timing: Vec::new(),
        processing_timing: None,
    }];
    app.chat_message_ids = vec![None];
    let parent_runtime = app.get_session_runtime_mut(MAIN_ID);
    parent_runtime.request_lifecycle.is_requesting = true;
    parent_runtime.request_lifecycle.active_agent_request = Some(agent_request(REQUEST_ID, MAIN_ID));

    for message in [
        message::ChatMessage::PlanRunSessionStarted {
            title_feedback: None,
            parent_request_id: REQUEST_ID,
            parent_session_id: MAIN_ID.to_string(),
            session_id: PLANNING_ID.to_string(),
            title: "规划 · task-1".to_string(),
            directory: DIRECTORY.to_string(),
            query: "拆分并执行任务".to_string(),
        },
        message::ChatMessage::PlanRunSessionDelta {
            session_id: PLANNING_ID.to_string(),
            delta: "已生成两批计划".to_string(),
        },
        message::ChatMessage::PlanRunSessionStarted {
            title_feedback: None,
            parent_request_id: REQUEST_ID,
            parent_session_id: MAIN_ID.to_string(),
            session_id: BATCH_ONE_ID.to_string(),
            title: "实现入口".to_string(),
            directory: DIRECTORY.to_string(),
            query: "实现入口".to_string(),
        },
        message::ChatMessage::PlanRunSessionDelta {
            session_id: BATCH_ONE_ID.to_string(),
            delta: "第一批执行中".to_string(),
        },
        message::ChatMessage::PlanRunSessionStarted {
            title_feedback: None,
            parent_request_id: REQUEST_ID,
            parent_session_id: MAIN_ID.to_string(),
            session_id: BATCH_TWO_ID.to_string(),
            title: "回归验证".to_string(),
            directory: DIRECTORY.to_string(),
            query: "回归验证".to_string(),
        },
        message::ChatMessage::PlanRunSessionDelta {
            session_id: BATCH_TWO_ID.to_string(),
            delta: "第二批执行中".to_string(),
        },
    ] {
        let _ = crate::app::message::chat::update(&mut app, message);
    }

    assert_eq!(app.active_session_id.as_deref(), Some(MAIN_ID));
    assert_eq!(app.chat.len(), 1);
    assert_eq!(app.chat[0].content, "主会话任务");
    for session_id in [PLANNING_ID, BATCH_ONE_ID, BATCH_TWO_ID] {
        assert!(!app.sessions.iter().any(|session| session.id == session_id));
        assert!(app.get_session_runtime(session_id).is_plan_run_session);
        assert!(app.get_session_runtime(session_id).request_lifecycle.is_requesting);
        let _ = crate::app::message::chat::update(
            &mut app,
            message::ChatMessage::AgentStreamDeltaFlush(session_id.to_string()),
        );
        assert_eq!(
            app.cached_chat_messages(session_id)
                .expect("live child chat cache")
                .last()
                .expect("live child delta")
                .role,
            crate::app::models::ChatRole::Assistant
        );
    }

    let running_batch_two = r#"tool workflow_node
{"tool_call_id":"plan-run-task-1-subtask-sub-2","status":"running","summary":"批次 2 · 回归验证","metadata":{"plan_run":true,"event_type":"subtask_started","session_id":"task-1-sub-2","title":"回归验证","batch":2}}"#;
    assert_eq!(
        crate::app::components::chat_panel::tools::tool_call_id_from_raw(running_batch_two)
            .as_deref(),
        Some("plan-run-task-1-subtask-sub-2")
    );
    keep_element(
        crate::app::components::chat_panel::tools::tool_workflow_view(
            &app,
            0,
            0,
            running_batch_two,
        )
        .expect("running plan-run node should parse and render"),
    );

    let collapsed = session_tree_projection(&app.sessions, 10, &HashSet::new());
    assert_eq!(collapsed.total_roots, 1);
    assert_eq!(
        collapsed.rows.iter().map(|row| row.session.id.as_str()).collect::<Vec<_>>(),
        vec![MAIN_ID]
    );

    let parent_expanded = session_tree_projection(&app.sessions, 10, &HashSet::new());
    assert_eq!(
        parent_expanded.rows.iter().map(|row| row.session.id.as_str()).collect::<Vec<_>>(),
        vec![MAIN_ID]
    );

    let recursively_expanded = session_tree_projection(&app.sessions, 10, &HashSet::new());
    assert_eq!(
        recursively_expanded.rows.iter().map(|row| row.session.id.as_str()).collect::<Vec<_>>(),
        vec![MAIN_ID]
    );
    assert!(
        recursively_expanded.rows.iter().all(|row| !row.session.is_plan_run()),
        "plan-run sessions must stay hidden from the ordinary session tree"
    );

    for session_id in [BATCH_ONE_ID, BATCH_TWO_ID, PLANNING_ID] {
        let _ = crate::app::message::chat::update(
            &mut app,
            message::ChatMessage::PlanRunSessionFinished(session_id.to_string()),
        );
        assert!(!app.get_session_runtime(session_id).request_lifecycle.is_requesting);
    }
    assert!(app.get_session_runtime(MAIN_ID).request_lifecycle.active_agent_request.is_some());
}
