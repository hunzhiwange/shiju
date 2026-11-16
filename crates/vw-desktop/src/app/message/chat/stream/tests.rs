// 宿主契约：核对路由消息、活动视图、项目列表、缓存与保存 hook 的跨域投影。
//! 验证聊天流终止、会话目录解析、缓存恢复和计划运行生命周期。
//!
//! 缓存会话与网关历史故意制造标识不一致，加载逻辑必须选择一致来源并立即刷新终止增量。

use crate::app::models::{ChatMessage as ModelChatMessage, ChatRole};

fn chat_message(role: ChatRole, content: &str) -> ModelChatMessage {
    ModelChatMessage {
        role,
        content: content.to_string(),
        model: None,
        think_timing: Vec::new(),
        processing_timing: None,
    }
}

fn session_info(id: &str, title: &str, directory: &str) -> session_info::Info {
    session_info::Info {
        id: id.to_string(),
        slug: format!("{id}-slug"),
        project_id: "project-1".to_string(),
        directory: directory.to_string(),
        parent_id: None,
        summary: None,
        share: None,
        title: title.to_string(),
        version: "0.0.0".to_string(),
        time: session_info::TimeInfo {
            created: 10,
            updated: 20,
            compacting: None,
            archived: None,
        },
        permission: None,
        fork_execution_started: None,
        revert: None,
    }
}

fn agent_request(id: u64, session_id: &str) -> std::sync::Arc<crate::app::state::AgentRequest> {
    std::sync::Arc::new(crate::app::state::AgentRequest {
        id,
        session: session_id.to_string(),
        query: "query".to_string(),
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
        acp_history_mode: crate::app::state::AcpHistoryReplayMode::Discard,
        acp_recent_count: 0,
        full_access_enabled: Some(false),
        resume_history_only: false,
        workflow_mode_enabled: true,
        goal_mode_enabled: false,
        history: Vec::new(),
    })
}

#[test]
fn completed_session_summary_reload_refreshes_both_lists_without_an_active_request() {
    let (mut app, _) = crate::app::App::new();
    let mut info = session_info("finished", "Finished", "/project");
    app.session_state.sessions = vec![info.clone()];
    app.workspace_state.project_sessions.insert("/project".into(), vec![info.clone()]);
    info.summary = Some(session_info::Summary {
        additions: 1434,
        deletions: 7,
        files: 16,
        diffs: None,
    });
    info.time.updated += 1;
    let _ = super::update(&mut app, super::ChatMessage::SessionSummaryReloaded(Ok(info)));
    for sessions in [&app.session_state.sessions, &app.workspace_state.project_sessions["/project"]] {
        let summary = sessions[0].summary.as_ref().unwrap();
        assert_eq!((summary.additions, summary.deletions, summary.files), (1434, 7, 16));
    }
}

#[test]
fn stream_diff_summary_updates_the_request_session_in_both_lists() {
    let (mut app, _) = crate::app::App::new();
    let info = session_info("edited", "Edited", "/project");
    app.session_state.sessions = vec![info.clone(), session_info("other", "Other", "/project")];
    app.workspace_state.project_sessions.insert("/project".into(), app.session_state.sessions.clone());
    app.session_state.active_session_id = Some("other".into());
    app.get_session_runtime_mut("edited").request_lifecycle.active_agent_request = Some(agent_request(42, "edited"));
    let summary =
        session_info::Summary { additions: 2, deletions: 1, files: 1, diffs: None };

    let _ = super::update(
        &mut app,
        super::ChatMessage::AgentSessionDiffSummary(41, "edited".into(), summary.clone()),
    );
    assert!(app.session_state.sessions[0].summary.is_none());
    app.get_session_runtime_mut("edited").request_lifecycle.active_agent_request = None;
    let _ = super::update(
        &mut app,
        super::ChatMessage::AgentSessionDiffSummary(42, "edited".into(), summary),
    );

    for sessions in [&app.session_state.sessions, &app.workspace_state.project_sessions["/project"]] {
        let summary = sessions[0].summary.as_ref().unwrap();
        assert_eq!((summary.additions, summary.deletions), (2, 1));
        assert!(sessions[1].summary.is_none());
    }
}

