use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::PathBuf;
use std::thread;

use serde_json::json;
use vw_gateway_client::vw_api_types::agent_scheduler::{
    AgentSchedulerCommandKindDto, AgentSchedulerScopeDto,
};
use vw_gateway_client::vw_api_types::id::{ProjectId, ProjectScope, TenantScope};
use vw_gateway_client::{GatewayClient, GatewayEndpoint};

use super::runtime::gateway::{GatewaySessionSeed, GatewayUiRuntime, block_on_gateway};
use super::runtime::orchestration::{
    control_owned_task_blocking, load_orchestration,
};

fn runtime(port: u16) -> GatewayUiRuntime {
    GatewayUiRuntime::new(
        GatewayClient::new(GatewayEndpoint::new("127.0.0.1", port)).expect("client"),
        GatewaySessionSeed::new(PathBuf::from("/workspace"))
            .with_id(Some("orchestration-session".to_string())),
    )
}

fn scope() -> AgentSchedulerScopeDto {
    AgentSchedulerScopeDto::new(
        ProjectScope::new(TenantScope::default(), ProjectId::from("project-one"))
            .expect("project scope"),
    )
}

#[test]
fn orchestration_panels_render_gateway_snapshots() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("fixture");
    let port = listener.local_addr().expect("address").port();
    let server = thread::spawn(move || {
        let mut paths = Vec::new();
        for _ in 0..4 {
            let (mut stream, _) = listener.accept().expect("accept");
            let request = read_request(&mut stream);
            let path = request.lines().next().unwrap_or_default().split_whitespace().nth(1).unwrap_or_default();
            paths.push(path.to_string());
            let body = match path {
                "/v1/workflow/applications" => json!([{
                    "uuid":"wf-1","name":"Deploy","description":"Ship safely",
                    "created_at_ms":1,"updated_at_ms":2
                }]),
                "/v1/cron" => json!({"jobs":[{
                    "id":"cron-1","name":"Nightly","schedule_kind":"cron",
                    "expression":"0 0 * * *","next_run":"tomorrow","enabled":true
                }]}),
                path if path.ends_with("/tasks") => json!({
                    "project_id":"project-one",
                    "tasks":[{
                        "project_id":"project-one","task_id":"task-1","status_key":"running",
                        "status":"running","priority":1,"order_no":0,"model":"auto",
                        "prompt":"work","retry_count":0,"updated_at_ms":3,
                        "read_model":{"title":"Owned task","status_label":"Running"}
                    }]
                }),
                path if path.ends_with("/workers") => json!({"workers":[{
                    "id":"worker-1","model":"auto","status":"idle","worker_role":"reviewer",
                    "team_id":"team-1","room_id":"room-1","updated_at_ms":4
                }]}),
                _ => json!({}),
            };
            write_json(&mut stream, &body.to_string());
        }
        paths
    });

    let snapshot = block_on_gateway(async { Ok(load_orchestration(&runtime(port), Some(scope())).await) })
        .expect("snapshot");
    assert_eq!(snapshot.workflows.items[0].uuid, "wf-1");
    assert_eq!(snapshot.schedules.items[0].id, "cron-1");
    assert_eq!(snapshot.agents.members[0].id, "worker-1");
    assert_eq!(snapshot.tasks.tasks[0].id, "task-1");
    assert!(snapshot.tasks.tasks[0].supports_stop);
    assert!(snapshot.warnings.is_empty(), "{:?}", snapshot.warnings);
    assert_eq!(server.join().expect("server").len(), 4);
}

#[test]
fn orchestration_controls_require_owned_stable_ids() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("fixture");
    let port = listener.local_addr().expect("address").port();
    let snapshot = super::runtime::orchestration::OrchestrationSnapshot {
        scope: Some(scope()),
        tasks: super::model::background_task::BackgroundTaskPanel {
            tasks: vec![super::model::background_task::BackgroundTask {
                id: "owned-task".to_string(),
                session_id: "project-one".to_string(),
                kind: super::model::background_task::BackgroundTaskKind::Agent,
                title: "Owned".to_string(),
                status: super::model::background_task::BackgroundTaskStatus::Running,
                detail: "running".to_string(),
                log: Vec::new(),
                supports_stop: true,
            }],
            ..Default::default()
        },
        ..Default::default()
    };

    let error = control_owned_task_blocking(
        &runtime(port),
        &snapshot,
        "foreign-task",
        AgentSchedulerCommandKindDto::Cancel,
    )
    .expect_err("foreign ID rejected before request");
    assert!(error.contains("owned by the current project"));

    let server = thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("owned command");
        let request = read_request(&mut stream);
        write_json(&mut stream, r#"{"accepted":true}"#);
        request
    });
    control_owned_task_blocking(
        &runtime(port),
        &snapshot,
        "owned-task",
        AgentSchedulerCommandKindDto::Cancel,
    )
    .expect("owned command");
    let request = server.join().expect("server");
    assert!(request.starts_with("POST /v1/agent-scheduler/projects/project-one/commands "));
    assert!(request.contains(r#""task_id":"owned-task""#));
    assert!(request.contains(r#""command":"cancel""#));
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
        if let Some(header_end) = bytes.windows(4).position(|window| window == b"\r\n\r\n") {
            let headers = String::from_utf8_lossy(&bytes[..header_end + 4]);
            let length = headers
                .lines()
                .find_map(|line| line.to_ascii_lowercase().strip_prefix("content-length:").map(str::trim).and_then(|value| value.parse::<usize>().ok()))
                .unwrap_or_default();
            if bytes.len() >= header_end + 4 + length {
                break;
            }
        }
    }
    String::from_utf8(bytes).expect("utf8")
}

fn write_json(stream: &mut std::net::TcpStream, body: &str) {
    let response = format!(
        "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
        body.len()
    );
    stream.write_all(response.as_bytes()).expect("write");
}
