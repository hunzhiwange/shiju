use super::agent_sessions::{
    agent_session_open_message, agent_session_status_label, agent_session_stop_message,
    agent_sessions_badge_label, agent_sessions_badge_style, agent_sessions_entry,
    agent_sessions_popover, agent_sessions_popover_style, agent_sessions_row_style,
    agent_sessions_stop_style, running_agent_sessions,
};
use crate::app::models::{ChatMessage as ChatEntry, ChatRole};
use crate::app::session::{AgentChildSession, AgentChildSessionKind, AgentChildSessionStatus};
use crate::app::state::{AcpHistoryReplayMode, AgentRequest};
use crate::app::{Message, message};
use iced::Theme;
use vw_shared::session::info::{Info, TimeInfo};

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

fn session(id: &str, parent_id: Option<&str>, title: &str, directory: &str) -> Info {
    Info {
        id: id.to_string(),
        slug: id.to_string(),
        project_id: "project".to_string(),
        directory: directory.to_string(),
        parent_id: parent_id.map(str::to_string),
        summary: None,
        share: None,
        title: title.to_string(),
        version: "1".to_string(),
        time: TimeInfo { created: 1, updated: 1, compacting: None, archived: None },
        permission: None,
        revert: None,
    }
}

fn agent_request(id: u64, session_id: &str) -> std::sync::Arc<AgentRequest> {
    std::sync::Arc::new(AgentRequest {
        id,
        session: session_id.to_string(),
        query: "delegate".to_string(),
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
    })
}

fn agent_tool_event(
    status: &str,
    parent_session_id: &str,
    child_session_id: &str,
    agent: &str,
    task: &str,
) -> String {
    let payload = serde_json::json!({
        "status": status,
        "input": serde_json::to_string(&serde_json::json!({
            "action": "launch",
            "agent": agent,
            "prompt": task,
        }))
        .expect("serialize AgentTool input"),
        "title": "AgentTool",
        "metadata": {
            "parent_session_id": parent_session_id,
            "child_session_id": child_session_id,
            "agent": agent,
            "task": task,
        },
        "output": "",
    });
    format!("tool AgentTool\n{payload}\n")
}

fn assistant(content: &str) -> ChatEntry {
    ChatEntry {
        role: ChatRole::Assistant,
        content: content.to_string(),
        model: None,
        think_timing: Vec::new(),
        processing_timing: None,
    }
}

fn child(session_id: &str, agent: &str, title: &str, task: &str) -> AgentChildSession {
    AgentChildSession {
        session_id: session_id.to_string(),
        agent: agent.to_string(),
        task: task.to_string(),
        title: title.to_string(),
        directory: "/workspace/project".to_string(),
        kind: AgentChildSessionKind::Agent,
        status: AgentChildSessionStatus::Running,
        depth: 0,
        plan_run_task_id: None,
    }
}

#[test]
fn agent_sessions_popover_hides_without_running_children_and_counts_visible_items() {
    let app = crate::app::App::new().0;

    assert!(agent_sessions_entry(&app, &[]).is_none());
    assert_eq!(agent_sessions_badge_label(1), "1 个工作中");
    assert_eq!(agent_sessions_badge_label(3), "3 个工作中");
}

#[test]
fn agent_sessions_popover_lists_running_children_and_opens_selected_session() {
    let mut app = crate::app::App::new().0;
    app.show_agent_sessions_popover = true;
    let children = vec![
        child("child-writer", "writer", "实现输入面板工作中徽标", "增加入口并连接真实会话"),
        child(
            "child-reviewer",
            "reviewer",
            "审查超长会话标题是否会在固定宽度弹层中稳定截断",
            "验证浅色与暗色主题",
        ),
    ];

    let entry = agent_sessions_entry(&app, &children).expect("running children should show entry");
    assert_eq!(entry.as_widget().children().len(), 2);

    let popover = agent_sessions_popover(&app, &children);
    assert!(!popover.as_widget().children().is_empty());

    match agent_session_open_message(&children[1]) {
        Some(Message::Chat(message::ChatMessage::OpenAgentSession {
            project_path,
            session_id,
        })) => {
            assert_eq!(project_path, "/workspace/project");
            assert_eq!(session_id, "child-reviewer");
        }
        other => panic!("expected child session open message, got {other:?}"),
    }

    let mut invalid = children[0].clone();
    invalid.session_id = " ".to_string();
    assert!(agent_session_open_message(&invalid).is_none());
    invalid.session_id = "child".to_string();
    invalid.directory.clear();
    assert!(agent_session_open_message(&invalid).is_none());
}

