use std::fmt::Write as _;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::PathBuf;
use std::thread;

use vw_gateway_client::vw_api_types::task::{
    TaskPlanRunMode, TaskPlanRunRequest, TaskPlanRunStatus, TaskPlanRunStreamEventDto,
};
use vw_gateway_client::{GatewayClient, GatewayEndpoint};

use super::gateway::{GatewaySessionSeed, GatewayUiRuntime};
use super::plan_run::plan_run_request_for_submission;
use super::stream_adapter::{
    UiRuntimeEvent, UiRuntimeTerminalEvent, adapt_task_plan_run_event,
};
use crate::cli::tui_v2::model::{PlanRunMode, PromptSubmission, UiMessage};
use crate::cli::tui_v2::state::{TuiState, apply_runtime_event};

#[test]
fn workflow_mode_streams_task_plan_run_into_tui_state() {
    let events = workflow_fixture_events();
    let mut response_body = String::new();
    for (index, event) in events.iter().enumerate() {
        writeln!(
            response_body,
            "id: {}\ndata: {}\n",
            index + 1,
            serde_json::to_string(event).expect("serialize plan-run event")
        )
        .expect("append SSE fixture");
    }
    let (port, server) =
        spawn_response_server("text/event-stream", response_body, 1);
    let runtime = runtime_for_port(port);

    let mut submission = PromptSubmission::new("请实现工作流模式")
        .with_root("/repo")
        .with_model("provider/model")
        .with_session_id("parent-session");
    submission.execution.plan_run_mode = PlanRunMode::Workflow;
    submission.execution.delegate_agent = Some("reviewer".to_string());
    submission.execution.acp_backend = Some("codex".to_string());

    let request = plan_run_request_for_submission(&submission)
        .expect("workflow route")
        .expect("explicit execution should use plan-run");
    assert_eq!(
        request,
        TaskPlanRunRequest {
            query: "请实现工作流模式".to_string(),
            root: "/repo".to_string(),
            model: Some("provider/model".to_string()),
            agent: Some("reviewer".to_string()),
            acp_agent: Some("codex".to_string()),
            parent_session_id: Some("parent-session".to_string()),
            mode: TaskPlanRunMode::Workflow,
        }
    );

    let mut state = TuiState::default();
    let terminal = runtime.stream_task_plan_run_blocking(&request, |event| {
        apply_runtime_event(&mut state, event);
        true
    });
    let requests = server.join().expect("join plan-run stream server");
    let body = request_json(&requests[0]);
    let recorded: TaskPlanRunRequest =
        serde_json::from_value(body).expect("decode recorded TaskPlanRunRequest");

    assert_eq!(recorded.mode, TaskPlanRunMode::Workflow);
    assert_eq!(recorded.root, "/repo");
    assert_eq!(recorded.agent.as_deref(), Some("reviewer"));
    assert_eq!(recorded.acp_agent.as_deref(), Some("codex"));
    assert_eq!(
        terminal,
        UiRuntimeTerminalEvent::Done {
            finish_reason: Some("completed".to_string()),
            usage: None,
            message_id: None,
            parent_message_id: None,
        }
    );
    assert!(state.runtime.active_plan_run_task_id.is_none());
    assert!(state.messages.iter().any(|message| {
        matches!(
            message,
            UiMessage::Progress(progress)
                if progress.label.contains("计划已创建")
                    && progress.current == Some(1)
                    && progress.completed
        )
    }));
    assert!(state.messages.iter().any(|message| {
        matches!(
            message,
            UiMessage::Progress(progress)
                if progress.label.contains("实现核心") && progress.completed
        )
    }));
    assert!(state.messages.iter().any(|message| {
        matches!(
            message,
            UiMessage::Assistant(assistant)
                if assistant.text.contains("最终完成")
        )
    }));

    let discussion = workflow_submission("如何实现工作流模式？", Some("/repo"));
    let discussion_request = plan_run_request_for_submission(&discussion)
        .expect("discussion route")
        .expect("workflow toggle must enter plan-run for any non-empty query");
    assert_eq!(discussion_request.mode, TaskPlanRunMode::Workflow);
    assert_eq!(discussion_request.query, "如何实现工作流模式？");
    let missing_root = workflow_submission("实现工作流模式", None);
    assert!(
        plan_run_request_for_submission(&missing_root)
            .expect_err("execution without a root must be rejected")
            .contains("根目录")
    );

    assert!(matches!(
        adapt_task_plan_run_event(TaskPlanRunStreamEventDto::Blocked {
            task_id: "task-1".to_string(),
            batch: Some(1),
            subtask_id: Some("task-a".to_string()),
            reason: "需要人工输入".to_string(),
            session_id: "task-1-a".to_string(),
        })
        .last(),
        Some(UiRuntimeEvent::Terminal(UiRuntimeTerminalEvent::Error(message)))
            if message.contains("需要人工输入")
    ));
    assert!(matches!(
        adapt_task_plan_run_event(TaskPlanRunStreamEventDto::Error {
            task_id: Some("task-1".to_string()),
            message: "gateway failed".to_string(),
        })
        .as_slice(),
        [UiRuntimeEvent::Terminal(UiRuntimeTerminalEvent::Error(message))]
            if message == "gateway failed"
    ));
    assert!(matches!(
        adapt_task_plan_run_event(TaskPlanRunStreamEventDto::Cancelled {
            task_id: "task-1".to_string(),
            session_id: Some("task-1-a".to_string()),
        })
        .as_slice(),
        [UiRuntimeEvent::Terminal(UiRuntimeTerminalEvent::Cancelled { .. })]
    ));

    let stop_body = serde_json::json!({
        "task_id": "task-1",
        "status": "cancelled",
        "current_session_id": "task-1-a"
    })
    .to_string();
    let (stop_port, stop_server) =
        spawn_response_server("application/json", stop_body, 1);
    let stopped = runtime_for_port(stop_port)
        .task_plan_run_stop_blocking("task-1")
        .expect("stop plan-run");
    let stop_requests = stop_server.join().expect("join stop server");
    assert_eq!(stopped.status, TaskPlanRunStatus::Cancelled);
    assert!(stop_requests[0].starts_with("POST /v1/task-plan-run/task-1/stop "));
}