#[test]
fn desktop_plan_run_waiting_and_cancelled_events_keep_session_lifecycle_consistent_in_chat_stream()
{
    let (mut app, _task) = crate::app::App::new();
    app.session_state.active_session_id = Some("main-session".to_string());
    app.session_state.sessions = vec![
        session_info("main-session", "Main", "/project"),
        session_info("task-2-sub-9", "Child", "/project"),
    ];
    app.chat_view.chat = vec![chat_message(ChatRole::User, "run workflow")];
    {
        let parent = app.get_session_runtime_mut("main-session");
        parent.request_lifecycle.is_requesting = true;
        parent.request_lifecycle.active_agent_request = Some(agent_request(42, "main-session"));
    }
    app.get_session_runtime_mut("task-2-sub-9").request_lifecycle.is_requesting = true;

    let _waiting = super::update(
        &mut app,
        super::ChatMessage::PlanRunSessionWaiting("task-2-sub-9".to_string()),
    );

    assert!(app.get_session_runtime("task-2-sub-9").request_lifecycle.is_requesting);
    assert!(app.get_session_runtime("main-session").request_lifecycle.is_requesting);

    let _child_cancelled = super::update(
        &mut app,
        super::ChatMessage::PlanRunSessionCancelled("task-2-sub-9".to_string()),
    );

    assert!(!app.get_session_runtime("task-2-sub-9").request_lifecycle.is_requesting);
    assert!(app.get_session_runtime("main-session").request_lifecycle.is_requesting);

    let _parent_cancelled = super::update(&mut app, super::ChatMessage::AgentStreamCancelled(42));

    let parent = app.get_session_runtime("main-session");
    assert!(!parent.request_lifecycle.is_requesting);
    assert!(parent.request_lifecycle.active_agent_request.is_none());
    assert!(!parent.has_unseen_success);
    assert!(app.chat_view.chat.iter().all(|message| message.role != ChatRole::System));
}

#[test]
fn stream_terminal_flushes_pending_active_delta_immediately() {
    fn active_app(request_id: u64) -> crate::app::App {
        let (mut app, _) = crate::app::App::new();
        let session_id = format!("session-{request_id}");
        app.session_state.active_session_id = Some(session_id.clone());
        app.session_state.sessions = vec![session_info(&session_id, "Session", "/project")];
        app.chat_view.chat =
            vec![chat_message(ChatRole::User, "question"), chat_message(ChatRole::Assistant, "")];
        app.chat_view.chat_message_ids = vec![None, None];
        let runtime = app.get_session_runtime_mut(&session_id);
        runtime.request_lifecycle.is_requesting = true;
        runtime.request_lifecycle.active_agent_request = Some(agent_request(request_id, &session_id));
        app
    }

    let mut done_app = active_app(51);
    let done_session = done_app.session_state.active_session_id.clone().unwrap();
    let _ =
        super::stream_events::queue_agent_stream_delta(&mut done_app, 51, "done tail".to_string());
    let _ = super::update(
        &mut done_app,
        super::ChatMessage::AgentStreamDone(
            51,
            crate::app::models::TokenUsage::default(),
            None,
            None,
        ),
    );
    assert_eq!(done_app.chat_view.chat[1].content, "done tail");
    assert!(done_app.get_session_runtime(&done_session).pending_stream_delta.is_empty());
    assert!(!done_app.get_session_runtime(&done_session).stream_delta_flush_scheduled);
    assert!(!done_app.get_session_runtime(&done_session).stream_save.flush_scheduled);
    assert_eq!(
        super::stream_events::flush_agent_stream_delta(&mut done_app, done_session).units(),
        0
    );

    let mut cancelled_app = active_app(52);
    let cancelled_session = cancelled_app.session_state.active_session_id.clone().unwrap();
    let _ = super::stream_events::queue_agent_stream_delta(
        &mut cancelled_app,
        52,
        "cancelled tail".to_string(),
    );
    let _ = super::update(&mut cancelled_app, super::ChatMessage::AgentStreamCancelled(52));
    assert_eq!(cancelled_app.chat_view.chat[1].content, "cancelled tail");
    assert!(cancelled_app.get_session_runtime(&cancelled_session).pending_stream_delta.is_empty());
    assert!(!cancelled_app.get_session_runtime(&cancelled_session).stream_delta_flush_scheduled);
    assert!(!cancelled_app.get_session_runtime(&cancelled_session).stream_save.flush_scheduled);

    let mut error_app = active_app(53);
    let error_session = error_app.session_state.active_session_id.clone().unwrap();
    let _ = super::stream_events::queue_agent_stream_delta(
        &mut error_app,
        53,
        "error tail".to_string(),
    );
    let _ = super::update(
        &mut error_app,
        super::ChatMessage::AgentStreamError(53, "failed".to_string()),
    );
    assert_eq!(error_app.chat_view.chat[1].content, "error tail");
    assert!(error_app.get_session_runtime(&error_session).pending_stream_delta.is_empty());
    assert!(!error_app.get_session_runtime(&error_session).stream_delta_flush_scheduled);
    assert!(!error_app.get_session_runtime(&error_session).stream_save.flush_scheduled);
}











