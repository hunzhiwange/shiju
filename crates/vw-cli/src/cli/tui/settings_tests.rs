#![allow(unused_imports, dead_code)]

use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::PathBuf;
use std::sync::Arc;
use std::thread;

use vw_gateway_client::{GatewayClient, GatewayEndpoint};
use vw_tui::{V3Action, V3ApprovalDecision};

use super::app::{ActiveRunner, RunnerControl, V3GatewayApp, gateway_stream_request_from_state};
use super::commands::{
    SETTINGS_ACCESS_SURFACE, SETTINGS_ACP_SURFACE, SETTINGS_EFFORT_SURFACE, SETTINGS_MODEL_SURFACE,
    SETTINGS_PERMISSION_SURFACE, SETTINGS_SUBAGENT_SURFACE,
};
use super::controller::build_prompt_submission;
use super::model::settings_panel::{SettingKey, SettingsPanel, SettingsValues};
use super::model::{
    AcpBackendOption, DelegateAgentOption, PermissionMode, ReasoningEffort,
    SessionCapabilityMatrix, UiOverlay,
};
use super::runtime::capabilities::load_session_capabilities_blocking;
use super::runtime::gateway::{GatewaySessionSeed, GatewayUiRuntime};
use super::state::TuiState;

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