#[test]
fn goal_mode_streams_turns_and_evaluations_until_terminal() {
    let goal_events = [
        TaskPlanRunStreamEventDto::PlanningStarted {
            task_id: "goal-1".to_string(),
            plan_dir: "/repo/.plans/goal-1".to_string(),
            session_id: "goal-1-plan".to_string(),
        },
        TaskPlanRunStreamEventDto::GoalTurnStarted {
            task_id: "goal-1".to_string(),
            turn: 1,
            session_id: "goal-1-plan".to_string(),
            reason: "开始执行".to_string(),
        },
        TaskPlanRunStreamEventDto::GoalEvaluated {
            task_id: "goal-1".to_string(),
            turn: 1,
            achieved: false,
            reason: "测试尚未通过".to_string(),
            session_id: "goal-1-plan".to_string(),
        },
        TaskPlanRunStreamEventDto::GoalTurnStarted {
            task_id: "goal-1".to_string(),
            turn: 2,
            session_id: "goal-1-plan".to_string(),
            reason: "继续修复".to_string(),
        },
        TaskPlanRunStreamEventDto::GoalEvaluated {
            task_id: "goal-1".to_string(),
            turn: 2,
            achieved: true,
            reason: "全部验证通过".to_string(),
            session_id: "goal-1-plan".to_string(),
        },
        TaskPlanRunStreamEventDto::Finished {
            task_id: "goal-1".to_string(),
            plan_dir: "/repo/.plans/goal-1".to_string(),
            status: "completed".to_string(),
            output: "目标完成".to_string(),
            session_id: Some("goal-1-plan".to_string()),
        },
    ];
    let mut response_body = String::new();
    for (index, event) in goal_events.iter().enumerate() {
        writeln!(
            response_body,
            "id: {}\ndata: {}\n",
            index + 1,
            serde_json::to_string(event).expect("serialize goal event")
        )
        .expect("append goal SSE fixture");
    }
    let (port, server) =
        spawn_response_server("text/event-stream", response_body, 1);
    let mut submission =
        PromptSubmission::new("持续实现并验收目标").with_root("/repo");
    submission.execution.plan_run_mode = PlanRunMode::Goal;

    let request = plan_run_request_for_submission(&submission)
        .expect("goal route")
        .expect("goal mode should use plan-run");

    assert_eq!(request.mode, TaskPlanRunMode::Goal);

    let mut state = TuiState::default();
    let mut runtime_events = Vec::new();
    let terminal = runtime_for_port(port).stream_task_plan_run_blocking(&request, |event| {
        runtime_events.push(event.clone());
        apply_runtime_event(&mut state, event);
        true
    });
    let requests = server.join().expect("join goal stream server");
    let recorded: TaskPlanRunRequest =
        serde_json::from_value(request_json(&requests[0])).expect("decode goal request");

    assert_eq!(recorded.mode, TaskPlanRunMode::Goal);
    assert_eq!(
        runtime_events
            .iter()
            .filter(|event| matches!(event, UiRuntimeEvent::Terminal(_)))
            .count(),
        1,
        "an unmet evaluation must keep the stream active"
    );
    assert!(state.messages.iter().any(|message| {
        matches!(
            message,
            UiMessage::Progress(progress)
                if progress.label.contains("目标评估 1")
                    && progress.label.contains("未达成")
                    && progress.label.contains("测试尚未通过")
                    && !progress.completed
        )
    }));
    assert!(state.messages.iter().any(|message| {
        matches!(
            message,
            UiMessage::Progress(progress)
                if progress.label.contains("目标续轮 2")
                    && progress.label.contains("继续修复")
        )
    }));
    assert!(state.messages.iter().any(|message| {
        matches!(
            message,
            UiMessage::Progress(progress)
                if progress.label.contains("目标评估 2")
                    && progress.label.contains("已达成")
                    && progress.label.contains("全部验证通过")
                    && progress.completed
        )
    }));
    assert!(matches!(
        terminal,
        UiRuntimeTerminalEvent::Done {
            finish_reason: Some(reason),
            ..
        } if reason == "completed"
    ));
}