#[test]
fn agent_sessions_popover_styles_support_light_and_dark_themes() {
    for theme in [Theme::Light, Theme::Dark] {
        let badge = agent_sessions_badge_style(&theme, iced::widget::button::Status::Hovered);
        let popover = agent_sessions_popover_style(&theme);
        let row = agent_sessions_row_style(&theme, iced::widget::button::Status::Active);
        let stop = agent_sessions_stop_style(&theme, iced::widget::button::Status::Hovered);

        assert!(badge.background.is_some());
        assert!(badge.text_color.a > 0.0);
        assert!(popover.background.is_some());
        assert!(popover.text_color.is_some());
        assert!(row.background.is_some());
        assert!(row.text_color.a > 0.0);
        assert!(stop.background.is_some());
        assert!(stop.text_color.a > 0.0);
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[tokio::test(flavor = "multi_thread")]
async fn agent_sessions_popover_layouts_nested_non_ascii_text_in_light_and_dark_themes() {
    use iced::advanced::renderer::Headless;
    use iced::advanced::{layout, widget::Tree};
    use iced::{Font, Pixels, Size};

    let renderer =
        <iced::Renderer as Headless>::new(Font::DEFAULT, Pixels(16.0), Some("tiny-skia"))
            .await
            .expect("tiny-skia headless renderer should be available");
    let app = crate::app::App::new().0;
    let mut workflow = child("workflow-child", "子智能体", "递归工作流", "等待孙会话");
    workflow.kind = AgentChildSessionKind::WorkflowExecution;
    workflow.status = AgentChildSessionStatus::Waiting;
    workflow.plan_run_task_id = Some("task-render".to_string());
    let mut nested = child("agent-grandchild", "reviewer", "检查中文动态文本", "等待递归后代完成");
    nested.status = AgentChildSessionStatus::Waiting;
    nested.depth = 1;
    let children = vec![workflow, nested];

    for theme in [Theme::Light, Theme::Dark] {
        let _ = agent_sessions_popover_style(&theme);
        let _ = agent_sessions_stop_style(&theme, iced::widget::button::Status::Active);
        let mut element = agent_sessions_popover(&app, &children);
        let mut tree = Tree::new(element.as_widget());
        let limits = layout::Limits::new(Size::ZERO, Size::new(400.0, 320.0));
        let node = element.as_widget_mut().layout(&mut tree, &renderer, &limits);
        assert!(node.bounds().width > 0.0);
        assert!(node.bounds().height > 0.0);
    }
}

#[test]
fn workflow_plan_run_children_reuse_input_session_list() {
    const DIRECTORY: &str = "/workspace/project";
    const PARENT_ID: &str = "parent-session";
    const RUNNING_CHILD_ID: &str = "workflow-child-running";
    const FINISHED_CHILD_ID: &str = "workflow-child-finished";

    let mut running_child =
        session(RUNNING_CHILD_ID, Some(PARENT_ID), "实现输入面板复用", DIRECTORY);
    running_child.slug = Info::plan_run_slug(RUNNING_CHILD_ID);
    let mut finished_child =
        session(FINISHED_CHILD_ID, Some(PARENT_ID), "已完成的工作流子任务", DIRECTORY);
    finished_child.slug = Info::plan_run_slug(FINISHED_CHILD_ID);

    let (mut app, _) = crate::app::App::new();
    app.active_session_id = Some(PARENT_ID.to_string());
    app.sessions = vec![
        session(PARENT_ID, None, "父会话", DIRECTORY),
        running_child,
        finished_child,
        session("ordinary-child", Some(PARENT_ID), "普通子会话", DIRECTORY),
    ];
    app.get_session_runtime_mut(RUNNING_CHILD_ID).is_requesting = true;
    app.get_session_runtime_mut("ordinary-child").is_requesting = true;

    let children = running_agent_sessions(&app);
    assert_eq!(children.len(), 1);
    assert_eq!(children[0].session_id, RUNNING_CHILD_ID);
    assert_eq!(children[0].agent, "子智能体");
    assert_eq!(children[0].task, "工作流执行");
    assert_eq!(children[0].title, "实现输入面板复用");
    assert_eq!(children[0].kind, AgentChildSessionKind::WorkflowExecution);
    assert_eq!(children[0].directory, DIRECTORY);
    assert!(agent_sessions_entry(&app, &children).is_some());

    app.get_session_runtime_mut(RUNNING_CHILD_ID).is_requesting = false;
    assert!(running_agent_sessions(&app).is_empty());
}

#[test]
fn workflow_planning_child_uses_planning_copy_and_keeps_directory_for_tooltip() {
    const TASK_ID: &str = "019fb6ed-fc1c-7c21-8d82-c13454526594";
    const SESSION_ID: &str = "019fb6ed-fc1c-7c21-8d82-c13454526594-plan";
    const DIRECTORY: &str = "/workspace/project";
    const PARENT_ID: &str = "parent-session";

    let mut planning_child =
        session(SESSION_ID, Some(PARENT_ID), &format!("规划 · {TASK_ID}"), DIRECTORY);
    planning_child.slug = Info::plan_run_slug(SESSION_ID);

    let (mut app, _) = crate::app::App::new();
    app.active_session_id = Some(PARENT_ID.to_string());
    app.sessions = vec![session(PARENT_ID, None, "父会话", DIRECTORY), planning_child];
    app.get_session_runtime_mut(SESSION_ID).is_requesting = true;

    let children = running_agent_sessions(&app);
    assert_eq!(children.len(), 1);
    assert_eq!(children[0].title, "规划");
    assert_eq!(children[0].task, format!("计划 {TASK_ID}"));
    assert_eq!(children[0].directory, DIRECTORY);
    assert_eq!(children[0].kind, AgentChildSessionKind::WorkflowPlanning);
    assert_eq!(agent_session_status_label(children[0].kind, children[0].status), "正在规划");
    assert!(!agent_sessions_popover(&app, &children).as_widget().children().is_empty());
}

#[test]
fn agent_sessions_popover_lists_nested_plan_run_descendants_and_stops_owner() {
    const DIRECTORY: &str = "/workspace/project";
    const ROOT_SESSION_ID: &str = "root-session";
    const TASK_ID: &str = "task-nested";
    const WORKFLOW_SESSION_ID: &str = "task-nested-1";
    const AGENT_SESSION_ID: &str = "agent-grandchild";

    let mut workflow_session =
        session(WORKFLOW_SESSION_ID, Some(ROOT_SESSION_ID), "实现递归会话树", DIRECTORY);
    workflow_session.slug = Info::plan_run_slug(WORKFLOW_SESSION_ID);

    let workflow_event = serde_json::json!({
        "status": "running",
        "metadata": {
            "canonical_tool_id": "workflow_node",
            "plan_run": true,
            "event_type": "subtask_started",
            "task_id": TASK_ID,
            "session_id": WORKFLOW_SESSION_ID,
            "title": "实现递归会话树",
        },
    });
    let workflow_chat = vec![assistant(&format!("tool workflow_node\n{workflow_event}\n"))];
    let agent_chat = vec![assistant(&agent_tool_event(
        "waiting",
        WORKFLOW_SESSION_ID,
        AGENT_SESSION_ID,
        "reviewer",
        "等待递归后代完成",
    ))];

    let (mut app, _) = crate::app::App::new();
    app.active_session_id = Some(ROOT_SESSION_ID.to_string());
    app.sessions = vec![
        session(ROOT_SESSION_ID, None, "根会话", DIRECTORY),
        workflow_session,
        session(AGENT_SESSION_ID, Some(WORKFLOW_SESSION_ID), "检查孙会话", DIRECTORY),
    ];
    app.chat = workflow_chat;
    app.chat_message_ids = vec![None];
    app.store_session_chat_snapshot(
        WORKFLOW_SESSION_ID.to_string(),
        crate::app::session::shared_chat_messages(agent_chat),
        vec![None],
    );
    app.store_session_chat_snapshot(
        AGENT_SESSION_ID.to_string(),
        crate::app::session::shared_chat_messages(vec![assistant(&agent_tool_event(
            "running",
            AGENT_SESSION_ID,
            ROOT_SESSION_ID,
            "writer",
            "malformed lifecycle cycle",
        ))]),
        vec![None],
    );
    app.get_session_runtime_mut(WORKFLOW_SESSION_ID).is_requesting = true;

    let children = running_agent_sessions(&app);

    assert_eq!(
        children.iter().map(|child| child.session_id.as_str()).collect::<Vec<_>>(),
        vec![WORKFLOW_SESSION_ID, AGENT_SESSION_ID]
    );
    assert_eq!(children[0].depth, 0);
    assert_eq!(children[0].plan_run_task_id.as_deref(), Some(TASK_ID));
    assert_eq!(children[1].depth, 1);
    assert_eq!(children[1].status, AgentChildSessionStatus::Waiting);
    assert_eq!(agent_session_status_label(children[1].kind, children[1].status), "等待中");
    match agent_session_open_message(&children[1]) {
        Some(Message::Chat(message::ChatMessage::OpenAgentSession { session_id, .. })) => {
            assert_eq!(session_id, AGENT_SESSION_ID);
        }
        other => panic!("expected nested child session open message, got {other:?}"),
    }
    assert!(agent_session_stop_message(&children[1]).is_none());
    assert!(matches!(
        agent_session_stop_message(&children[0]),
        Some(Message::Chat(message::ChatMessage::StopPlanRun(task_id))) if task_id == TASK_ID
    ));
    assert!(!agent_sessions_popover(&app, &children).as_widget().children().is_empty());

    let _ = crate::app::message::chat::input::update(
        &mut app,
        message::ChatMessage::StopPlanRunFinished {
            task_id: TASK_ID.to_string(),
            result: Ok(vw_gateway_client::vw_api_types::task::TaskPlanRunStatusResponse {
                task_id: TASK_ID.to_string(),
                status: vw_gateway_client::vw_api_types::task::TaskPlanRunStatus::Cancelled,
                current_session_id: Some(WORKFLOW_SESSION_ID.to_string()),
                terminal_event: None,
            }),
        },
    );
    assert!(
        app.get_session_runtime(WORKFLOW_SESSION_ID).is_requesting,
        "successful stop waits for SSE cancelled before hiding the row"
    );
    assert!(
        app.active_toast.as_ref().is_some_and(|toast| toast.message.contains("等待运行状态收敛"))
    );

    for error in ["task-plan-run 不存在", "当前租户不可见", "gateway request failed"] {
        let _ = crate::app::message::chat::input::update(
            &mut app,
            message::ChatMessage::StopPlanRunFinished {
                task_id: TASK_ID.to_string(),
                result: Err(error.to_string()),
            },
        );
        assert!(app.active_toast.as_ref().is_some_and(|toast| toast.message.contains(error)));
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn agent_tool_live_child_integrates_with_input_session_list() {
    const REQUEST_ID: u64 = 42;
    const DIRECTORY: &str = "/workspace/project";
    const PARENT_ID: &str = "parent-session";
    const SUCCESS_CHILD_ID: &str = "child-success";
    const FAILED_CHILD_ID: &str = "child-failed";

    let (mut app, _) = crate::app::App::new();
    app.active_session_id = Some(PARENT_ID.to_string());
    app.sessions = vec![
        session(PARENT_ID, None, "父会话", DIRECTORY),
        session(SUCCESS_CHILD_ID, Some(PARENT_ID), "检查输入面板工作中入口", DIRECTORY),
        session(FAILED_CHILD_ID, Some(PARENT_ID), "检查失败终态收口", DIRECTORY),
    ];
    app.chat = vec![assistant("AgentTool integration")];
    app.chat_message_ids = vec![None];
    app.get_session_runtime_mut(PARENT_ID).active_agent_request =
        Some(agent_request(REQUEST_ID, PARENT_ID));

    let (delta_tx, mut delta_rx) = tokio::sync::mpsc::unbounded_channel();
    let (release_success_tx, release_success_rx) = tokio::sync::oneshot::channel();
    let producer = tokio::spawn(async move {
        delta_tx
            .send(agent_tool_event(
                "running",
                PARENT_ID,
                SUCCESS_CHILD_ID,
                "writer",
                "inspect the live child",
            ))
            .expect("send running AgentTool event");
        release_success_rx.await.expect("release successful child");
        delta_tx
            .send(agent_tool_event(
                "completed",
                PARENT_ID,
                SUCCESS_CHILD_ID,
                "writer",
                "inspect the live child",
            ))
            .expect("send completed AgentTool event");

        let (release_failed_tx, release_failed_rx) = tokio::sync::oneshot::channel();
        delta_tx
            .send(agent_tool_event(
                "running",
                PARENT_ID,
                FAILED_CHILD_ID,
                "reviewer",
                "inspect the failed child",
            ))
            .expect("send second running AgentTool event");
        release_failed_tx.send(()).expect("release failed child");
        release_failed_rx.await.expect("receive failed child release");
        delta_tx
            .send(agent_tool_event(
                "error",
                PARENT_ID,
                FAILED_CHILD_ID,
                "reviewer",
                "inspect the failed child",
            ))
            .expect("send failed AgentTool event");
    });

    let running_delta = delta_rx.recv().await.expect("running event before child completion");
    let _ = crate::app::message::chat::update(
        &mut app,
        message::ChatMessage::AgentStreamDelta(REQUEST_ID, running_delta),
    );

    let children = running_agent_sessions(&app);
    assert_eq!(children.len(), 1);
    assert_eq!(children[0].session_id, SUCCESS_CHILD_ID);
    assert_eq!(children[0].title, "检查输入面板工作中入口");
    assert_eq!(children[0].directory, DIRECTORY);
    assert!(agent_sessions_entry(&app, &children).is_some());
    assert!(!agent_sessions_popover(&app, &children).as_widget().children().is_empty());

    let _ = crate::app::message::chat::update(
        &mut app,
        message::ChatMessage::ToggleAgentSessionsPopover,
    );
    assert!(app.show_agent_sessions_popover);
    let open_message =
        agent_session_open_message(&children[0]).expect("running child should be openable");
    let Message::Chat(open_message) = open_message else {
        panic!("expected chat open message");
    };
    let open_task = crate::app::message::chat::update(&mut app, open_message);
    assert!(!app.show_agent_sessions_popover);
    assert!(matches!(
        collect_task_messages(open_task).await.as_slice(),
        [Message::Project(message::ProjectMessage::OpenProjectSessionPressed(path, id))]
            if path == DIRECTORY && id == SUCCESS_CHILD_ID
    ));

    release_success_tx.send(()).expect("release successful child executor");
    let completed_delta = delta_rx.recv().await.expect("successful terminal event");
    let _ = crate::app::message::chat::update(
        &mut app,
        message::ChatMessage::AgentStreamDelta(REQUEST_ID, completed_delta),
    );
    assert!(running_agent_sessions(&app).is_empty());

    let failed_running_delta = delta_rx.recv().await.expect("failed child running event");
    let _ = crate::app::message::chat::update(
        &mut app,
        message::ChatMessage::AgentStreamDelta(REQUEST_ID, failed_running_delta),
    );
    assert_eq!(
        running_agent_sessions(&app)
            .iter()
            .map(|child| child.session_id.as_str())
            .collect::<Vec<_>>(),
        vec![FAILED_CHILD_ID]
    );
    let _ = crate::app::message::chat::update(
        &mut app,
        message::ChatMessage::ToggleAgentSessionsPopover,
    );
    assert!(app.show_agent_sessions_popover);

    let failed_delta = delta_rx.recv().await.expect("failed terminal event");
    let _ = crate::app::message::chat::update(
        &mut app,
        message::ChatMessage::AgentStreamDelta(REQUEST_ID, failed_delta),
    );
    assert!(running_agent_sessions(&app).is_empty());
    assert!(!app.show_agent_sessions_popover);
    producer.await.expect("AgentTool event producer should finish");
}

#[test]
fn agent_tool_batch_lifecycle_stream_refreshes_badge_and_closes_popover() {
    const REQUEST_ID: u64 = 77;
    const PARENT_ID: &str = "parent-batch";
    let (mut app, _) = crate::app::App::new();
    app.active_session_id = Some(PARENT_ID.to_string());
    app.sessions = vec![session(PARENT_ID, None, "父会话", "/workspace/project")];
    app.chat = vec![assistant("")];
    app.chat_message_ids = vec![None];
    app.get_session_runtime_mut(PARENT_ID).active_agent_request =
        Some(agent_request(REQUEST_ID, PARENT_ID));

    let running = format!(
        "{}{}",
        agent_tool_event("running", PARENT_ID, "child-1", "writer", "write"),
        agent_tool_event("running", PARENT_ID, "child-2", "reviewer", "review")
    );
    let _ = crate::app::message::chat::update(
        &mut app,
        message::ChatMessage::AgentStreamDelta(REQUEST_ID, running),
    );
    assert_eq!(running_agent_sessions(&app).len(), 2);
    let _ = crate::app::message::chat::update(
        &mut app,
        message::ChatMessage::ToggleAgentSessionsPopover,
    );
    assert!(app.show_agent_sessions_popover);

    let mixed_output = serde_json::json!({
        "action": "wait",
        "timed_out": true,
        "tasks": [
            {
                "parent_session_id": PARENT_ID,
                "child_session_id": "child-1",
                "agent": "writer",
                "task": "write",
                "state": "succeeded"
            },
            {
                "parent_session_id": PARENT_ID,
                "child_session_id": "child-2",
                "agent": "reviewer",
                "task": "review",
                "state": "running"
            }
        ]
    });
    let mixed = format!(
        "tool AgentTool\n{}\n",
        serde_json::json!({"status":"waiting","output":mixed_output.to_string()})
    );
    let _ = crate::app::message::chat::update(
        &mut app,
        message::ChatMessage::AgentStreamDelta(REQUEST_ID, mixed),
    );
    let children = running_agent_sessions(&app);
    assert_eq!(children.len(), 1);
    assert_eq!(children[0].session_id, "child-2");
    assert_eq!(children[0].status, AgentChildSessionStatus::Waiting);
    assert!(app.show_agent_sessions_popover);

    let terminal_output = serde_json::json!({
        "action": "wait",
        "timed_out": false,
        "tasks": [{
            "parent_session_id": PARENT_ID,
            "child_session_id": "child-2",
            "agent": "reviewer",
            "task": "review",
            "state": "succeeded"
        }]
    });
    let terminal = format!(
        "tool AgentTool\n{}\n",
        serde_json::json!({"status":"completed","output":terminal_output.to_string()})
    );
    let _ = crate::app::message::chat::update(
        &mut app,
        message::ChatMessage::AgentStreamDelta(REQUEST_ID, terminal),
    );
    assert!(running_agent_sessions(&app).is_empty());
    assert!(!app.show_agent_sessions_popover);
}
