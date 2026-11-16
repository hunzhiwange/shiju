//! 任务宠物派生状态测试，覆盖标题摘要、Unicode 截断和思考标签识别。
//! 文本压缩必须保持字符边界，并避免把行内伪标签误判为工具或思考块。

use super::{TaskPetAvatarKind, TaskPetItem, TaskPetStatus, newest_open_thinking_text};
use crate::app::App;
use crate::app::models::{ChatMessage, ChatRole};
use crate::app::state::{AcpHistoryReplayMode, AgentRequest, SessionRuntimeState};

fn app() -> App {
    App::new().0
}

fn chat_message(role: ChatRole, content: &str) -> ChatMessage {
    ChatMessage {
        role,
        content: content.to_string(),
        model: None,
        think_timing: Vec::new(),
        processing_timing: None,
    }
}

fn agent_request(
    id: u64,
    session: &str,
    query: &str,
    root: Option<&str>,
) -> std::sync::Arc<AgentRequest> {
    std::sync::Arc::new(AgentRequest {
        id,
        session: session.to_string(),
        query: query.to_string(),
        root: root.map(str::to_string),
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
        workflow_mode_enabled: false,
        goal_mode_enabled: false,
        history: Vec::new(),
    })
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
        time: session_info::TimeInfo { created: 10, updated: 20, compacting: None, archived: None },
        permission: None,
        fork_execution_started: None,
        revert: None,
    }
}

#[test]
fn newest_open_thinking_text_ignores_visible_answer() {
    let messages = vec![ChatMessage {
        role: ChatRole::Assistant,
        content: "<think>分析实现路径".to_string(),
        model: None,
        think_timing: Vec::new(),
        processing_timing: None,
    }];

    let detail = newest_open_thinking_text(
        messages
            .iter()
            .map(|message| (message.role == ChatRole::Assistant, message.content.as_str())),
    );

    assert_eq!(detail, Some("分析实现路径".to_string()));
}

#[test]
fn newest_open_thinking_text_returns_none_without_open_think() {
    let messages = vec![ChatMessage {
        role: ChatRole::Assistant,
        content: "<think>分析完成</think>\n最终回答".to_string(),
        model: None,
        think_timing: Vec::new(),
        processing_timing: None,
    }];

    let detail = newest_open_thinking_text(
        messages
            .iter()
            .map(|message| (message.role == ChatRole::Assistant, message.content.as_str())),
    );

    assert_eq!(detail, None);
}

#[test]
fn newest_open_thinking_text_skips_user_and_uses_newest_open_assistant() {
    let messages = vec![
        ChatMessage {
            role: ChatRole::Assistant,
            content: "<think>older analysis".to_string(),
            model: None,
            think_timing: Vec::new(),
            processing_timing: None,
        },
        ChatMessage {
            role: ChatRole::User,
            content: "<think>user text".to_string(),
            model: None,
            think_timing: Vec::new(),
            processing_timing: None,
        },
        ChatMessage {
            role: ChatRole::Assistant,
            content: "<think>newest analysis".to_string(),
            model: None,
            think_timing: Vec::new(),
            processing_timing: None,
        },
    ];

    assert_eq!(
        newest_open_thinking_text(
            messages
                .iter()
                .map(|message| (message.role == ChatRole::Assistant, message.content.as_str()))
        )
        .as_deref(),
        Some("newest analysis")
    );
}

#[test]
fn sync_task_pet_from_runtime_adds_updates_completes_and_sorts_items() {
    let mut app = app();
    app.session_state.session_runtime_states.clear();
    app.session_state.sessions.push(session_info("s1", "  Session Title  ", "/tmp/project-one"));
    app.session_state.active_session_id = Some("s1".to_string());
    app.chat_view.chat =
        vec![chat_message(ChatRole::Assistant, "<think>正在检查文件\ntool read\nresult")];

    let mut runtime = SessionRuntimeState::default();
    runtime.request_lifecycle.is_requesting = true;
    runtime.request_lifecycle.active_agent_request =
        Some(agent_request(7, "s1", "fallback title", Some("/tmp/project-one")));
    app.session_state.session_runtime_states.insert("s1".to_string(), runtime);

    app.sync_task_pet_from_runtime();

    assert_eq!(app.task_pet_active_count(), 1);
    assert_eq!(app.pet_state.task_pet_items.len(), 1);
    assert_eq!(app.pet_state.task_pet_items[0].request_id, 7);
    assert_eq!(app.pet_state.task_pet_items[0].title, "Session Title");
    assert_eq!(app.pet_state.task_pet_items[0].detail, "正在检查文件");
    assert_eq!(app.pet_state.task_pet_items[0].project.as_deref(), Some("project-one"));

    let runtime =
        app.session_state.session_runtime_states.get_mut("s1").expect("runtime should exist");
    runtime.request_lifecycle.active_agent_request =
        Some(agent_request(8, "s1", "replacement query", Some("/tmp/project-two")));
    app.chat_view.chat.clear();

    app.sync_task_pet_from_runtime();

    assert_eq!(app.pet_state.task_pet_items.len(), 1);
    assert_eq!(app.pet_state.task_pet_items[0].request_id, 8);
    assert_eq!(app.pet_state.task_pet_items[0].detail, "正在检查文件");
    assert_eq!(app.pet_state.task_pet_items[0].project.as_deref(), Some("project-two"));

    app.session_state
        .session_runtime_states
        .get_mut("s1")
        .expect("runtime should exist")
        .request_lifecycle
        .is_requesting = false;
    app.sync_task_pet_from_runtime();

    assert_eq!(app.task_pet_active_count(), 0);
    assert_eq!(app.pet_state.task_pet_items[0].status, TaskPetStatus::Completed);

    let mut other = SessionRuntimeState::default();
    other.request_lifecycle.is_requesting = true;
    other.request_lifecycle.active_agent_request =
        Some(agent_request(10, "s2", "newer task", None));
    app.session_state.session_runtime_states.insert("s2".to_string(), other);
    app.sync_task_pet_from_runtime();

    assert_eq!(app.pet_state.task_pet_items[0].request_id, 10);
    assert_eq!(app.pet_state.task_pet_items[0].status, TaskPetStatus::Running);
    assert_eq!(app.pet_state.task_pet_items[1].request_id, 8);
}