fn workflow_submission(text: &str, root: Option<&str>) -> PromptSubmission {
    let mut submission = PromptSubmission::new(text);
    submission.execution.plan_run_mode = PlanRunMode::Workflow;
    if let Some(root) = root {
        submission = submission.with_root(root);
    }
    submission
}

fn workflow_fixture_events() -> Vec<TaskPlanRunStreamEventDto> {
    vec![
        TaskPlanRunStreamEventDto::PlanningStarted {
            task_id: "task-1".to_string(),
            plan_dir: "/repo/.plans/task-1".to_string(),
            session_id: "task-1-plan".to_string(),
        },
        TaskPlanRunStreamEventDto::SessionDelta {
            task_id: "task-1".to_string(),
            session_id: "task-1-plan".to_string(),
            delta: "规划中\n".to_string(),
        },
        TaskPlanRunStreamEventDto::PlanCreated {
            task_id: "task-1".to_string(),
            plan_dir: "/repo/.plans/task-1".to_string(),
            plan_md_path: "/repo/.plans/task-1/plan.md".to_string(),
            prompt_md_path: "/repo/.plans/task-1/prompt.md".to_string(),
            tasks_json_path: "/repo/.plans/task-1/tasks.json".to_string(),
            subtask_count: 1,
            session_id: "task-1-plan".to_string(),
        },
        TaskPlanRunStreamEventDto::BatchStarted {
            task_id: "task-1".to_string(),
            batch: 1,
            subtask_ids: vec!["task-a".to_string()],
        },
        TaskPlanRunStreamEventDto::SubtaskStarted {
            task_id: "task-1".to_string(),
            batch: 1,
            subtask_id: "task-a".to_string(),
            title: "实现核心".to_string(),
            task_file: "/repo/.plans/task-1/task-a.md".to_string(),
            session_id: "task-1-a".to_string(),
        },
        TaskPlanRunStreamEventDto::SubtaskFinished {
            task_id: "task-1".to_string(),
            batch: 1,
            subtask_id: "task-a".to_string(),
            title: "实现核心".to_string(),
            task_file: "/repo/.plans/task-1/task-a.md".to_string(),
            success: true,
            output: Some("子任务输出\n".to_string()),
            error: None,
            session_id: "task-1-a".to_string(),
        },
        TaskPlanRunStreamEventDto::BatchFinished {
            task_id: "task-1".to_string(),
            batch: 1,
            subtask_ids: vec!["task-a".to_string()],
            success: true,
        },
        TaskPlanRunStreamEventDto::Finished {
            task_id: "task-1".to_string(),
            plan_dir: "/repo/.plans/task-1".to_string(),
            status: "completed".to_string(),
            output: "最终完成".to_string(),
            session_id: Some("task-1-plan".to_string()),
        },
    ]
}

fn runtime_for_port(port: u16) -> GatewayUiRuntime {
    GatewayUiRuntime::new(
        GatewayClient::new(GatewayEndpoint::new("127.0.0.1", port))
            .expect("gateway client"),
        GatewaySessionSeed::new(PathBuf::from("/repo")),
    )
}

fn spawn_response_server(
    content_type: &'static str,
    response_body: String,
    request_count: usize,
) -> (u16, thread::JoinHandle<Vec<String>>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind plan-run fixture");
    let port = listener.local_addr().expect("fixture address").port();
    let server = thread::spawn(move || {
        (0..request_count)
            .map(|_| {
                let (mut stream, _) = listener.accept().expect("accept plan-run request");
                let request = read_request(&mut stream);
                let response = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{response_body}",
                    response_body.len()
                );
                stream.write_all(response.as_bytes()).expect("write fixture response");
                request
            })
            .collect()
    });
    (port, server)
}

fn read_request(stream: &mut TcpStream) -> String {
    let mut request = Vec::new();
    let mut buffer = [0_u8; 1024];
    let mut expected_len = None;
    loop {
        let read = stream.read(&mut buffer).expect("read fixture request");
        if read == 0 {
            break;
        }
        request.extend_from_slice(&buffer[..read]);
        if expected_len.is_none()
            && let Some(header_end) =
                request.windows(4).position(|window| window == b"\r\n\r\n")
        {
            let headers = String::from_utf8_lossy(&request[..header_end]);
            let content_length = headers
                .lines()
                .find_map(|line| {
                    line.to_ascii_lowercase()
                        .strip_prefix("content-length:")
                        .map(str::trim)
                        .and_then(|value| value.parse::<usize>().ok())
                })
                .unwrap_or(0);
            expected_len = Some(header_end + 4 + content_length);
        }
        if expected_len.is_some_and(|expected| request.len() >= expected) {
            break;
        }
    }
    String::from_utf8(request).expect("fixture request utf8")
}

fn request_json(request: &str) -> serde_json::Value {
    let (_, body) = request.split_once("\r\n\r\n").expect("request body");
    serde_json::from_str(body).expect("request JSON")
}
