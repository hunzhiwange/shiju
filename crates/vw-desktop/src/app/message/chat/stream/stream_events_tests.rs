#[test]
fn stream_events_tests_module_is_wired() {
    assert!(module_path!().ends_with("stream_events_tests"));
}

use crate::app::components::chat_panel::virtualizer::{ChatScrollMode, ChatVirtualizer};
use crate::app::models::{ChatMessage, ChatRole, ThinkTiming};
use crate::app::state::{
    AcpHistoryReplayMode, AgentRequest, ChatSendBehavior, QuickMessageHandoffState,
};
use crate::app::{Message, QueueItem};
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

fn message(role: ChatRole, content: &str) -> ChatMessage {
    ChatMessage {
        role,
        content: content.to_string(),
        model: None,
        think_timing: Vec::new(),
        processing_timing: None,
    }
}

async fn collect_task_messages(task: iced::Task<Message>) -> Vec<Message> {
    use iced_runtime::futures::futures::StreamExt;

    let Some(mut stream) = iced_runtime::task::into_stream(task.collect()) else {
        return Vec::new();
    };
    while let Some(action) = stream.next().await {
        if let iced_runtime::Action::Output(messages) = action {
            return messages;
        }
    }
    Vec::new()
}

fn queue_item(send_behavior: ChatSendBehavior) -> QueueItem {
    QueueItem {
        created_ms: 1,
        query: "query".to_string(),
        attachments: Vec::new(),
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
        acp_history_mode: AcpHistoryReplayMode::Discard,
        acp_recent_count: 0,
        full_access_enabled: false,
        send_behavior,
        request_history_override: None,
        resume_history_only: false,
        workflow_mode_enabled: false,
        goal_mode_enabled: false,
    }
}

fn agent_request(id: u64, session_id: &str) -> AgentRequest {
    AgentRequest {
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
        acp_history_mode: AcpHistoryReplayMode::Discard,
        acp_recent_count: 0,
        full_access_enabled: false,
        resume_history_only: false,
        workflow_mode_enabled: false,
        goal_mode_enabled: false,
        history: Vec::new(),
    }
}

#[test]
fn background_stream_mutation_invalidates_cached_session_view() {
    let (mut app, _) = crate::app::App::new();
    app.active_session_id = Some("background".to_string());
    app.chat = vec![message(ChatRole::Assistant, "cached response")];
    app.stash_active_session_view();
    assert!(app.session_view_cache.contains("background"));

    app.active_session_id = Some("foreground".to_string());
    let _ = super::handle_agent_session_delta(
        &mut app,
        "background".to_string(),
        " updated".to_string(),
    );

    assert!(!app.session_view_cache.contains("background"));
}

#[tokio::test(flavor = "multi_thread")]
async fn generated_git_commit_message_fills_form_then_dispatches_commit_action() {
    let (mut app, _) = crate::app::App::new();
    app.project_path = Some("/workspace/project".to_string());
    app.quick_message_handoff = Some(QuickMessageHandoffState::WaitingForGitCommitMessage {
        request_id: 43,
        source_session_id: "session-1".to_string(),
        project_path: "/workspace/project".to_string(),
        action: crate::app::state::QuickMessageGitCommitAction::StageAllAndCommit,
    });
    let chat = vec![message(
        ChatRole::Assistant,
        "```json\n{\"title\":\"feat(chat): add quick actions\",\"summary\":\"Generate commit metadata with AI before submitting changes.\"}\n```",
    )];

    let task = super::complete_quick_message_handoff(&mut app, 43, "session-1", &chat)
        .expect("git commit handoff should complete");

    assert!(app.quick_message_handoff.is_none());
    assert_eq!(app.git_commit_type, None);
    assert_eq!(app.git_commit_scope, "");
    assert_eq!(app.git_commit_message, "feat(chat): add quick actions");
    assert_eq!(
        app.git_commit_description,
        "Generate commit metadata with AI before submitting changes."
    );
    assert_eq!(app.git_commit_description_editor.text(), app.git_commit_description);
    assert!(matches!(
        collect_task_messages(task).await.as_slice(),
        [Message::Git(crate::app::message::GitMessage::CommitAll)]
    ));

    app.quick_message_handoff = Some(QuickMessageHandoffState::WaitingForGitCommitMessage {
        request_id: 45,
        source_session_id: "session-1".to_string(),
        project_path: "/workspace/project".to_string(),
        action: crate::app::state::QuickMessageGitCommitAction::CommitSelected,
    });
    let task = super::complete_quick_message_handoff(&mut app, 45, "session-1", &chat)
        .expect("selected commit handoff should complete");
    assert!(matches!(
        collect_task_messages(task).await.as_slice(),
        [Message::Git(crate::app::message::GitMessage::CommitSelected)]
    ));
}

#[tokio::test(flavor = "multi_thread")]
async fn invalid_generated_git_commit_message_does_not_dispatch_commit() {
    let (mut app, _) = crate::app::App::new();
    app.quick_message_handoff = Some(QuickMessageHandoffState::WaitingForGitCommitMessage {
        request_id: 44,
        source_session_id: "session-1".to_string(),
        project_path: "/workspace/project".to_string(),
        action: crate::app::state::QuickMessageGitCommitAction::CommitSelected,
    });
    let chat = vec![message(ChatRole::Assistant, "{\"title\":\"\",\"summary\":\"\"}")];

    let task = super::complete_quick_message_handoff(&mut app, 44, "session-1", &chat)
        .expect("invalid result should produce warning task");
    let messages = collect_task_messages(task).await;

    assert!(app.quick_message_handoff.is_none());
    assert!(!app.git_commit_in_progress);
    assert!(messages.iter().all(|message| !matches!(message, Message::Git(_))));
    assert!(app.active_toast.as_ref().is_some_and(|toast| toast.message.contains("未执行提交")));
}

