#![allow(unused_imports, dead_code)]

use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::PathBuf;
use std::sync::Arc;
use std::thread;

use vw_gateway_client::{GatewayClient, GatewayEndpoint};
use vw_tui::{V3Action, V3ApprovalDecision};

use super::request::gateway_stream_request_from_state;
use super::app::{ActiveRunner, RunnerControl, V3GatewayApp};
use super::commands::{
    SETTINGS_ACCESS_SURFACE, SETTINGS_ACP_SURFACE, SETTINGS_EFFORT_SURFACE, SETTINGS_MODEL_SURFACE,
    SETTINGS_PERMISSION_SURFACE, SETTINGS_SUBAGENT_SURFACE,
};
use super::controller::build_prompt_submission;
use cli_interaction_model::settings_panel::{SettingKey, SettingsPanel, SettingsValues};
use cli_interaction_model::{
    AcpBackendOption, DelegateAgentOption, PermissionMode, ReasoningEffort,
    SessionCapabilityMatrix, UiOverlay,
};
use cli_gateway_capabilities::capabilities::load_session_capabilities_blocking;
use cli_gateway_transport::gateway::{GatewaySessionSeed, GatewayUiRuntime};
use cli_interaction_state::TuiState;

fn spawn_json_sequence_server(
    responses: Vec<&'static str>,
) -> (u16, thread::JoinHandle<Vec<String>>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind settings fixture");
    let port = listener.local_addr().expect("settings fixture address").port();
    let server = thread::spawn(move || {
        responses
            .into_iter()
            .map(|body| {
                let (mut stream, _) = listener.accept().expect("accept settings request");
                let mut request = Vec::new();
                let mut buffer = [0_u8; 1024];
                while !request.windows(4).any(|window| window == b"\r\n\r\n") {
                    let read = stream.read(&mut buffer).expect("read settings request");
                    if read == 0 {
                        break;
                    }
                    request.extend_from_slice(&buffer[..read]);
                }
                let response = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                );
                stream.write_all(response.as_bytes()).expect("write settings response");
                String::from_utf8(request).expect("settings request should be utf8")
            })
            .collect()
    });
    (port, server)
}

fn runtime_for_port(port: u16) -> GatewayUiRuntime {
    GatewayUiRuntime::new(
        GatewayClient::new(GatewayEndpoint::new("127.0.0.1", port)).expect("gateway client"),
        GatewaySessionSeed::new(PathBuf::from("/tmp/settings-work"))
            .with_id(Some("settings-session".to_string())),
    )
}

#[test]
fn acp_model_selection_uses_gateway_source_without_forwarding_provider_model() {
    use vw_api_acp_model::acp_model_source::AcpModelSource;

    let capabilities = r#"{
        "acp_model_source": "session_model",
        "model_source_capabilities": [
            {"status": "supported", "source": "session_model"},
            {"status": "supported", "source": "cli_local"}
        ],
        "connected": false,
        "options": []
    }"#;
    let (port, server) = spawn_json_sequence_server(vec![capabilities; 3]);
    let mut state = TuiState::default();
    let model = "openai/gpt-5.6".to_owned();
    state.session_options.model = Some(model.clone());
    state.session_capabilities.models.push(model.clone());
    state
        .session_capabilities
        .acp_backends
        .push(AcpBackendOption { key: Some("codex".to_owned()), label: "Codex".to_owned() });
    let mut app = V3GatewayApp::new(runtime_for_port(port), state);
    let mut runner = ActiveRunner::new();

    runner.execute_action(
        &mut app,
        V3Action::Select { surface: SETTINGS_ACP_SURFACE.into(), item_id: "codex".into() },
    );
    for (item_id, source, expected_model) in [
        ("acp-cli-local", AcpModelSource::CliLocal, None),
        (model.as_str(), AcpModelSource::SessionModel, Some(model.as_str())),
    ] {
        runner.execute_action(
            &mut app,
            V3Action::Select { surface: SETTINGS_MODEL_SURFACE.into(), item_id: item_id.into() },
        );
        let submission = build_prompt_submission(app.state(), "hello").expect("build prompt");
        let request = gateway_stream_request_from_state(app.state(), &submission);
        assert_eq!(request.agent.as_deref(), Some("acp-codex"));
        assert_eq!(request.acp_model_source, source);
        assert_eq!(request.model.as_deref(), expected_model);
    }
    let requests = server.join().expect("capabilities fixture");
    assert!(requests.iter().all(|request| request.starts_with("GET /v1/acp-model-source/codex ")));
}
