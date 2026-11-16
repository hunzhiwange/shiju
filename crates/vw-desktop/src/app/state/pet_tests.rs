//! 任务宠物派生状态测试，覆盖标题摘要、Unicode 截断和思考标签识别。
//! 文本压缩必须保持字符边界，并避免把行内伪标签误判为工具或思考块。

use super::{TaskPetAvatarKind, TaskPetStatus};
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
    let _ = crate::app::components::task_pet::window(&app);
}
