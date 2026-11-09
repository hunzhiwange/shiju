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

fn session_info(id: &str, title: &str, directory: &str) -> vw_shared::session::info::Info {
    vw_shared::session::info::Info {
        id: id.to_string(),
        slug: format!("{id}-slug"),
        project_id: "project-1".to_string(),
        directory: directory.to_string(),
        parent_id: None,
        summary: None,
        share: None,
        title: title.to_string(),
        version: "0.0.0".to_string(),
        time: vw_shared::session::info::TimeInfo {
            created: 10,
            updated: 20,
            compacting: None,
            archived: None,
        },
        permission: None,
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
        full_access_enabled: false,
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
    app.sessions = vec![info.clone()];
    app.project_sessions.insert("/project".into(), vec![info.clone()]);
    info.summary = Some(vw_shared::session::info::Summary {
        additions: 1434,
        deletions: 7,
        files: 16,
        diffs: None,
    });
    info.time.updated += 1;
    let _ = super::update(&mut app, super::ChatMessage::SessionSummaryReloaded(Ok(info)));
    for sessions in [&app.sessions, &app.project_sessions["/project"]] {
        let summary = sessions[0].summary.as_ref().unwrap();
        assert_eq!((summary.additions, summary.deletions, summary.files), (1434, 7, 16));
    }
}

#[test]
fn stream_diff_summary_updates_the_request_session_in_both_lists() {
    let (mut app, _) = crate::app::App::new();
    let info = session_info("edited", "Edited", "/project");
    app.sessions = vec![info.clone(), session_info("other", "Other", "/project")];
    app.project_sessions.insert("/project".into(), app.sessions.clone());
    app.active_session_id = Some("other".into());
    app.get_session_runtime_mut("edited").active_agent_request = Some(agent_request(42, "edited"));
    let summary =
        vw_shared::session::info::Summary { additions: 2, deletions: 1, files: 1, diffs: None };

    let _ = super::update(
        &mut app,
        super::ChatMessage::AgentSessionDiffSummary(41, "edited".into(), summary.clone()),
    );
    assert!(app.sessions[0].summary.is_none());
    app.get_session_runtime_mut("edited").active_agent_request = None;
    let _ = super::update(
        &mut app,
        super::ChatMessage::AgentSessionDiffSummary(42, "edited".into(), summary),
    );

    for sessions in [&app.sessions, &app.project_sessions["/project"]] {
        let summary = sessions[0].summary.as_ref().unwrap();
        assert_eq!((summary.additions, summary.deletions), (2, 1));
        assert!(sessions[1].summary.is_none());
    }
}

#[test]
fn desktop_plan_run_waiting_and_cancelled_events_keep_session_lifecycle_consistent_in_chat_stream()
{
    let (mut app, _task) = crate::app::App::new();
    app.active_session_id = Some("main-session".to_string());
    app.sessions = vec![
        session_info("main-session", "Main", "/project"),
        session_info("task-2-sub-9", "Child", "/project"),
    ];
    app.chat = vec![chat_message(ChatRole::User, "run workflow")];
    {
        let parent = app.get_session_runtime_mut("main-session");
        parent.is_requesting = true;
        parent.active_agent_request = Some(agent_request(42, "main-session"));
    }
    app.get_session_runtime_mut("task-2-sub-9").is_requesting = true;

    let _waiting = super::update(
        &mut app,
        super::ChatMessage::PlanRunSessionWaiting("task-2-sub-9".to_string()),
    );

    assert!(app.get_session_runtime("task-2-sub-9").is_requesting);
    assert!(app.get_session_runtime("main-session").is_requesting);

    let _child_cancelled = super::update(
        &mut app,
        super::ChatMessage::PlanRunSessionCancelled("task-2-sub-9".to_string()),
    );

    assert!(!app.get_session_runtime("task-2-sub-9").is_requesting);
    assert!(app.get_session_runtime("main-session").is_requesting);

    let _parent_cancelled = super::update(&mut app, super::ChatMessage::AgentStreamCancelled(42));

    let parent = app.get_session_runtime("main-session");
    assert!(!parent.is_requesting);
    assert!(parent.active_agent_request.is_none());
    assert!(!parent.has_unseen_success);
    assert!(app.chat.iter().all(|message| message.role != ChatRole::System));
}

#[test]
fn stream_terminal_flushes_pending_active_delta_immediately() {
    fn active_app(request_id: u64) -> crate::app::App {
        let (mut app, _) = crate::app::App::new();
        let session_id = format!("session-{request_id}");
        app.active_session_id = Some(session_id.clone());
        app.sessions = vec![session_info(&session_id, "Session", "/project")];
        app.chat =
            vec![chat_message(ChatRole::User, "question"), chat_message(ChatRole::Assistant, "")];
        app.chat_message_ids = vec![None, None];
        let runtime = app.get_session_runtime_mut(&session_id);
        runtime.is_requesting = true;
        runtime.active_agent_request = Some(agent_request(request_id, &session_id));
        app
    }

    let mut done_app = active_app(51);
    let done_session = done_app.active_session_id.clone().unwrap();
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
    assert_eq!(done_app.chat[1].content, "done tail");
    assert!(done_app.get_session_runtime(&done_session).pending_stream_delta.is_empty());
    assert!(!done_app.get_session_runtime(&done_session).stream_delta_flush_scheduled);
    assert!(!done_app.get_session_runtime(&done_session).stream_save_flush_scheduled);
    assert_eq!(
        super::stream_events::flush_agent_stream_delta(&mut done_app, done_session).units(),
        0
    );

    let mut cancelled_app = active_app(52);
    let cancelled_session = cancelled_app.active_session_id.clone().unwrap();
    let _ = super::stream_events::queue_agent_stream_delta(
        &mut cancelled_app,
        52,
        "cancelled tail".to_string(),
    );
    let _ = super::update(&mut cancelled_app, super::ChatMessage::AgentStreamCancelled(52));
    assert_eq!(cancelled_app.chat[1].content, "cancelled tail");
    assert!(cancelled_app.get_session_runtime(&cancelled_session).pending_stream_delta.is_empty());
    assert!(!cancelled_app.get_session_runtime(&cancelled_session).stream_delta_flush_scheduled);
    assert!(!cancelled_app.get_session_runtime(&cancelled_session).stream_save_flush_scheduled);

    let mut error_app = active_app(53);
    let error_session = error_app.active_session_id.clone().unwrap();
    let _ = super::stream_events::queue_agent_stream_delta(
        &mut error_app,
        53,
        "error tail".to_string(),
    );
    let _ = super::update(
        &mut error_app,
        super::ChatMessage::AgentStreamError(53, "failed".to_string()),
    );
    assert_eq!(error_app.chat[1].content, "error tail");
    assert!(error_app.get_session_runtime(&error_session).pending_stream_delta.is_empty());
    assert!(!error_app.get_session_runtime(&error_session).stream_delta_flush_scheduled);
    assert!(!error_app.get_session_runtime(&error_session).stream_save_flush_scheduled);
}

#[test]
// 保存目录优先采用会话元数据，空白路径不能覆盖有效回退目录。
fn session_directory_for_save_skips_blank_session_directory() {
    let (mut app, _task) = crate::app::App::new();
    app.sessions = vec![session_info("session-1", "Title", "  \n")];
    app.project_path = Some("/project".to_string());

    assert_eq!(super::session_directory_for_save(&app, "session-1").as_deref(), Some("/project"));
}

#[test]
fn session_directory_for_save_prefers_known_session_directory() {
    let (mut app, _task) = crate::app::App::new();
    app.sessions = vec![session_info("session-1", "Title", "  /session/root  ")];
    app.project_path = Some("/project/root".to_string());

    assert_eq!(
        super::session_directory_for_save(&app, "session-1").as_deref(),
        Some("  /session/root  ")
    );
}

#[test]
fn session_directory_for_save_rejects_blank_fallbacks() {
    let (mut app, _task) = crate::app::App::new();
    app.sessions = vec![session_info("session-1", "Title", " \n\t ")];
    app.project_path = Some("   ".to_string());

    assert_eq!(super::session_directory_for_save(&app, "session-1"), None);
}

#[test]
// 缓存消息标识与消息数量不一致时必须重建，避免后续流事件关联到错误消息。
fn load_session_or_default_replaces_mismatched_cached_message_ids() {
    let (mut app, _task) = crate::app::App::new();
    app.sessions = vec![session_info("session-1", "Title", "/project")];
    app.store_session_chat_snapshot(
        "session-1".to_string(),
        crate::app::session::shared_chat_messages(vec![
            chat_message(ChatRole::User, "question"),
            chat_message(ChatRole::Assistant, "answer"),
        ]),
        vec![Some("only-one-id".to_string())],
    );

    let loaded = super::load_session_or_default(&app, "session-1".to_string());

    assert_eq!(loaded.messages.len(), 2);
    assert_eq!(loaded.message_ids, vec![None, None]);
    assert!(loaded.calls.is_empty());
    assert!(loaded.steps.is_empty());
}

#[test]
fn load_session_or_default_keeps_inactive_session_metadata_without_cache() {
    let (mut app, _task) = crate::app::App::new();
    app.sessions = vec![session_info("session-1", "Stored title", "/project")];

    let loaded = super::load_session_or_default(&app, "session-1".to_string());

    assert_eq!(loaded.id, "session-1");
    assert_eq!(loaded.title, "Stored title");
    assert_eq!(loaded.created_ms, 10);
    assert_eq!(loaded.updated_ms, 20);
    assert!(loaded.messages.is_empty());
    assert!(loaded.message_ids.is_empty());
    assert!(loaded.calls.is_empty());
    assert!(loaded.steps.is_empty());
}

#[test]
fn load_session_or_default_uses_active_chat_ids_calls_and_steps() {
    let (mut app, _task) = crate::app::App::new();
    app.sessions = vec![session_info("session-1", "Active", "/project")];
    app.active_session_id = Some("session-1".to_string());
    app.chat = vec![chat_message(ChatRole::User, "question")];
    app.chat_message_ids = vec![Some("message-1".to_string())];

    let loaded = super::load_session_or_default(&app, "session-1".to_string());

    assert_eq!(loaded.messages.len(), 1);
    assert_eq!(loaded.messages[0].role, ChatRole::User);
    assert_eq!(loaded.messages[0].content, "question");
    assert_eq!(loaded.message_ids, app.chat_message_ids);
    assert_eq!(loaded.calls.len(), app.active_session_view_state.calls.len());
    assert_eq!(loaded.steps.len(), app.active_session_view_state.steps.len());
}

#[test]
fn load_session_or_default_preserves_matching_cached_message_ids() {
    let (mut app, _task) = crate::app::App::new();
    app.sessions = vec![session_info("session-1", "Cached", "/project")];
    app.store_session_chat_snapshot(
        "session-1".to_string(),
        crate::app::session::shared_chat_messages(vec![chat_message(ChatRole::User, "cached")]),
        vec![Some("cached-id".to_string())],
    );

    let loaded = super::load_session_or_default(&app, "session-1".to_string());

    assert_eq!(loaded.messages[0].content, "cached");
    assert_eq!(loaded.message_ids, vec![Some("cached-id".to_string())]);
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
fn desktop_first_response_title_updates_only_untitled_copies_for_its_project() {
    let (mut app, _task) = crate::app::App::new();
    let mut foreign = session_info("session-1", "", "/other");
    foreign.project_id = "foreign-project".to_string();
    app.active_session_id = Some("session-2".to_string());
    app.sessions = vec![
        session_info("session-1", "手工改名", "/project"),
        session_info("session-2", "当前会话", "/project"),
    ];
    app.project_sessions
        .insert("/project".to_string(), vec![session_info("session-1", "", "/project")]);
    app.project_sessions.insert("/other".to_string(), vec![foreign]);

    let task = super::update(
        &mut app,
        super::ChatMessage::PublishedSessionTitleLoaded(session_info(
            "session-1",
            "服务端标题",
            "/project",
        )),
    );

    assert_eq!(task.units(), 0);
    assert_eq!(app.sessions[0].title, "手工改名");
    assert_eq!(app.sessions[1].title, "当前会话");
    assert_eq!(app.project_sessions["/project"][0].title, "服务端标题");
    assert!(app.project_sessions["/other"][0].title.is_empty());
}

#[test]
fn externally_created_session_does_not_replace_active_session() {
    let (mut app, _task) = crate::app::App::new();
    let mut visible = session_info("visible-task", "Visible task", "/project");
    visible.parent_id = Some("main-session".to_string());
    visible.slug = vw_shared::session::info::Info::visible_task_slug("visible-task");
    app.project_path = Some("/project".to_string());
    app.active_session_id = Some("main-session".to_string());
    app.sessions = vec![session_info("main-session", "Main", "/project")];
    app.chat = vec![chat_message(ChatRole::User, "keep this chat")];
    app.chat_message_ids = vec![Some("main-message".to_string())];
    app.get_session_runtime_mut("main-session").is_requesting = true;

    let _ =
        super::update(&mut app, super::ChatMessage::VisibleTaskSessionLoaded(Ok(visible.clone())));
    let _ = super::update(&mut app, super::ChatMessage::VisibleTaskSessionLoaded(Ok(visible)));

    assert_eq!(app.active_session_id.as_deref(), Some("main-session"));
    assert_eq!(app.chat[0].content, "keep this chat");
    assert_eq!(app.chat_message_ids, vec![Some("main-message".to_string())]);
    assert!(app.get_session_runtime("main-session").is_requesting);
    assert_eq!(app.sessions.iter().filter(|info| info.id == "visible-task").count(), 1);
    assert_eq!(
        app.project_sessions["/project"].iter().filter(|info| info.id == "visible-task").count(),
        1
    );
}

#[test]
fn update_unhandled_message_leaves_chat_state_unchanged() {
    let (mut app, _task) = crate::app::App::new();
    app.chat = vec![chat_message(ChatRole::User, "existing")];

    let _task = super::update(
        &mut app,
        super::ChatMessage::InputChanged("ignored by stream router".to_string()),
    );

    assert_eq!(app.chat.len(), 1);
    assert_eq!(app.chat[0].content, "existing");
}

#[test]
#[cfg(not(target_arch = "wasm32"))]
// 同一会话保存进行中时只保留最新快照，防止旧写入在完成后覆盖新状态。
fn stream_session_saves_coalesce_to_latest_snapshot_while_in_flight() {
    use std::sync::{Arc, Mutex, mpsc};
    use std::time::Duration;

    let coordinator = Arc::new(super::StreamSessionSaveCoordinator::default());
    let saved_contents = Arc::new(Mutex::new(Vec::new()));
    let other_session_saved = Arc::new(Mutex::new(false));
    let (first_save_started_tx, first_save_started_rx) = mpsc::channel();
    let (release_first_save_tx, release_first_save_rx) = mpsc::channel();

    let snapshot = |content: &str| crate::app::models::ChatSession {
        id: "session-1".to_string(),
        title: "Session".to_string(),
        messages: vec![chat_message(ChatRole::Assistant, content)],
        message_ids: vec![None],
        calls: Vec::new(),
        steps: Vec::new(),
        created_ms: 1,
        updated_ms: 2,
    };
    let first_snapshot = snapshot("first");
    let middle_snapshot = snapshot("middle");
    let latest_snapshot = snapshot("latest");
    let mut other_session = snapshot("other");
    other_session.id = "session-2".to_string();

    let first_coordinator = Arc::clone(&coordinator);
    let first_saved_contents = Arc::clone(&saved_contents);
    let first_save = std::thread::spawn(move || {
        first_coordinator.save(
            first_snapshot,
            Some("/project".to_string()),
            move |session, directory| {
                first_saved_contents
                    .lock()
                    .expect("saved contents lock")
                    .push((session.messages[0].content.clone(), directory.map(str::to_string)));
                if session.messages[0].content == "first" {
                    first_save_started_tx.send(()).expect("signal first save start");
                    release_first_save_rx
                        .recv_timeout(Duration::from_secs(10))
                        .expect("release first save");
                }
                Ok(Some(std::path::PathBuf::from(&session.id)))
            },
        );
    });

    first_save_started_rx.recv_timeout(Duration::from_secs(10)).expect("first save should start");

    coordinator.save(middle_snapshot, Some("/middle".to_string()), |_, _| {
        panic!("an in-flight save must use the active runner")
    });
    coordinator.save(latest_snapshot, Some("/latest".to_string()), |_, _| {
        panic!("an in-flight save must use the active runner")
    });
    let other_session_saved_by_callback = Arc::clone(&other_session_saved);
    coordinator.save(other_session, Some("/other".to_string()), move |session, directory| {
        assert_eq!(session.id, "session-2");
        assert_eq!(directory, Some("/other"));
        *other_session_saved_by_callback.lock().expect("other session saved lock") = true;
        Ok(Some(std::path::PathBuf::from(&session.id)))
    });
    assert!(*other_session_saved.lock().expect("other session saved lock"));

    release_first_save_tx.send(()).expect("release first save");
    first_save.join().expect("first save thread");

    assert_eq!(
        saved_contents.lock().expect("saved contents lock").as_slice(),
        [
            ("first".to_string(), Some("/project".to_string())),
            ("latest".to_string(), Some("/latest".to_string())),
        ]
    );
}

#[test]
#[cfg(not(target_arch = "wasm32"))]
fn stream_session_save_retains_latest_snapshot_after_failure() {
    use std::sync::{Arc, Mutex, mpsc};
    use std::time::Duration;

    let coordinator = Arc::new(super::StreamSessionSaveCoordinator::default());
    let attempts = Arc::new(Mutex::new(Vec::new()));
    let (first_save_started_tx, first_save_started_rx) = mpsc::channel();
    let (release_first_save_tx, release_first_save_rx) = mpsc::channel();
    let snapshot = |content: &str| crate::app::models::ChatSession {
        id: "retry-session".to_string(),
        title: "Session".to_string(),
        messages: vec![chat_message(ChatRole::Assistant, content)],
        message_ids: vec![None],
        calls: Vec::new(),
        steps: Vec::new(),
        created_ms: 1,
        updated_ms: 2,
    };

    let first_coordinator = Arc::clone(&coordinator);
    let first_attempts = Arc::clone(&attempts);
    let first = std::thread::spawn(move || {
        first_coordinator.save(snapshot("first"), None, move |session, _| {
            first_attempts.lock().expect("attempts lock").push(session.messages[0].content.clone());
            first_save_started_tx.send(()).expect("signal first save start");
            release_first_save_rx
                .recv_timeout(Duration::from_secs(10))
                .expect("release first save");
            Err("gateway request failed: 503 Service Unavailable".to_string())
        })
    });
    first_save_started_rx.recv_timeout(Duration::from_secs(10)).expect("first save should start");

    let queued = coordinator.save(snapshot("queued-latest"), None, |_, _| {
        panic!("an in-flight save must use the active runner")
    });
    assert_eq!(queued, super::StreamSessionSaveResult::Coalesced);
    release_first_save_tx.send(()).expect("release first save");
    assert!(matches!(
        first.join().expect("first save thread"),
        super::StreamSessionSaveResult::Deferred(_)
    ));
    assert_eq!(attempts.lock().expect("attempts lock").as_slice(), ["first"]);

    let retry_attempts = Arc::clone(&attempts);
    let retried = coordinator.save(snapshot("newest"), None, move |session, _| {
        retry_attempts.lock().expect("attempts lock").push(session.messages[0].content.clone());
        Ok(Some(std::path::PathBuf::from(&session.id)))
    });

    assert_eq!(retried, super::StreamSessionSaveResult::Committed);
    assert_eq!(attempts.lock().expect("attempts lock").as_slice(), ["first", "newest"]);
}

#[test]
#[cfg(not(target_arch = "wasm32"))]
fn stream_session_save_ack_requires_atomic_commit() {
    let coordinator = super::StreamSessionSaveCoordinator::default();
    let snapshot = crate::app::models::ChatSession {
        id: "ack-session".to_string(),
        title: "Session".to_string(),
        messages: vec![chat_message(ChatRole::Assistant, "answer")],
        message_ids: vec![None],
        calls: Vec::new(),
        steps: Vec::new(),
        created_ms: 1,
        updated_ms: 2,
    };

    let failed =
        coordinator.save(snapshot.clone(), None, |_, _| Err("database is locked".to_string()));
    assert!(matches!(&failed, super::StreamSessionSaveResult::Deferred(_)));
    assert!(matches!(super::stream_session_save_message(Some(failed)), crate::app::Message::None));

    let committed = coordinator.save(snapshot, None, |_, _| Ok(None));
    assert_eq!(committed, super::StreamSessionSaveResult::Committed);
    assert!(matches!(
        super::stream_session_save_message(Some(committed)),
        crate::app::Message::Chat(super::ChatMessage::SessionSaveAck)
    ));
}

#[test]
#[cfg(not(target_arch = "wasm32"))]
fn stream_session_save_stops_retries_after_payload_too_large() {
    use std::sync::{Arc, Mutex};

    let coordinator = super::StreamSessionSaveCoordinator::default();
    let saved_count = Arc::new(Mutex::new(0));
    let first_count = Arc::clone(&saved_count);
    let snapshot = crate::app::models::ChatSession {
        id: "oversized-session".to_string(),
        title: "Session".to_string(),
        messages: vec![chat_message(ChatRole::Assistant, "answer")],
        message_ids: vec![None],
        calls: Vec::new(),
        steps: Vec::new(),
        created_ms: 1,
        updated_ms: 2,
    };

    let failed = coordinator.save(snapshot.clone(), None, move |_, _| {
        *first_count.lock().expect("save count lock") += 1;
        Err("gateway request failed: 413 Payload Too Large".to_string())
    });
    assert!(matches!(&failed, super::StreamSessionSaveResult::PermanentlyFailed(_)));
    assert!(matches!(
        super::stream_session_save_message(Some(failed)),
        crate::app::Message::Chat(super::ChatMessage::SessionSaveFailed(_))
    ));

    let second_count = Arc::clone(&saved_count);
    let blocked = coordinator.save(snapshot, None, move |_, _| {
        *second_count.lock().expect("save count lock") += 1;
        Ok(None)
    });

    assert_eq!(blocked, super::StreamSessionSaveResult::Blocked);
    assert!(matches!(super::stream_session_save_message(Some(blocked)), crate::app::Message::None));
    assert_eq!(*saved_count.lock().expect("save count lock"), 1);
}

#[test]
fn desktop_first_response_late_success_publishes_to_the_original_project_once() {
    let (mut app, _) = crate::app::App::new();
    app.project_path = Some("/other".into());
    app.active_session_id = Some("current".into());
    app.sessions = vec![session_info("current", "Current", "/other")];
    app.chat = vec![chat_message(ChatRole::User, "current chat")];

    let runtime = app.get_session_runtime_mut("draft");
    runtime.draft = Some(crate::app::state::SessionDraft {
        directory: "/project/worktree".into(),
        project_path: "/project".into(),
        history_len: 0,
    });
    runtime.active_agent_request = Some(agent_request(7, "draft"));
    runtime.is_requesting = true;
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
            preview: Some(vw_shared::session::ui_types::ChatSessionMeta {
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

    assert_eq!(app.active_session_id.as_deref(), Some("current"));
    assert_eq!(app.sessions.len(), 1);
    assert_eq!(app.chat[0].content, "current chat");
    let published = &app.project_sessions["/project"];
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
    app.sessions = vec![parent.clone()];
    app.project_sessions.insert("/project".into(), vec![parent]);
    app.get_session_runtime_mut("parent").active_agent_request = Some(agent_request(42, "parent"));

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

    assert_eq!(app.sessions.len(), 1);
    assert_eq!(app.project_sessions["/project"].len(), 1);
    assert!(app.get_session_runtime("plan-child").is_requesting);
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
    assert!(!runtime.is_requesting);
    assert!(!runtime.stream_save_flush_scheduled);
    assert!(runtime.stream_save_in_flight.is_none());
    assert!(runtime.stream_save_dirty_records.is_empty());
    let chat = app.cached_chat_messages("plan-child").unwrap();
    assert_eq!(chat[1].content, "partial output");
}
