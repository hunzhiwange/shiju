use crate::app::models::{ChatMessage as ModelChatMessage, ChatRole};

fn chat_message(role: ChatRole, content: &str) -> ModelChatMessage {
    ModelChatMessage { role, content: content.to_string(), model: None, think_timing: Vec::new(), processing_timing: None }
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

fn agent_request(id: u64, session_id: &str) -> crate::app::state::AgentRequest {
    crate::app::state::AgentRequest {
        id,
        session: session_id.to_string(),
        query: "query".to_string(),
        root: None,
        model: None,
        reasoning_effort: None,
        acp_test: false,
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

    let _parent_cancelled =
        super::update(&mut app, super::ChatMessage::AgentStreamCancelled(42));

    let parent = app.get_session_runtime("main-session");
    assert!(!parent.is_requesting);
    assert!(parent.active_agent_request.is_none());
    assert!(!parent.has_unseen_success);
    assert!(app.chat.iter().all(|message| message.role != ChatRole::System));
}

#[test]
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
    use std::cell::RefCell;
    use std::path::PathBuf;
    use std::rc::Rc;

    let session =
        super::load_session_or_default(&crate::app::App::new().0, "session-1".to_string());
    let observed = Rc::new(RefCell::new(None));
    let hook_observed = Rc::clone(&observed);

    let result = super::with_save_session_hook(
        move |session, directory| {
            *hook_observed.borrow_mut() = Some((session.id.clone(), directory.map(str::to_string)));
            Some(PathBuf::from("/saved/session.json"))
        },
        || super::call_save_session_hook(&session, Some("/project")),
    );

    assert_eq!(result, Some(Some(PathBuf::from("/saved/session.json"))));
    assert_eq!(
        observed.borrow().as_ref(),
        Some(&("session-1".to_string(), Some("/project".to_string())))
    );
    assert!(super::call_save_session_hook(&session, None).is_none());
}

#[test]
fn update_rejects_empty_generated_title_without_mutating_sessions() {
    let (mut app, _task) = crate::app::App::new();
    app.sessions = vec![session_info("session-1", "Original", "/project")];
    app.project_sessions.insert(
        "project-1".to_string(),
        vec![session_info("session-1", "Project original", "/project")],
    );

    let _task = super::update(
        &mut app,
        super::ChatMessage::SessionTitleGenerated("session-1".to_string(), String::new()),
    );

    assert_eq!(app.sessions[0].title, "Original");
    assert_eq!(app.project_sessions["project-1"][0].title, "Project original");
}

#[test]
fn update_generated_title_updates_loaded_and_project_session_copies() {
    let (mut app, _task) = crate::app::App::new();
    app.sessions = vec![session_info("session-1", "Original", "/project")];
    app.project_sessions.insert(
        "project-1".to_string(),
        vec![
            session_info("session-1", "Project original", "/project"),
            session_info("session-2", "Other", "/project"),
        ],
    );

    let _task = super::update(
        &mut app,
        super::ChatMessage::SessionTitleGenerated(
            "session-1".to_string(),
            "Generated title".to_string(),
        ),
    );

    assert_eq!(app.sessions[0].title, "Generated title");
    assert_eq!(app.project_sessions["project-1"][0].title, "Generated title");
    assert_eq!(app.project_sessions["project-1"][1].title, "Other");
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
fn maybe_generate_session_title_returns_none_without_active_default_session() {
    let (mut app, _task) = crate::app::App::new();

    let no_active = super::maybe_generate_session_title(&app);
    assert_eq!(no_active.units(), 0);

    app.active_session_id = Some("session-1".to_string());
    let missing_session = super::maybe_generate_session_title(&app);
    assert_eq!(missing_session.units(), 0);

    app.sessions = vec![session_info("session-1", "Named session", "/project")];
    app.chat = vec![chat_message(ChatRole::User, "first question")];
    let named_session = super::maybe_generate_session_title(&app);
    assert_eq!(named_session.units(), 0);

    app.sessions[0].title = "新会话".to_string();
    app.chat.push(chat_message(ChatRole::User, "second question"));
    let multiple_user_messages = super::maybe_generate_session_title(&app);
    assert_eq!(multiple_user_messages.units(), 0);
}

#[test]
fn maybe_generate_session_title_schedules_task_for_single_user_default_title() {
    let (mut app, _task) = crate::app::App::new();
    app.active_session_id = Some("session-1".to_string());
    app.sessions = vec![session_info("session-1", "新会话", "/project")];
    app.chat = vec![chat_message(ChatRole::User, "first question")];
    app.auto_model = false;
    app.model = "openai/gpt-5".to_string();

    let task = super::maybe_generate_session_title(&app);

    assert!(iced_runtime::task::into_stream(task.collect()).is_some());
}

#[test]
#[cfg(not(target_arch = "wasm32"))]
fn stream_session_saves_coalesce_to_latest_snapshot_while_in_flight() {
    use std::sync::{mpsc, Arc, Mutex};
    use std::time::Duration;

    let coordinator = Arc::new(super::StreamSessionSaveCoordinator::default());
    let saved_contents = Arc::new(Mutex::new(Vec::new()));
    let other_session_saved = Arc::new(Mutex::new(false));
    let (first_save_started_tx, first_save_started_rx) = mpsc::channel();
    let (release_first_save_tx, release_first_save_rx) = mpsc::channel();

    let snapshot = |content: &str| {
        crate::app::models::ChatSession {
            id: "session-1".to_string(),
            title: "Session".to_string(),
            messages: vec![chat_message(ChatRole::Assistant, content)],
            message_ids: vec![None],
            calls: Vec::new(),
            steps: Vec::new(),
            created_ms: 1,
            updated_ms: 2,
        }
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
                    .push((
                        session.messages[0].content.clone(),
                        directory.map(str::to_string),
                    ));
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

    first_save_started_rx
        .recv_timeout(Duration::from_secs(10))
        .expect("first save should start");

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

    coordinator.save(snapshot.clone(), None, move |_, _| {
        *first_count.lock().expect("save count lock") += 1;
        Err("gateway request failed: 413 Payload Too Large".to_string())
    });

    let second_count = Arc::clone(&saved_count);
    coordinator.save(snapshot, None, move |_, _| {
        *second_count.lock().expect("save count lock") += 1;
        Ok(None)
    });

    assert_eq!(*saved_count.lock().expect("save count lock"), 1);
}

#[test]
fn maybe_generate_session_title_accepts_multiline_initial_input_title() {
    let (mut app, _task) = crate::app::App::new();
    app.active_session_id = Some("session-1".to_string());
    app.sessions = vec![session_info("session-1", "first line", "/project")];
    app.chat = vec![chat_message(ChatRole::User, "first line\nsecond line")];

    let task = super::maybe_generate_session_title(&app);

    assert!(iced_runtime::task::into_stream(task.collect()).is_some());
}
