use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::PathBuf;
use std::thread;

use vw_gateway_client::vw_api_types::task::TaskPlanRunStreamEventDto;
use vw_gateway_client::{GatewayClient, GatewayEndpoint};

use super::app::V3GatewayApp;
use super::runtime::gateway::{GatewaySessionSeed, GatewayUiRuntime};
use super::runtime::plan_run::{stop_owned_plan_run_blocking, validate_owned_plan_run};
use super::runtime::stream_adapter::adapt_task_plan_run_event;
use super::state::TuiState;

fn test_app(port: u16) -> V3GatewayApp {
    let runtime = GatewayUiRuntime::new(
        GatewayClient::new(GatewayEndpoint::new("127.0.0.1", port)).expect("client"),
        GatewaySessionSeed::new(PathBuf::from("/workspace"))
            .with_id(Some("plan-session".to_string())),
    );
    let mut state = TuiState::default();
    state.session.session_id = Some("plan-session".to_string());
    state.runtime.active_plan_run_request_id = Some("owned-request".to_string());
    V3GatewayApp::new(runtime, state)
}

#[test]
fn plan_goal_events_reach_the_active_transcript() {
    let mut app = test_app(9);
    let events = vec![
        TaskPlanRunStreamEventDto::PlanningStarted {
            task_id: "owned-task".to_string(),
            plan_dir: "/workspace/.plans/run".to_string(),
            session_id: "plan-child".to_string(),
        },
        TaskPlanRunStreamEventDto::BatchStarted {
            task_id: "owned-task".to_string(),
            batch: 1,
            subtask_ids: vec!["child-one".to_string()],
        },
        TaskPlanRunStreamEventDto::SubtaskStarted {
            task_id: "owned-task".to_string(),
            batch: 1,
            subtask_id: "child-one".to_string(),
            title: "实现闭环".to_string(),
            task_file: "task.md".to_string(),
            session_id: "child-session".to_string(),
        },
        TaskPlanRunStreamEventDto::SessionWaiting {
            task_id: "owned-task".to_string(),
            session_id: "plan-child".to_string(),
            active_descendant_session_ids: vec!["child-session".to_string()],
        },
        TaskPlanRunStreamEventDto::Finished {
            task_id: "owned-task".to_string(),
            plan_dir: "/workspace/.plans/run".to_string(),
            status: "completed".to_string(),
            output: "目标已达成".to_string(),
            session_id: Some("plan-child".to_string()),
        },
    ];
    for event in events {
        for event in adapt_task_plan_run_event(event) {
            app.accept_runtime_event(event);
        }
    }

    let transcript = app
        .transcript_lines(100, true)
        .into_iter()
        .flat_map(|line| line.spans)
        .map(|span| span.content.into_owned())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(transcript.contains("正在规划"));
    assert!(transcript.contains("批次 1"));
    assert!(transcript.contains("实现闭环"));
    assert!(transcript.contains("等待 1 个子智能体"));
    assert!(transcript.contains("目标已达成"));
    assert_eq!(app.state().runtime.active_plan_run_request_id, None);
    assert_eq!(app.state().runtime.active_plan_run_task_id, None);
}

#[test]
fn plan_goal_stop_requires_owned_identifiers() {
    let mismatch = validate_owned_plan_run(
        Some("owned-request"),
        Some("owned-task"),
        "foreign-request",
        "owned-task",
    )
    .expect_err("foreign request must be rejected before I/O");
    assert!(mismatch.contains("not owned"));

    let listener = TcpListener::bind("127.0.0.1:0").expect("fixture");
    let port = listener.local_addr().expect("address").port();
    let server = thread::spawn(move || {
        let mut requests = Vec::new();
        for body in [r#"{"cancelled":true}"#, r#"{"task_id":"owned-task","status":"cancelled"}"#] {
            let (mut stream, _) = listener.accept().expect("accept");
            requests.push(read_request(&mut stream));
            write_json(&mut stream, body);
        }
        requests
    });

    let stopped = stop_owned_plan_run_blocking(
        test_app(port).runtime(),
        Some("owned-request"),
        Some("owned-task"),
        "owned-request",
        "owned-task",
    )
    .expect("owned plan-run stops");
    assert!(stopped);
    let requests = server.join().expect("server");
    assert!(requests[0].starts_with("POST /v1/task-plan-run/stream/owned-request/stop "));
    assert!(requests[1].starts_with("POST /v1/task-plan-run/owned-task/stop "));
}

fn read_request(stream: &mut std::net::TcpStream) -> String {
    let mut bytes = Vec::new();
    let mut buffer = [0_u8; 2048];
    loop {
        let read = stream.read(&mut buffer).expect("read");
        if read == 0 {
            break;
        }
        bytes.extend_from_slice(&buffer[..read]);
        let text = String::from_utf8_lossy(&bytes);
        if let Some(header_end) = text.find("\r\n\r\n") {
            let content_length = text[..header_end]
                .lines()
                .find_map(|line| line.strip_prefix("content-length: "))
                .and_then(|value| value.trim().parse::<usize>().ok())
                .unwrap_or_default();
            if bytes.len() >= header_end + 4 + content_length {
                break;
            }
        }
    }
    String::from_utf8(bytes).expect("request")
}

fn write_json(stream: &mut std::net::TcpStream, body: &str) {
    let response = format!(
        "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
        body.len()
    );
    stream.write_all(response.as_bytes()).expect("response");
}