#[test]
fn load_session_or_default_uses_active_chat_ids_calls_and_steps() {
    let (mut app, _task) = crate::app::App::new();
    app.session_state.sessions = vec![session_info("session-1", "Active", "/project")];
    app.session_state.active_session_id = Some("session-1".to_string());
    app.chat_view.chat = vec![chat_message(ChatRole::User, "question")];
    app.chat_view.chat_message_ids = vec![Some("message-1".to_string())];

    let loaded = super::load_session_or_default(&app, "session-1".to_string());

    assert_eq!(loaded.messages.len(), 1);
    assert_eq!(loaded.messages[0].role, ChatRole::User);
    assert_eq!(loaded.messages[0].content, "question");
    assert_eq!(loaded.message_ids, app.chat_view.chat_message_ids);
    assert_eq!(loaded.calls.len(), app.chat_view.active_session_view_state.calls.len());
    assert_eq!(loaded.steps.len(), app.chat_view.active_session_view_state.steps.len());
}



#[test]
#[cfg(not(target_arch = "wasm32"))]
fn save_session_hook_observes_arguments_and_is_cleared_after_scope() {
    use std::path::PathBuf;
    use std::sync::{Arc, Mutex};

    let session =
        super::load_session_or_default(&crate::app::App::new().0, "session-1".to_string());
    let observed = Arc::new(Mutex::new(None));
    let hook_observed = Arc::clone(&observed);

    let result = super::with_save_session_hook(
        move |session, directory| {
            *hook_observed.lock().unwrap() =
                Some((session.id.clone(), directory.map(str::to_string)));
            Some(PathBuf::from("/saved/session.json"))
        },
        || super::call_save_session_hook(&session, Some("/project")),
    );

    assert_eq!(result, Some(Some(PathBuf::from("/saved/session.json"))));
    assert_eq!(
        observed.lock().unwrap().as_ref(),
        Some(&("session-1".to_string(), Some("/project".to_string())))
    );
    assert!(super::call_save_session_hook(&session, None).is_none());
}

#[test]
fn externally_created_session_does_not_replace_active_session() {
    let (mut app, _task) = crate::app::App::new();
    let mut visible = session_info("visible-task", "Visible task", "/project");
    visible.parent_id = Some("main-session".to_string());
    visible.slug = session_info::Info::visible_task_slug("visible-task");
    app.navigation_tabs.project_path = Some("/project".to_string());
    app.session_state.active_session_id = Some("main-session".to_string());
    app.session_state.sessions = vec![session_info("main-session", "Main", "/project")];
    app.chat_view.chat = vec![chat_message(ChatRole::User, "keep this chat")];
    app.chat_view.chat_message_ids = vec![Some("main-message".to_string())];
    app.get_session_runtime_mut("main-session").request_lifecycle.is_requesting = true;

    let _ =
        super::update(&mut app, super::ChatMessage::VisibleTaskSessionLoaded(Ok(visible.clone())));
    let _ = super::update(&mut app, super::ChatMessage::VisibleTaskSessionLoaded(Ok(visible)));

    assert_eq!(app.session_state.active_session_id.as_deref(), Some("main-session"));
    assert_eq!(app.chat_view.chat[0].content, "keep this chat");
    assert_eq!(app.chat_view.chat_message_ids, vec![Some("main-message".to_string())]);
    assert!(app.get_session_runtime("main-session").request_lifecycle.is_requesting);
    assert_eq!(app.session_state.sessions.iter().filter(|info| info.id == "visible-task").count(), 1);
    assert_eq!(
        app.workspace_state.project_sessions["/project"].iter().filter(|info| info.id == "visible-task").count(),
        1
    );
}

#[test]
fn update_unhandled_message_leaves_chat_state_unchanged() {
    let (mut app, _task) = crate::app::App::new();
    app.chat_view.chat = vec![chat_message(ChatRole::User, "existing")];

    let _task = super::update(
        &mut app,
        super::ChatMessage::InputChanged("ignored by stream router".to_string()),
    );

    assert_eq!(app.chat_view.chat.len(), 1);
    assert_eq!(app.chat_view.chat[0].content, "existing");
}