#[test]
fn mini_companion_none_keeps_controls() {
    let mut app = app();
    app.pet_state.task_pet_avatar_kind = TaskPetAvatarKind::None;
    app.companion_navigation.task_pet_visible = true;
    assert!(app.companion_navigation.task_pet_visible);
    assert_eq!(app.task_pet_window_size(), iced::Size::new(160.0, 168.0));
    let _ = crate::app::components::task_pet::window(&app);
    app.toggle_task_pet_collapsed();
    assert!(!app.pet_state.task_pet_collapsed);
    assert_eq!(app.pet_state.task_pet_avatar_kind, TaskPetAvatarKind::None);
}

#[test]
fn mini_companion_scale_rejects_non_finite_and_bounds_values() {
    let mut app = app();
    assert_eq!(app.pet_state.task_pet_scale, 1.0);
    for invalid in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
        assert!(app.set_task_pet_scale(invalid).is_err());
        assert_eq!(app.pet_state.task_pet_scale, 1.0);
    }
    app.set_task_pet_scale(0.1).unwrap();
    assert_eq!(app.pet_state.task_pet_scale, 0.5);
    app.set_task_pet_scale(8.0).unwrap();
    assert_eq!(app.pet_state.task_pet_scale, 2.0);
}

fn mini_task(id: u64, status: TaskPetStatus) -> TaskPetItem {
    TaskPetItem {
        request_id: id,
        session_id: format!("session-{id}"),
        title: format!("任务 {id}"),
        detail: "正在整理上下文".into(),
        detail_feedback: None,
        project: None,
        status,
        last_click_at: None,
    }
}

#[test]
fn mini_companion_stack_prioritizes_running_tasks() {
    let mut app = app();
    app.pet_state.task_pet_items = vec![mini_task(99, TaskPetStatus::Completed)];
    for id in [1, 3, 2] {
        let mut runtime = SessionRuntimeState::default();
        runtime.request_lifecycle.is_requesting = true;
        runtime.request_lifecycle.active_agent_request =
            Some(agent_request(id, &format!("session-{id}"), "任务", None));
        app.session_state.session_runtime_states.insert(format!("session-{id}"), runtime);
    }
    app.sync_task_pet_from_runtime();
    assert_eq!(
        app.pet_state.task_pet_items.iter().map(|i| i.request_id).collect::<Vec<_>>(),
        [3, 2, 1, 99]
    );
    app.session_state
        .session_runtime_states
        .get_mut("session-3")
        .unwrap()
        .request_lifecycle
        .is_requesting = false;
    app.sync_task_pet_from_runtime();
    assert_eq!(app.pet_state.task_pet_items[0].request_id, 2);
    assert_eq!(app.pet_state.task_pet_items[2].status, TaskPetStatus::Completed);
}

#[test]
fn mini_companion_dismiss_updates_top_card() {
    let mut app = app();
    app.pet_state.task_pet_items =
        vec![mini_task(3, TaskPetStatus::Running), mini_task(2, TaskPetStatus::Completed)];
    app.open_task_pet_reply(3);
    app.update_task_pet_reply_input("草稿".into());
    app.dismiss_task_pet_item(3);
    assert_eq!(app.pet_state.task_pet_items[0].request_id, 2);
    assert!(app.pet_state.task_pet_reply_request_id.is_none());
    assert!(app.pet_state.task_pet_reply_input.is_empty());
    app.sync_task_pet_from_runtime();
    assert_eq!(app.pet_state.task_pet_items[0].request_id, 2);
}

#[test]
fn mini_companion_scaled_bounds_include_controls() {
    let mut app = app();
    for scale in [0.5, 1.0, 2.0] {
        app.pet_state.task_pet_scale = scale;
        let bounds = app.task_pet_window_size();
        assert!(bounds.width >= 160.0);
        assert!(bounds.height >= 104.0 * scale.max(1.0) + 6.0 + 36.0 + 16.0);
    }
}

#[test]
fn mini_companion_size_preview_preserves_bottom_edge_and_card_dimensions() {
    let mut app = app();
    app.task_pet_window_id = Some(iced::window::Id::unique());
    app.pet_state.task_pet_position = iced::Point::new(500.0, 500.0);
    app.pet_state.task_pet_items = vec![mini_task(1, TaskPetStatus::Running)];
    for progress in [0.0, 0.5, 1.0] {
        app.pet_state.task_pet_expand_progress = progress;
        app.pet_state.task_pet_scale = 1.0;
        let original = app.task_pet_window_size();
        let bottom = app.pet_state.task_pet_position.y + original.height;
        app.pet_state.task_pet_size_draft = Some(2.0);
        let _ = app.resize_task_pet_preview(original);
        let enlarged = app.task_pet_window_size();
        assert_eq!(enlarged.height - original.height, 104.0);
        assert_eq!(enlarged.width, original.width);
        assert_eq!(app.pet_state.task_pet_position.y + enlarged.height, bottom);
        app.pet_state.task_pet_size_draft = None;
    }
}