#[tokio::test(flavor = "multi_thread")]
async fn generated_git_commit_message_rejects_a_switched_worktree() {
    let (mut app, _) = crate::app::App::new();
    app.project_path = Some("/workspace/other".to_string());
    app.quick_message_handoff = Some(QuickMessageHandoffState::WaitingForGitCommitMessage {
        request_id: 46,
        source_session_id: "session-1".to_string(),
        project_path: "/workspace/project".to_string(),
        action: crate::app::state::QuickMessageGitCommitAction::StageAllAndCommit,
    });
    let chat = vec![message(
        ChatRole::Assistant,
        "{\"title\":\"feat: safe commit\",\"summary\":\"Keep the generated commit scoped.\"}",
    )];

    let task = super::complete_quick_message_handoff(&mut app, 46, "session-1", &chat)
        .expect("worktree mismatch should produce warning task");
    let messages = collect_task_messages(task).await;

    assert!(app.quick_message_handoff.is_none());
    assert!(!app.git_commit_in_progress);
    assert!(messages.iter().all(|message| !matches!(message, Message::Git(_))));
    assert!(app.active_toast.as_ref().is_some_and(|toast| toast.message.contains("工作树已切换")));
}

fn session_info(
    id: &str,
    directory: &str,
    parent_id: Option<&str>,
) -> vw_shared::session::info::Info {
    vw_shared::session::info::Info {
        id: id.to_string(),
        slug: id.to_string(),
        project_id: "project".to_string(),
        directory: directory.to_string(),
        parent_id: parent_id.map(str::to_string),
        summary: None,
        share: None,
        title: id.to_string(),
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
fn plan_run_session_started_registers_without_leaving_parent() {
    let (mut app, _) = crate::app::App::new();
    let directory = "/workspace/demo";
    app.project_path = Some(directory.to_string());
    app.active_session_id = Some("main-session".to_string());
    app.sessions = vec![session_info("main-session", directory, None)];
    app.chat = vec![message(ChatRole::User, "主会话任务")];
    app.chat_message_ids = vec![None];
    let parent_runtime = app.get_session_runtime_mut("main-session");
    parent_runtime.is_requesting = true;
    parent_runtime.active_agent_request = Some(agent_request(42, "main-session"));

    let _register_task = super::handle_plan_run_session_started(
        &mut app,
        42,
        "main-session".to_string(),
        "task-1-plan".to_string(),
        "规划 · task-1".to_string(),
        directory.to_string(),
        "实现需求".to_string(),
    );

    assert_eq!(app.active_session_id.as_deref(), Some("main-session"));
    let child = app
        .sessions
        .iter()
        .find(|session| session.id == "task-1-plan")
        .expect("planning child should be listed");
    assert_eq!(child.parent_id.as_deref(), Some("main-session"));
    assert_eq!(child.title, "规划 · task-1");
    assert!(child.is_plan_run());
    assert!(app.get_session_runtime("task-1-plan").is_requesting);
    assert_eq!(app.chat, vec![message(ChatRole::User, "主会话任务")]);
    assert_eq!(
        app.cached_chat_messages("task-1-plan")
            .expect("planning chat should be cached")
            .iter()
            .cloned()
            .collect::<Vec<_>>(),
        vec![message(ChatRole::User, "实现需求")]
    );

    let _delta_task = super::handle_agent_session_delta(
        &mut app,
        "task-1-plan".to_string(),
        "<think>规划中</think>开始拆分".to_string(),
    );

    assert_eq!(app.chat, vec![message(ChatRole::User, "主会话任务")]);
    let planning_chat = app
        .cached_chat_messages("task-1-plan")
        .expect("planning delta should stay in the child cache");
    assert_eq!(planning_chat.len(), 2);
    assert_eq!(planning_chat[1].role, ChatRole::Assistant);
    assert!(planning_chat[1].content.contains("开始拆分"));

    let _finish_task = super::handle_plan_run_session_finished(&mut app, "task-1-plan".to_string());
    assert!(!app.get_session_runtime("task-1-plan").is_requesting);
    assert!(
        app.get_session_runtime("main-session").active_agent_request.is_some(),
        "registering the child must not detach the main plan-run request"
    );
}

#[test]
fn plan_run_subtask_session_registers_streams_and_finishes_in_background() {
    let (mut app, _) = crate::app::App::new();
    let directory = "/workspace/demo";
    app.project_path = Some(directory.to_string());
    app.active_session_id = Some("main-session".to_string());
    app.sessions = vec![session_info("main-session", directory, None)];
    app.chat = vec![message(ChatRole::User, "主会话任务")];
    app.chat_message_ids = vec![None];
    let parent_runtime = app.get_session_runtime_mut("main-session");
    parent_runtime.is_requesting = true;
    parent_runtime.active_agent_request = Some(agent_request(42, "main-session"));

    let _register_task = super::handle_plan_run_session_started(
        &mut app,
        42,
        "main-session".to_string(),
        "task-1-sub-1".to_string(),
        "实现入口".to_string(),
        directory.to_string(),
        "实现入口".to_string(),
    );
    let _delta_task = super::handle_agent_session_delta(
        &mut app,
        "task-1-sub-1".to_string(),
        "正在实现".to_string(),
    );

    assert_eq!(app.active_session_id.as_deref(), Some("main-session"));
    assert_eq!(app.chat, vec![message(ChatRole::User, "主会话任务")]);
    let child = app
        .project_sessions
        .get(directory)
        .and_then(|sessions| sessions.iter().find(|session| session.id == "task-1-sub-1"))
        .expect("execution child should be registered");
    assert_eq!(child.title, "实现入口");
    assert_eq!(child.parent_id.as_deref(), Some("main-session"));
    assert!(app.get_session_runtime("task-1-sub-1").is_requesting);
    let child_chat =
        app.cached_chat_messages("task-1-sub-1").expect("execution child chat should be cached");
    assert_eq!(child_chat.len(), 2);
    assert_eq!(child_chat[1].content, "正在实现");

    let _finish_task =
        super::handle_plan_run_session_finished(&mut app, "task-1-sub-1".to_string());

    assert!(!app.get_session_runtime("task-1-sub-1").is_requesting);
    assert!(app.get_session_runtime("main-session").active_agent_request.is_some());
}

#[test]
fn update_think_timing_ignores_empty_delta() {
    let mut message = message(ChatRole::Assistant, "<think>open");
    message.think_timing.push(ThinkTiming { start_ms: 1, end_ms: None, last_update_ms: 1 });

    super::update_think_timing_from_delta(&mut message, "", 99);

    assert_eq!(message.think_timing[0].last_update_ms, 1);
    assert_eq!(message.think_timing[0].end_ms, None);
}

#[test]
fn update_think_timing_ignores_plain_delta_without_think_blocks() {
    let mut message = message(ChatRole::Assistant, "plain");

    super::update_think_timing_from_delta(&mut message, " text", 42);

    assert!(message.think_timing.is_empty());
}

#[test]
fn chat_scroll_semantics_stream_growth_keeps_manual_scroll_top() {
    let (mut app, _) = crate::app::App::new();
    let session_id = "session-1".to_string();
    app.active_session_id = Some(session_id.clone());
    app.chat = vec![
        message(ChatRole::User, "question"),
        message(ChatRole::Assistant, "answer"),
        message(ChatRole::User, "follow-up"),
        message(ChatRole::Assistant, "streaming"),
    ];
    app.chat_message_ids = vec![None; app.chat.len()];
    app.chat_auto_scroll = false;
    app.chat_virtualizer = ChatVirtualizer::from_estimated_heights(&[100.0, 120.0, 100.0, 180.0]);
    app.chat_virtualizer.set_viewport_height(220.0);
    app.chat_virtualizer.set_scroll_top_px(140.0);
    app.sync_chat_virtualizer_compat_fields();
    app.get_session_runtime_mut(&session_id).active_agent_request =
        Some(agent_request(42, &session_id));
    let before_scroll_top = app.chat_virtualizer.scroll_top_px();

    let _task = super::handle_agent_stream_delta(&mut app, 42, " delta".to_string());

    assert!(!app.chat_auto_scroll);
    assert_eq!(app.chat_virtualizer.scroll_mode(), ChatScrollMode::Anchored);
    assert_eq!(app.chat_virtualizer.scroll_top_px(), before_scroll_top);
    assert!(app.chat.last().unwrap().content.ends_with(" delta"));
}

#[test]
#[cfg(not(target_arch = "wasm32"))]
fn stream_delta_defers_session_save_off_update_path() {
    let (mut app, _) = crate::app::App::new();
    let session_id = "session-1".to_string();
    app.active_session_id = Some(session_id.clone());
    app.chat = vec![message(ChatRole::User, "question")];
    app.chat_message_ids = vec![None; app.chat.len()];
    app.get_session_runtime_mut(&session_id).active_agent_request =
        Some(agent_request(42, &session_id));
    let save_calls = Arc::new(AtomicUsize::new(0));
    let observed_save_calls = Arc::clone(&save_calls);

    let _task = super::super::with_save_session_hook(
        move |_, _| {
            observed_save_calls.fetch_add(1, Ordering::SeqCst);
            None
        },
        || super::handle_agent_stream_delta(&mut app, 42, " delta".to_string()),
    );

    assert_eq!(save_calls.load(Ordering::SeqCst), 0);
    assert_eq!(app.chat.last().unwrap().content, " delta");
}

#[test]
fn queued_stream_deltas_are_coalesced_until_flush() {
    let (mut app, _) = crate::app::App::new();
    let session_id = "session-1".to_string();
    app.active_session_id = Some(session_id.clone());
    app.chat = vec![message(ChatRole::User, "question")];
    app.chat_message_ids = vec![None; app.chat.len()];
    app.get_session_runtime_mut(&session_id).active_agent_request =
        Some(agent_request(42, &session_id));

    let first_task = super::queue_agent_stream_delta(&mut app, 42, "hello".to_string());
    let second_task = super::queue_agent_stream_delta(&mut app, 42, " world".to_string());

    assert!(first_task.units() > 0);
    assert_eq!(second_task.units(), 0);
    assert_eq!(app.chat.len(), 1);
    assert_eq!(app.chat.last().unwrap().content, "question");
    assert_eq!(app.get_session_runtime(&session_id).pending_stream_delta, "hello world");

    let _flush_task = super::flush_agent_stream_delta(&mut app, session_id.clone());

    assert_eq!(app.chat.last().unwrap().content, "hello world");
    assert!(app.get_session_runtime(&session_id).pending_stream_delta.is_empty());
    assert!(!app.get_session_runtime(&session_id).stream_delta_flush_scheduled);
}

#[test]
fn active_stream_delta_flush_interval_is_one_second() {
    let (mut app, _) = crate::app::App::new();
    let session_id = "active-session".to_string();
    app.active_session_id = Some(session_id.clone());

    assert_eq!(
        super::stream_delta_flush_interval(&app, &session_id),
        std::time::Duration::from_secs(1)
    );
}

#[test]
fn background_stream_delta_keeps_existing_interval() {
    let (mut app, _) = crate::app::App::new();
    app.active_session_id = Some("active-session".to_string());

    assert_eq!(
        super::stream_delta_flush_interval(&app, "background-session"),
        std::time::Duration::from_millis(100)
    );
}

#[test]
fn active_stream_delta_updates_tail_without_replacing_chat_vector() {
    let (mut app, _) = crate::app::App::new();
    let session_id = "session-1".to_string();
    app.active_session_id = Some(session_id.clone());
    app.chat = Vec::with_capacity(4);
    app.chat.push(message(ChatRole::User, "question"));
    app.chat_message_ids = vec![Some("user-id".to_string())];
    app.chat_auto_scroll = false;
    app.chat_virtualizer = ChatVirtualizer::from_estimated_heights(&[100.0]);
    app.chat_virtualizer.set_viewport_height(220.0);
    app.chat_virtualizer.set_scroll_top_px(0.0);
    app.sync_chat_virtualizer_compat_fields();
    app.get_session_runtime_mut(&session_id).active_agent_request =
        Some(agent_request(42, &session_id));

    let chat_ptr = app.chat.as_ptr();
    let chat_capacity = app.chat.capacity();

    let _task = super::handle_agent_stream_delta(&mut app, 42, "hello".to_string());

    assert_eq!(app.chat.as_ptr(), chat_ptr);
    assert_eq!(app.chat.capacity(), chat_capacity);
    assert_eq!(app.chat.len(), 2);
    assert_eq!(app.chat[0].content, "question");
    assert_eq!(app.chat[1].role, ChatRole::Assistant);
    assert_eq!(app.chat[1].content, "hello");
    assert_eq!(app.chat_message_ids, vec![Some("user-id".to_string()), None]);
    assert_eq!(app.chat_virtualizer.scroll_mode(), ChatScrollMode::Anchored);
    app.store_session_chat_snapshot(
        session_id.clone(),
        crate::app::session::shared_chat_messages(vec![
            message(ChatRole::User, "question"),
            message(ChatRole::Assistant, "stale"),
        ]),
        vec![Some("user-id".to_string()), None],
    );

    let _task = super::handle_agent_stream_delta(&mut app, 42, " world".to_string());

    assert_eq!(app.chat.as_ptr(), chat_ptr);
    assert_eq!(app.chat.capacity(), chat_capacity);
    assert_eq!(app.chat.len(), 2);
    assert_eq!(app.chat[1].content, "hello world");
    assert_eq!(app.chat_message_ids, vec![Some("user-id".to_string()), None]);
    assert_eq!(app.chat_virtualizer.scroll_mode(), ChatScrollMode::Anchored);
    assert_eq!(app.active_shared_chat_messages()[1].content, "hello world");
}

#[test]
fn tail_message_prewarm_does_not_mark_chunk_state() {
    let (mut app, _) = crate::app::App::new();
    let session_id = "session-1".to_string();
    app.active_session_id = Some(session_id.clone());
    app.chat = vec![message(ChatRole::User, "question")];
    app.chat_message_ids = vec![None; app.chat.len()];
    app.chat_auto_scroll = true;
    app.chat_virtualizer = ChatVirtualizer::from_estimated_heights(&[100.0]);
    app.chat_virtualizer.set_viewport_height(400.0);
    app.chat_virtualizer.set_scroll_top_px(0.0);
    app.sync_chat_virtualizer_compat_fields();
    app.get_session_runtime_mut(&session_id).active_agent_request =
        Some(agent_request(42, &session_id));

    let first_task = super::handle_agent_stream_delta(&mut app, 42, "hello".to_string());
    let second_task = super::handle_agent_stream_delta(&mut app, 42, " world".to_string());

    assert_eq!(first_task.units(), 2);
    assert_eq!(second_task.units(), 2);
    assert!(app.active_session_view_state.preparing_chat_ui_chunks.is_empty());
    assert!(app.active_session_view_state.dirty_preparing_chat_ui_chunks.is_empty());
    assert_eq!(app.chat.last().unwrap().content, "hello world");
}

#[test]
fn active_stream_growth_preserves_turn_top_focus() {
    let (mut app, _) = crate::app::App::new();
    let session_id = "session-1".to_string();
    app.active_session_id = Some(session_id.clone());
    app.chat = vec![
        message(ChatRole::Assistant, "old answer"),
        message(ChatRole::Assistant, "more history"),
        message(ChatRole::User, "new question"),
        message(ChatRole::Assistant, ""),
    ];
    app.chat_message_ids = vec![None; app.chat.len()];
    app.chat_auto_scroll = false;
    app.chat_virtualizer = ChatVirtualizer::from_estimated_heights(&[180.0, 180.0, 100.0, 0.0]);
    app.chat_virtualizer.set_viewport_height(300.0);
    app.chat_virtualizer.focus_turn_on_user(2);
    app.sync_chat_virtualizer_compat_fields();
    app.get_session_runtime_mut(&session_id).active_agent_request =
        Some(agent_request(42, &session_id));
    let anchored_top = app.chat_virtualizer.scroll_top_px();

    let _task = super::handle_agent_stream_delta(&mut app, 42, "growing answer".to_string());

    assert_eq!(app.chat_virtualizer.turn_focus_user_index(), Some(2));
    assert_eq!(app.chat_virtualizer.scroll_top_px(), anchored_top);
}

#[test]
fn update_think_timing_opens_and_closes_think_block() {
    let mut message = message(ChatRole::Assistant, "");

    super::update_think_timing_from_delta(&mut message, "<think>working", 10);
    message.content.push_str("<think>working");
    super::update_think_timing_from_delta(&mut message, "</think>done", 25);

    assert_eq!(message.think_timing.len(), 1);
    assert_eq!(message.think_timing[0].start_ms, 10);
    assert_eq!(message.think_timing[0].end_ms, Some(25));
    assert_eq!(message.think_timing[0].last_update_ms, 25);
}

#[test]
fn update_think_timing_closes_previous_block_and_keeps_latest_open() {
    let mut message = message(ChatRole::Assistant, "");

    super::update_think_timing_from_delta(&mut message, "<think>first</think><think>second", 40);

    assert_eq!(message.think_timing.len(), 2);
    assert_eq!(message.think_timing[0].end_ms, Some(40));
    assert_eq!(message.think_timing[0].last_update_ms, 40);
    assert_eq!(message.think_timing[1].end_ms, None);
    assert_eq!(message.think_timing[1].last_update_ms, 40);
}

#[test]
fn stream_done_message_ids_resize_and_assign_last_assistant_and_parent_user() {
    let chat = vec![
        message(ChatRole::User, "first"),
        message(ChatRole::Assistant, "old"),
        message(ChatRole::User, "latest"),
        message(ChatRole::Assistant, "answer"),
    ];
    let mut ids = vec![Some("first-id".to_string())];

    super::apply_stream_done_message_ids(
        &chat,
        &mut ids,
        Some("assistant-id".to_string()),
        Some("parent-id".to_string()),
    );

    assert_eq!(ids.len(), 4);
    assert_eq!(ids[0].as_deref(), Some("first-id"));
    assert_eq!(ids[2].as_deref(), Some("parent-id"));
    assert_eq!(ids[3].as_deref(), Some("assistant-id"));
}

#[test]
fn stream_done_message_ids_ignore_assistant_id_without_assistant_message() {
    let chat = vec![
        message(ChatRole::System, "context"),
        message(ChatRole::User, "question"),
        message(ChatRole::Tool, "result"),
    ];
    let mut ids = Vec::new();

    super::apply_stream_done_message_ids(
        &chat,
        &mut ids,
        Some("unused-assistant-id".to_string()),
        Some("parent-id".to_string()),
    );

    assert_eq!(ids, vec![None, Some("parent-id".to_string()), None]);
}

#[test]
fn message_ids_tail_returns_at_most_four_entries() {
    let ids = vec![Some("1".to_string()), None, Some("3".to_string()), None, Some("5".to_string())];

    assert_eq!(
        super::message_ids_tail(&ids),
        vec![None, Some("3".to_string()), None, Some("5".to_string())]
    );
}

#[test]
fn message_ids_tail_preserves_short_and_empty_inputs() {
    let ids = vec![Some("1".to_string()), None];

    assert_eq!(super::message_ids_tail(&ids), ids);
    assert!(super::message_ids_tail(&[]).is_empty());
}

#[test]
fn parse_tool_block_len_requires_tool_prefix_and_complete_json() {
    let block = "tool shell\n{\"tool_call_id\":\"call-1\",\"status\":\"ok\"}\ntrailing";

    assert_eq!(super::parse_tool_block_len(block), Some(block.find("trailing").unwrap()));
    assert_eq!(super::parse_tool_block_len("prefix tool shell\n{}"), None);
    assert_eq!(super::parse_tool_block_len("tool shell\n{"), None);
}

#[test]
fn parse_tool_block_len_accepts_multiline_json_without_trailing_newline() {
    let block = "tool shell\n{\n  \"tool_call_id\": \"call-1\",\n  \"status\": \"ok\"\n}";

    assert_eq!(super::parse_tool_block_len(block), Some(block.len()));
}

#[test]
fn tool_call_id_from_raw_block_rejects_missing_or_non_string_ids() {
    assert_eq!(
        super::tool_call_id_from_raw_block("tool shell\n{\"tool_call_id\":\"call-1\"}"),
        Some("call-1".to_string())
    );
    assert_eq!(super::tool_call_id_from_raw_block("tool shell"), None);
    assert_eq!(super::tool_call_id_from_raw_block("tool shell\nnot-json"), None);
    assert_eq!(super::tool_call_id_from_raw_block("tool shell\n{\"tool_call_id\":7}"), None);
}

#[test]
fn tool_block_start_allowed_accepts_line_start_and_colon_prefix_only() {
    assert!(super::tool_block_start_allowed("tool shell\n{}", 0));
    let ascii_colon = "工具:\ntool shell\n{}";
    let full_width_colon = "工具：\ntool shell\n{}";
    assert!(super::tool_block_start_allowed(ascii_colon, ascii_colon.find("tool").unwrap()));
    assert!(super::tool_block_start_allowed(
        full_width_colon,
        full_width_colon.find("tool").unwrap()
    ));
    assert!(!super::tool_block_start_allowed("inline tool shell\n{}", 7));
}

#[test]
fn tool_block_start_allowed_accepts_same_line_colon_after_whitespace() {
    let content = "工具： \ttool shell\n{}";
    let start = content.find("tool").unwrap();

    assert!(super::tool_block_start_allowed(content, start));
    assert!(!super::tool_block_start_allowed("label  tool shell\n{}", 7));
}

#[test]
fn workflow_tool_block_upsert_replaces_matching_call_id() {
    let running = r#"tool workflow_node
{"tool_call_id":"workflow-node-2-llm","status":"running","summary":"Run Task · llm"}
"#;
    let finished = r#"tool workflow_node
{"tool_call_id":"workflow-node-2-llm","status":"completed","summary":"Run Task · llm","output":"ok"}
"#;
    let mut content = String::new();

    super::upsert_tool_block_by_call_id(&mut content, running);
    super::upsert_tool_block_by_call_id(&mut content, finished);

    assert_eq!(content.matches("tool workflow_node").count(), 1);
    assert!(content.contains("\"status\":\"completed\""));
    assert!(!content.contains("\"status\":\"running\""));
}

#[test]
fn workflow_tool_block_upsert_appends_when_call_id_is_missing_or_different() {
    let mut content = "prefix".to_string();

    super::upsert_tool_block_by_call_id(&mut content, "tool shell\n{\"status\":\"ok\"}\n");
    super::upsert_tool_block_by_call_id(
        &mut content,
        "tool shell\n{\"tool_call_id\":\"call-2\",\"status\":\"ok\"}\n",
    );

    assert_eq!(content.matches("tool shell").count(), 2);
    assert!(content.ends_with('\n'));
}

#[test]
fn workflow_tool_block_upsert_ignores_inline_text_that_looks_like_a_tool_block() {
    let mut content =
        "documentation tool shell\n{\"tool_call_id\":\"call-1\",\"status\":\"old\"}\n".to_string();

    super::upsert_tool_block_by_call_id(
        &mut content,
        "tool shell\n{\"tool_call_id\":\"call-1\",\"status\":\"new\"}\n",
    );

    assert_eq!(content.matches("tool shell").count(), 2);
    assert!(content.contains("\"status\":\"old\""));
    assert!(content.contains("\"status\":\"new\""));
}

#[test]
fn append_tool_block_normalizes_separators_and_trailing_newlines() {
    let mut content = "prefix".to_string();

    super::append_tool_block(&mut content, "tool shell\n{}\n\n");

    assert_eq!(content, "prefix\ntool shell\n{}\n");
}

#[test]
fn append_stream_delta_to_tail_appends_or_creates_assistant_message() {
    let mut assistant_chat = vec![message(ChatRole::Assistant, "hello")];
    let mut assistant_ids = vec![Some("assistant-id".to_string())];

    let changed_idx = super::append_stream_delta_to_tail(
        &mut assistant_chat,
        &mut assistant_ids,
        " world".to_string(),
        10,
    );

    assert_eq!(changed_idx, 0);
    assert_eq!(assistant_chat[0].content, "hello world");
    assert_eq!(assistant_ids, vec![Some("assistant-id".to_string())]);

    let mut user_chat = vec![message(ChatRole::User, "question")];
    let mut user_ids = Vec::new();
    let changed_idx =
        super::append_stream_delta_to_tail(&mut user_chat, &mut user_ids, "answer".to_string(), 20);

    assert_eq!(changed_idx, 1);
    assert_eq!(user_chat[1].role, ChatRole::Assistant);
    assert_eq!(user_chat[1].content, "answer");
    assert!(user_chat[1].think_timing.is_empty());
    assert_eq!(user_ids, vec![None, None]);
}

#[test]
fn upsert_workflow_tool_to_tail_reuses_assistant_or_creates_one() {
    let running = "tool workflow_node\n{\"tool_call_id\":\"node-1\",\"status\":\"running\"}\n";
    let finished = "tool workflow_node\n{\"tool_call_id\":\"node-1\",\"status\":\"completed\"}\n";
    let mut assistant_chat = vec![message(ChatRole::Assistant, running)];
    let mut assistant_ids = vec![Some("assistant-id".to_string())];

    let changed_idx = super::upsert_workflow_tool_to_tail(
        &mut assistant_chat,
        &mut assistant_ids,
        finished.to_string(),
    );

    assert_eq!(changed_idx, 0);
    assert!(assistant_chat[0].content.contains("\"status\":\"completed\""));
    assert!(!assistant_chat[0].content.contains("\"status\":\"running\""));
    assert_eq!(assistant_ids, vec![Some("assistant-id".to_string())]);

    let mut user_chat = vec![message(ChatRole::User, "question")];
    let mut user_ids = Vec::new();
    let changed_idx =
        super::upsert_workflow_tool_to_tail(&mut user_chat, &mut user_ids, running.to_string());

    assert_eq!(changed_idx, 1);
    assert_eq!(user_chat[1].role, ChatRole::Assistant);
    assert!(user_chat[1].content.contains("\"status\":\"running\""));
    assert_eq!(user_ids, vec![None, None]);
}

#[test]
fn leading_guide_count_only_counts_initial_guide_items() {
    let guide = queue_item(ChatSendBehavior::Guide);
    let normal = queue_item(ChatSendBehavior::Queue);
    let late_guide = queue_item(ChatSendBehavior::Guide);

    assert_eq!(super::leading_guide_count(&[guide, normal, late_guide]), 1);
}

#[test]
fn leading_guide_count_handles_empty_and_all_guide_queues() {
    assert_eq!(super::leading_guide_count(&[]), 0);
    assert_eq!(
        super::leading_guide_count(&[
            queue_item(ChatSendBehavior::Guide),
            queue_item(ChatSendBehavior::Guide),
        ]),
        2
    );
}

#[test]
fn continuation_label_from_history_uses_last_user_message() {
    let history = vec![
        message(ChatRole::User, "first"),
        message(ChatRole::Assistant, "answer"),
        message(ChatRole::User, "second"),
    ];

    assert_eq!(super::continuation_label_from_history(&history), "second");
    assert_eq!(super::continuation_label_from_history(&[]), "");
}

#[test]
fn stream_handlers_reject_unknown_request_without_mutating_chat() {
    let (mut app, _) = crate::app::App::new();
    app.chat = vec![message(ChatRole::User, "kept")];
    app.chat_message_ids = vec![Some("kept-id".to_string())];

    let _delta = super::handle_agent_stream_delta(&mut app, 404, "ignored".to_string());
    let _workflow =
        super::handle_agent_workflow_node_update(&mut app, 404, "tool shell\n{}\n".to_string());
    let _done = super::handle_agent_stream_done(
        &mut app,
        404,
        crate::app::models::TokenUsage::default(),
        Some("assistant-id".to_string()),
        Some("parent-id".to_string()),
    );
    let _error = super::handle_agent_stream_error(&mut app, 404, "ignored".to_string());

    assert_eq!(app.chat.len(), 1);
    assert_eq!(app.chat[0].role, ChatRole::User);
    assert_eq!(app.chat[0].content, "kept");
    assert_eq!(app.chat_message_ids, vec![Some("kept-id".to_string())]);
}

#[test]
fn inactive_stream_updates_cached_snapshot_without_touching_active_chat() {
    let (mut app, _) = crate::app::App::new();
    let session_id = "background-session".to_string();
    app.active_session_id = Some("active-session".to_string());
    app.chat = vec![message(ChatRole::User, "active")];
    app.chat_message_ids = vec![Some("active-id".to_string())];
    app.store_session_chat_snapshot(
        session_id.clone(),
        crate::app::session::shared_chat_messages(vec![message(ChatRole::User, "background")]),
        vec![Some("background-id".to_string())],
    );
    app.get_session_runtime_mut(&session_id).active_agent_request =
        Some(agent_request(42, &session_id));

    let _delta = super::handle_agent_stream_delta(&mut app, 42, "answer".to_string());
    let tool_block = "tool workflow_node\n{\"tool_call_id\":\"node-1\",\"status\":\"running\"}\n";
    let _workflow = super::handle_agent_workflow_node_update(&mut app, 42, tool_block.to_string());

    let cached = app.cached_chat_messages(&session_id).unwrap();
    assert_eq!(cached.len(), 2);
    assert_eq!(cached[0].role, ChatRole::User);
    assert_eq!(cached[0].content, "background");
    assert_eq!(cached[1].role, ChatRole::Assistant);
    assert!(cached[1].content.starts_with("answer\n"));
    assert!(cached[1].content.contains("\"tool_call_id\":\"node-1\""));
    assert_eq!(
        app.cached_chat_message_ids(&session_id).unwrap(),
        vec![Some("background-id".to_string()), None]
    );
    assert_eq!(app.chat.len(), 1);
    assert_eq!(app.chat[0].role, ChatRole::User);
    assert_eq!(app.chat[0].content, "active");
    assert_eq!(app.chat_message_ids, vec![Some("active-id".to_string())]);
}

#[test]
fn active_workflow_update_appends_tool_block_and_preserves_message_id() {
    let (mut app, _) = crate::app::App::new();
    let session_id = "session-1".to_string();
    app.active_session_id = Some(session_id.clone());
    app.chat = vec![message(ChatRole::Assistant, "answer")];
    app.chat_message_ids = vec![Some("assistant-id".to_string())];
    app.get_session_runtime_mut(&session_id).active_agent_request =
        Some(agent_request(42, &session_id));

    let _task = super::handle_agent_workflow_node_update(
        &mut app,
        42,
        "tool workflow_node\n{\"tool_call_id\":\"node-1\",\"status\":\"running\"}\n".to_string(),
    );

    assert_eq!(app.chat.len(), 1);
    assert!(app.chat[0].content.starts_with("answer\ntool workflow_node\n"));
    assert_eq!(app.chat_message_ids, vec![Some("assistant-id".to_string())]);
}

#[test]
fn stream_done_clears_active_request_and_applies_ids_and_usage() {
    let (mut app, _) = crate::app::App::new();
    let session_id = "session-1".to_string();
    app.active_session_id = Some(session_id.clone());
    app.chat = vec![message(ChatRole::User, "question"), message(ChatRole::Assistant, "answer")];
    app.chat_message_ids = vec![None, None];
    app.show_send_mode_popover = true;
    let runtime = app.get_session_runtime_mut(&session_id);
    runtime.is_requesting = true;
    runtime.active_agent_request = Some(agent_request(42, &session_id));
    let usage = crate::app::models::TokenUsage {
        input_tokens: 11,
        output_tokens: 22,
        cached_tokens: 3,
        reasoning_tokens: 4,
    };

    let _task = super::handle_agent_stream_done(
        &mut app,
        42,
        usage.clone(),
        Some("assistant-id".to_string()),
        Some("parent-id".to_string()),
    );

    assert_eq!(app.chat_message_ids[0].as_deref(), Some("parent-id"));
    assert_eq!(app.chat_message_ids[1].as_deref(), Some("assistant-id"));
    assert_eq!(app.usage, usage);
    assert!(!app.show_send_mode_popover);
    let runtime = app.get_session_runtime(&session_id);
    assert!(!runtime.is_requesting);
    assert!(!runtime.has_unseen_success);
    assert!(runtime.active_agent_request.is_none());
}

#[test]
fn processing_timing_stream_done_closes_active_and_background_cycles() {
    let (mut app, _) = crate::app::App::new();
    let active_id = "active-session".to_string();
    app.active_session_id = Some(active_id.clone());
    let mut active_assistant = message(ChatRole::Assistant, "answer");
    active_assistant.processing_timing = Some(vw_shared::session::ui_types::ProcessingTiming {
        started_ms: 100,
        completed_ms: None,
        processing_elapsed_ms: None,
        llm_elapsed_ms: None,
    });
    app.chat = vec![active_assistant];
    app.chat_message_ids = vec![None];
    app.get_session_runtime_mut(&active_id).active_agent_request =
        Some(agent_request(41, &active_id));

    let _ = super::handle_agent_stream_done(
        &mut app,
        41,
        crate::app::models::TokenUsage::default(),
        None,
        None,
    );
    assert!(app.chat[0].processing_timing.as_ref().unwrap().completed_ms.is_some());

    let background_id = "background-session".to_string();
    let mut background_assistant = message(ChatRole::Assistant, "background answer");
    background_assistant.processing_timing = Some(vw_shared::session::ui_types::ProcessingTiming {
        started_ms: 200,
        completed_ms: None,
        processing_elapsed_ms: None,
        llm_elapsed_ms: None,
    });
    app.store_session_chat_snapshot(
        background_id.clone(),
        crate::app::session::shared_chat_messages(vec![background_assistant]),
        vec![None],
    );
    app.get_session_runtime_mut(&background_id).active_agent_request =
        Some(agent_request(42, &background_id));

    let _ = super::handle_agent_stream_done(
        &mut app,
        42,
        crate::app::models::TokenUsage::default(),
        None,
        None,
    );
    assert!(
        app.cached_chat_messages(&background_id).unwrap()[0]
            .processing_timing
            .as_ref()
            .unwrap()
            .completed_ms
            .is_some()
    );
}

#[test]
fn inactive_stream_done_updates_cached_ids_and_marks_unseen_success() {
    let (mut app, _) = crate::app::App::new();
    let session_id = "background-session".to_string();
    app.active_session_id = Some("active-session".to_string());
    app.chat = vec![message(ChatRole::User, "active")];
    app.chat_message_ids = vec![Some("active-id".to_string())];
    app.store_session_chat_snapshot(
        session_id.clone(),
        crate::app::session::shared_chat_messages(vec![
            message(ChatRole::User, "background question"),
            message(ChatRole::Assistant, "background answer"),
        ]),
        vec![None, None],
    );
    let runtime = app.get_session_runtime_mut(&session_id);
    runtime.is_requesting = true;
    runtime.active_agent_request = Some(agent_request(42, &session_id));

    let _task = super::handle_agent_stream_done(
        &mut app,
        42,
        crate::app::models::TokenUsage::default(),
        Some("assistant-id".to_string()),
        Some("parent-id".to_string()),
    );

    assert_eq!(app.chat, vec![message(ChatRole::User, "active")]);
    assert_eq!(app.chat_message_ids, vec![Some("active-id".to_string())]);
    assert_eq!(
        app.cached_chat_message_ids(&session_id).unwrap(),
        vec![Some("parent-id".to_string()), Some("assistant-id".to_string())]
    );
    let runtime = app.get_session_runtime(&session_id);
    assert!(!runtime.is_requesting);
    assert!(runtime.has_unseen_success);
    assert!(runtime.active_agent_request.is_none());
}

#[test]
fn active_stream_error_appends_system_message_and_clears_request_state() {
    let (mut app, _) = crate::app::App::new();
    let session_id = "session-1".to_string();
    app.active_session_id = Some(session_id.clone());
    app.chat = vec![message(ChatRole::User, "question")];
    app.chat_message_ids = vec![Some("user-id".to_string())];
    app.show_send_mode_popover = true;
    let runtime = app.get_session_runtime_mut(&session_id);
    runtime.is_requesting = true;
    runtime.active_agent_request = Some(agent_request(42, &session_id));

    let _task = super::handle_agent_stream_error(&mut app, 42, "gateway failed".to_string());

    assert_eq!(app.chat.last().map(|entry| entry.role), Some(ChatRole::System));
    assert_eq!(app.chat.last().map(|entry| entry.content.as_str()), Some("gateway failed"));
    assert_eq!(app.chat_message_ids, vec![Some("user-id".to_string()), None]);
    assert!(!app.show_send_mode_popover);
    assert!(app.active_toast.is_some());
    let runtime = app.get_session_runtime(&session_id);
    assert!(!runtime.is_requesting);
    assert!(runtime.active_agent_request.is_none());
}

#[test]
fn processing_timing_stream_error_closes_and_saves_background_cycle() {
    let (mut app, _) = crate::app::App::new();
    let active_id = "active-session".to_string();
    app.active_session_id = Some(active_id.clone());
    let mut active_assistant = message(ChatRole::Assistant, "active partial");
    active_assistant.processing_timing = Some(vw_shared::session::ui_types::ProcessingTiming {
        started_ms: 250,
        completed_ms: None,
        processing_elapsed_ms: None,
        llm_elapsed_ms: None,
    });
    app.chat = vec![active_assistant];
    app.chat_message_ids = vec![None];
    app.get_session_runtime_mut(&active_id).active_agent_request =
        Some(agent_request(42, &active_id));

    let _ = super::handle_agent_stream_error(&mut app, 42, "active failed".to_string());

    assert!(app.chat[0].processing_timing.as_ref().unwrap().completed_ms.is_some());
    assert_eq!(app.chat[1].role, ChatRole::System);

    let session_id = "background-session".to_string();
    let mut assistant = message(ChatRole::Assistant, "partial");
    assistant.processing_timing = Some(vw_shared::session::ui_types::ProcessingTiming {
        started_ms: 300,
        completed_ms: None,
        processing_elapsed_ms: None,
        llm_elapsed_ms: None,
    });
    app.store_session_chat_snapshot(
        session_id.clone(),
        crate::app::session::shared_chat_messages(vec![assistant]),
        vec![None],
    );
    app.get_session_runtime_mut(&session_id).active_agent_request =
        Some(agent_request(43, &session_id));

    let _ = super::handle_agent_stream_error(&mut app, 43, "failed".to_string());

    let cached = app.cached_chat_messages(&session_id).expect("background cache should be saved");
    assert_eq!(cached.len(), 2);
    assert!(cached[0].processing_timing.as_ref().unwrap().completed_ms.is_some());
    assert_eq!(cached[1].role, ChatRole::System);
    assert_eq!(cached[1].content, "failed");
}

#[test]
fn processing_timing_post_tool_round_closes_old_cycle_and_starts_new_cycle() {
    let (mut app, _) = crate::app::App::new();
    let session_id = "session-1".to_string();
    app.active_session_id = Some(session_id.clone());
    app.project_path = Some("/repo".to_string());
    let mut assistant = message(ChatRole::Assistant, "tool round");
    assistant.processing_timing = Some(vw_shared::session::ui_types::ProcessingTiming {
        started_ms: 400,
        completed_ms: None,
        processing_elapsed_ms: None,
        llm_elapsed_ms: None,
    });
    app.chat = vec![message(ChatRole::User, "question"), assistant];
    app.chat_message_ids = vec![None, None];
    let runtime = app.get_session_runtime_mut(&session_id);
    runtime.is_requesting = true;
    runtime.active_agent_request = Some(agent_request(44, &session_id));

    let _ = super::handle_agent_post_tool_round(&mut app, 44, session_id.clone(), 1);

    let timings = app
        .chat
        .iter()
        .filter_map(|message| message.processing_timing.as_ref())
        .collect::<Vec<_>>();
    assert_eq!(timings.len(), 2);
    assert_eq!(timings[0].started_ms, 400);
    assert!(timings[0].completed_ms.is_some());
    assert!(timings[1].completed_ms.is_none());
    assert_eq!(app.get_session_runtime(&session_id).request_started_ms, timings[1].started_ms);
}

#[test]
fn active_stream_error_humanizes_payload_too_large_and_shows_toast() {
    let (mut app, _) = crate::app::App::new();
    let session_id = "session-1".to_string();
    app.active_session_id = Some(session_id.clone());
    app.chat = vec![message(ChatRole::User, "question")];
    app.chat_message_ids = vec![Some("user-id".to_string())];
    let runtime = app.get_session_runtime_mut(&session_id);
    runtime.is_requesting = true;
    runtime.active_agent_request = Some(agent_request(42, &session_id));

    let _task = super::handle_agent_stream_error(
        &mut app,
        42,
        "gateway stream failed: 413 Payload Too Large Failed to buffer the request body: length limit exceeded"
            .to_string(),
    );

    let expected =
        "请求内容过大（图片或会话历史超出网关限制），请压缩图片、减少附件数量或开启新会话后重试";
    assert_eq!(app.chat.last().map(|entry| entry.content.as_str()), Some(expected));
    let toast = app.active_toast.as_ref().expect("toast should show");
    assert_eq!(toast.message, expected);
    assert_eq!(toast.kind, crate::app::state::ToastKind::Error);
    let runtime = app.get_session_runtime(&session_id);
    assert!(!runtime.is_requesting);
    assert!(runtime.active_agent_request.is_none());
}

#[test]
fn missing_request_stream_error_clears_active_requesting_and_shows_toast() {
    let (mut app, _) = crate::app::App::new();
    let session_id = "session-1".to_string();
    app.active_session_id = Some(session_id.clone());
    let runtime = app.get_session_runtime_mut(&session_id);
    runtime.is_requesting = true;
    runtime.active_agent_request = None;

    let _task = super::handle_agent_stream_error(
        &mut app,
        99,
        "gateway stream failed: 413 Payload Too Large".to_string(),
    );

    let expected =
        "请求内容过大（图片或会话历史超出网关限制），请压缩图片、减少附件数量或开启新会话后重试";
    let toast = app.active_toast.as_ref().expect("toast should show");
    assert_eq!(toast.message, expected);
    let runtime = app.get_session_runtime(&session_id);
    assert!(!runtime.is_requesting);
    assert!(app.chat.is_empty());
}

#[test]
fn post_tool_round_rejects_mismatched_or_inactive_session() {
    let (mut app, _) = crate::app::App::new();
    app.active_session_id = Some("active-session".to_string());
    app.get_session_runtime_mut("request-session").active_agent_request =
        Some(agent_request(42, "request-session"));

    let _mismatched =
        super::handle_agent_post_tool_round(&mut app, 42, "other-session".to_string(), 1);
    let _inactive =
        super::handle_agent_post_tool_round(&mut app, 42, "request-session".to_string(), 1);

    assert!(app.get_session_runtime("request-session").active_agent_request.is_some());
    assert!(app.get_session_runtime("request-session").queue.is_empty());
}

#[test]
fn context_loaded_adds_visible_agents_read_card() {
    let (mut app, _) = crate::app::App::new();
    let session_id = "session-1".to_string();
    app.active_session_id = Some(session_id.clone());
    app.chat = vec![message(ChatRole::User, "question"), message(ChatRole::Assistant, "")];
    app.chat_message_ids = vec![Some("user-id".to_string()), None];
    app.get_session_runtime_mut(&session_id).active_agent_request =
        Some(agent_request(42, &session_id));

    let _task = super::handle_agent_context_loaded(
        &mut app,
        42,
        vec![
            "/workspace/AGENTS.md".to_string(),
            "/workspace/AGENTS.md".to_string(),
            "/agent-workspace/SOUL.md".to_string(),
        ],
    );

    let content = &app.chat.last().expect("assistant message").content;
    assert!(content.starts_with("tool read\n"));
    assert!(content.contains(r#"\"filePath\":\"/workspace/AGENTS.md\""#));
    assert!(content.contains(r#"\"filePath\":\"/agent-workspace/SOUL.md\""#));
    assert!(content.contains(r#""status":"completed""#));
    assert_eq!(content.matches("tool read\n").count(), 2);
}
