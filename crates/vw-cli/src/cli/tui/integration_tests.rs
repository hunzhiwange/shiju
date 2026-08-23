use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::PathBuf;
use std::thread;

use vw_gateway_client::{GatewayClient, GatewayEndpoint};
use vw_tui::{V3Action, V3ApprovalDecision};

use super::app::V3GatewayApp;
use super::commands::{
    CommandRoute, command_palette_items_with_integrations, route_slash_submission,
};
use super::controller::TuiControllerCommand;
use super::model::integration::{IntegrationItem, IntegrationKind};
use super::model::{UiConfirmIntent, UiOverlay};
use super::runtime::gateway::{GatewaySessionSeed, GatewayUiRuntime};
use super::runtime::integrations::{delete_integration_blocking, load_integrations_blocking};
use super::state::TuiState;

fn runtime(port: u16, root: PathBuf) -> GatewayUiRuntime {
    GatewayUiRuntime::new(
        GatewayClient::new(GatewayEndpoint::new("127.0.0.1", port)).expect("client"),
        GatewaySessionSeed::new(root).with_id(Some("integration-session".to_string())),
    )
}

fn integration_fixture() -> (u16, thread::JoinHandle<Vec<String>>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("fixture");
    let port = listener.local_addr().expect("address").port();
    let handle = thread::spawn(move || {
        let mut paths = Vec::new();
        for _ in 0..4 {
            let (mut stream, _) = listener.accept().expect("accept");
            let request = read_request(&mut stream);
            let path = request.lines().next().unwrap_or_default().split_whitespace().nth(1).unwrap_or_default();
            paths.push(path.to_string());
            let body = match path {
                path if path.starts_with("/v1/skills") => r#"[
                    {"id":"review","title":"Project Review","description":"project copy","kind":"local","resource_count":1,"installed":true,"enabled":true,"source":"project"},
                    {"id":"review","title":"Built-in Review","description":"built-in copy","kind":"built-in","resource_count":1,"installed":true,"enabled":true,"source":"built-in"}
                ]"#,
                "/v1/plugins" => r#"[{"id":"demo","display_name":"Demo","version":"1.0","description":"plugin","enabled":true,"component_count":2}]"#,
                "/v1/mcp/inspect" => r#"{"servers":[{"name":"docs","transport":"http","enabled":true,"required":false,"connection_status":"connected","tool_count":1,"resource_count":2}]}"#,
                path if path.starts_with("/v1/memory") => r#"{"entries":[{"key":"project-note","title":"Note","content":"safe preview","scope":"project"}]}"#,
                _ => "{}",
            };
            write_json(&mut stream, body);
        }
        paths
    });
    (port, handle)
}

#[test]
fn dynamic_integration_catalog_merges_healthy_sources() {
    let temp = tempfile::tempdir().expect("workspace");
    let (port, server) = integration_fixture();
    let loaded = load_integrations_blocking(&runtime(port, temp.path().to_path_buf()));

    assert!(loaded.warnings.is_empty(), "{:?}", loaded.warnings);
    assert_eq!(loaded.items.len(), 4);
    let skill = loaded
        .items
        .iter()
        .find(|item| item.kind == IntegrationKind::Skill)
        .expect("deduplicated skill");
    assert_eq!(skill.title, "Project Review");
    assert_eq!(skill.source, "project");
    let commands = command_palette_items_with_integrations(&loaded.items);
    assert!(commands.iter().any(|item| item.label == "/skill:review"));
    assert_eq!(
        route_slash_submission("/skill:review inspect safely").expect("route"),
        Some(CommandRoute::ExpandIntegrationPrompt {
            kind: IntegrationKind::Skill,
            id: "review".to_string(),
            arguments: "inspect safely".to_string(),
        })
    );
    let paths = server.join().expect("server");
    assert_eq!(paths.len(), 4);
}

#[test]
fn integration_actions_use_stable_ids_and_confirmation() {
    let temp = tempfile::tempdir().expect("workspace");
    let item = plugin_item("demo-plugin");
    let app_runtime = runtime(9, temp.path().to_path_buf());
    let mut state = TuiState::default();
    state.project.workspace_root = Some(temp.path().to_path_buf());
    state.integration_catalog = vec![item.clone()];
    let mut app = V3GatewayApp::new(app_runtime, state);

    app.open_integration_delete_confirmation(IntegrationKind::Plugin, "demo-plugin");
    let UiOverlay::Confirm(confirm) = app.state().overlays.active().expect("confirmation") else {
        panic!("expected typed confirmation");
    };
    assert!(confirm.destructive);
    assert_eq!(
        confirm.intent,
        UiConfirmIntent::IntegrationDelete {
            kind: IntegrationKind::Plugin,
            id: "demo-plugin".to_string(),
        }
    );
    let cancel = app
        .handle_action(V3Action::ResolveApproval {
            request_id: confirm.request_id.clone(),
            decision: V3ApprovalDecision::Cancel,
        })
        .expect("cancel route");
    assert!(matches!(
        cancel,
        TuiControllerCommand::Overlay(super::controller::TuiOverlayCommand::ConfirmDecision {
            confirmed: false,
            ..
        })
    ));

    let listener = TcpListener::bind("127.0.0.1:0").expect("delete fixture");
    let port = listener.local_addr().expect("address").port();
    let server = thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept delete");
        let request = read_request(&mut stream);
        write_json(&mut stream, r#"{"path":"/plugins/demo-plugin"}"#);
        request
    });
    delete_integration_blocking(&runtime(port, temp.path().to_path_buf()), &item)
        .expect("delete stable plugin");
    let request = server.join().expect("delete server");
    assert!(request.starts_with("POST /v1/plugins/delete "));
    assert!(request.contains(r#""plugin_id":"demo-plugin""#));
}

fn plugin_item(id: &str) -> IntegrationItem {
    IntegrationItem {
        kind: IntegrationKind::Plugin,
        id: id.to_string(),
        title: "Demo".to_string(),
        detail: "plugin".to_string(),
        source: "gateway:plugin".to_string(),
        enabled: true,
        can_toggle: true,
        can_delete: true,
        prompt_prefix: None,
        actions: Vec::new(),
    }
}

fn read_request(stream: &mut std::net::TcpStream) -> String {
    let mut bytes = Vec::new();
    let mut buffer = [0_u8; 2048];
    loop {
        let read = stream.read(&mut buffer).expect("read request");
        if read == 0 {
            break;
        }
        bytes.extend_from_slice(&buffer[..read]);
        if let Some(header_end) = bytes.windows(4).position(|window| window == b"\r\n\r\n") {
            let headers = String::from_utf8_lossy(&bytes[..header_end + 4]);
            let content_length = headers
                .lines()
                .find_map(|line| line.to_ascii_lowercase().strip_prefix("content-length:").map(str::trim).and_then(|value| value.parse::<usize>().ok()))
                .unwrap_or_default();
            if bytes.len() >= header_end + 4 + content_length {
                break;
            }
        }
    }
    String::from_utf8(bytes).expect("utf8 request")
}

fn write_json(stream: &mut std::net::TcpStream, body: &str) {
    let response = format!(
        "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
        body.len()
    );
    stream.write_all(response.as_bytes()).expect("write response");
}