#[test]
fn desktop_first_response_late_success_publishes_to_the_original_project_once() {
    let (mut app, _) = crate::app::App::new();
    app.navigation_tabs.project_path = Some("/other".into());
    app.session_state.active_session_id = Some("current".into());
    app.session_state.sessions = vec![session_info("current", "Current", "/other")];
    app.chat_view.chat = vec![chat_message(ChatRole::User, "current chat")];

    let runtime = app.get_session_runtime_mut("draft");
    runtime.draft = Some(crate::app::state::SessionDraft {
        directory: "/project/worktree".into(),
        project_path: "/project".into(),
        history_len: 0,
    });
    runtime.request_lifecycle.active_agent_request = Some(agent_request(7, "draft"));
    runtime.request_lifecycle.is_requesting = true;
    app.store_session_chat_snapshot(
        "draft".into(),
        crate::app::session::shared_chat_messages(vec![
            chat_message(ChatRole::User, "首轮问题"),
            chat_message(ChatRole::Assistant, "首轮回复"),
        ]),
        vec![None, None],
    );

    let publish = super::ChatMessage::DraftResponseReloaded {
        id: 7,
        usage: Default::default(),
        assistant_message_id: Some("assistant-id".into()),
        parent_message_id: Some("user-id".into()),
        publication: Some(Box::new(super::draft::DraftPublication {
            info: session_info("draft", "", "/project/worktree"),
            preview: Some(session_ui_types::ChatSessionMeta {
                id: "draft".into(),
                title: String::new(),
                updated_ms: 20,
                message_count: 2,
                call_count: 0,
                last_content: Some("首轮问题".into()),
            }),
        })),
    };
    let _ = super::update(&mut app, publish.clone());
    let _ = super::update(&mut app, publish);

    assert_eq!(app.session_state.active_session_id.as_deref(), Some("current"));
    assert_eq!(app.session_state.sessions.len(), 1);
    assert_eq!(app.chat_view.chat[0].content, "current chat");
    let published = &app.workspace_state.project_sessions["/project"];
    assert_eq!(published.len(), 1);
    assert_eq!(published[0].id, "draft");
    assert!(published[0].title.is_empty());
    assert_eq!(crate::app::i18n::other::session_title(&app, "draft", ""), "首轮问题");
    assert_eq!(
        app.cached_chat_message_ids("draft").unwrap(),
        vec![Some("user-id".into()), Some("assistant-id".into())],
    );
}

#[test]
fn plan_run_first_response_start_updates_only_the_runtime_projection() {
    let (mut app, _) = crate::app::App::new();
    let parent = session_info("parent", "Main", "/project");
    app.session_state.sessions = vec![parent.clone()];
    app.workspace_state.project_sessions.insert("/project".into(), vec![parent]);
    app.get_session_runtime_mut("parent").request_lifecycle.active_agent_request = Some(agent_request(42, "parent"));

    let _ = super::update(
        &mut app,
        super::ChatMessage::PlanRunSessionStarted {
            title_feedback: None,
            parent_request_id: 42,
            parent_session_id: "parent".into(),
            session_id: "plan-child".into(),
            title: "Planning label".into(),
            directory: "/project".into(),
            query: "plan this".into(),
        },
    );

    assert_eq!(app.session_state.sessions.len(), 1);
    assert_eq!(app.workspace_state.project_sessions["/project"].len(), 1);
    assert!(app.get_session_runtime("plan-child").request_lifecycle.is_requesting);
    let chat = app.cached_chat_messages("plan-child").unwrap();
    assert_eq!(chat[0].content, "plan this");

    let _ = super::update(
        &mut app,
        super::ChatMessage::PlanRunSessionDelta {
            session_id: "plan-child".into(),
            delta: "partial output".into(),
        },
    );
    let _ =
        super::update(&mut app, super::ChatMessage::PlanRunSessionCancelled("plan-child".into()));
    let runtime = app.get_session_runtime("plan-child");
    assert!(!runtime.request_lifecycle.is_requesting);
    assert!(!runtime.stream_save.flush_scheduled);
    assert!(runtime.stream_save.in_flight.is_none());
    assert!(runtime.stream_save.dirty_records.is_empty());
    let chat = app.cached_chat_messages("plan-child").unwrap();
    assert_eq!(chat[1].content, "partial output");
}
