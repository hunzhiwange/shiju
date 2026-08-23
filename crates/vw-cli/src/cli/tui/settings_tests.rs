use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::PathBuf;
use std::sync::Arc;
use std::thread;

use vw_gateway_client::{GatewayClient, GatewayEndpoint};
use vw_tui::{V3Action, V3ApprovalDecision};

use super::app::{ActiveRunner, RunnerControl, V3GatewayApp, gateway_stream_request_from_state};
use super::commands::{
    SETTINGS_ACCESS_SURFACE, SETTINGS_ACP_SURFACE, SETTINGS_EFFORT_SURFACE,
    SETTINGS_MODEL_SURFACE, SETTINGS_PERMISSION_SURFACE, SETTINGS_SUBAGENT_SURFACE,
};
use super::controller::build_prompt_submission;
use super::model::{
    AcpBackendOption, DelegateAgentOption, PermissionMode, ReasoningEffort,
    SessionCapabilityMatrix, UiOverlay,
};
use super::model::settings_panel::{SettingKey, SettingsPanel, SettingsValues};
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

#[test]
fn settings_are_derived_from_gateway_capabilities() {
    let provider_response = r#"{
        "all":[{
            "id":"openai","name":"OpenAI","source":"api","env":[],
            "models":{"gpt-5.6":{
                "id":"gpt-5.6","providerID":"openai",
                "api":{"id":"responses","url":"https://example.invalid","adapter":"openai-compatible"},
                "name":"GPT-5.6","capabilities":{
                    "temperature":true,"reasoning":true,"attachment":true,"toolcall":true,
                    "input":{"text":true,"audio":false,"image":true,"video":false,"pdf":true},
                    "output":{"text":true,"audio":false,"image":false,"video":false,"pdf":false},
                    "interleaved":false
                },
                "cost":{"input":0.0,"output":0.0,"cache":{"read":0.0,"write":0.0}},
                "limit":{"context":200000,"output":32000},"status":"active","release_date":"2026-08-01"
            }}
        }],
        "default":{"openai":"gpt-5.6"},"connected":["openai"]
    }"#;
    let config_response = r#"{
        "default_model":"openai/gpt-5.6",
        "autonomy":{"level":"full","workspace_only":false},
        "agents":{"researcher":{"label":"Researcher","enabled":true,"hidden":false}}
    }"#;
    let acp_catalog_response = r#"{"codex":{"command":"codex"}}"#;
    let global_config_response = r#"{"acp":{"codex":{"command":"codex"}}}"#;
    let (port, server) = spawn_json_sequence_server(vec![
        provider_response,
        config_response,
        acp_catalog_response,
        global_config_response,
    ]);

    let loaded = load_session_capabilities_blocking(&runtime_for_port(port));

    assert!(loaded.warnings.is_empty(), "{:?}", loaded.warnings);
    assert_eq!(loaded.default_model.as_deref(), Some("openai/gpt-5.6"));
    assert_eq!(loaded.matrix.models, ["openai/gpt-5.6"]);
    assert_eq!(
        loaded.matrix.efforts,
        [
            ReasoningEffort::Minimal,
            ReasoningEffort::Low,
            ReasoningEffort::Medium,
            ReasoningEffort::High,
            ReasoningEffort::XHigh,
        ]
    );
    assert_eq!(
        loaded.matrix.delegate_agents,
        [
            DelegateAgentOption::main(),
            DelegateAgentOption {
                key: Some("researcher".to_string()),
                label: "Researcher".to_string(),
            },
        ]
    );
    assert_eq!(
        loaded.matrix.acp_backends,
        [
            AcpBackendOption::default_backend(),
            AcpBackendOption {
                key: Some("codex".to_string()),
                label: "codex".to_string(),
            },
        ]
    );
    assert_eq!(
        loaded.matrix.permission_modes,
        [PermissionMode::Default, PermissionMode::Plan, PermissionMode::Bypass]
    );
    let panel = SettingsPanel::for_capabilities(SettingsValues::default(), &loaded.matrix);
    assert!(
        panel
            .rows
            .iter()
            .find(|row| row.key == SettingKey::Model)
            .is_some_and(|row| row.unavailable_reason.is_none())
    );
    assert!(
        panel
            .rows
            .iter()
            .find(|row| row.key == SettingKey::Fast)
            .is_some_and(|row| row.unavailable_reason.is_some())
    );

    let requests = server.join().expect("join settings fixture");
    assert!(requests[0].starts_with("GET /v1/provider?directory="));
    assert!(requests[1].starts_with("GET /v1/config?directory="));
    assert!(requests[2].starts_with("GET /v1/global/config/acp "));
    assert!(requests[3].starts_with("GET /v1/global/config "));
}

#[test]
fn settings_capability_failures_keep_safe_defaults() {
    let (port, server) = spawn_json_sequence_server(vec!["{}", "[]", "[]"]);

    let loaded = load_session_capabilities_blocking(&runtime_for_port(port));

    assert!(loaded.default_model.is_none());
    assert!(loaded.matrix.models.is_empty());
    assert!(loaded.matrix.efforts.is_empty());
    assert_eq!(
        loaded.matrix.permission_modes,
        [PermissionMode::Default, PermissionMode::Plan]
    );
    assert_eq!(loaded.matrix.delegate_agents, [DelegateAgentOption::main()]);
    assert_eq!(loaded.matrix.acp_backends, [AcpBackendOption::default_backend()]);
    assert!(loaded.warnings.len() >= 2);
    assert!(!super::model::SessionOptions::default().full_access_enabled());

    server.join().expect("join failed settings fixture");
}

#[test]
fn settings_change_only_the_next_request_after_confirmation() {
    let runtime = runtime_for_port(1);
    let mut state = TuiState::default();
    state.session.session_id = Some("settings-session".to_string());
    state.status.model_name = Some("openai/old".to_string());
    state.session_options.model = Some("openai/old".to_string());
    state.session_options.set_full_access(false);
    state.session_capabilities = SessionCapabilityMatrix {
        models: vec!["openai/old".to_string(), "openai/gpt-5.6".to_string()],
        efforts: vec![ReasoningEffort::Medium, ReasoningEffort::High],
        supports_thinking: true,
        supports_fast: false,
        permission_modes: vec![PermissionMode::Default, PermissionMode::Plan, PermissionMode::Bypass],
        delegate_agents: vec![DelegateAgentOption::main()],
        acp_backends: vec![
            AcpBackendOption::default_backend(),
            AcpBackendOption { key: Some("codex".to_string()), label: "codex".to_string() },
        ],
    };
    let before = build_prompt_submission(&state, "before").expect("submission before settings");
    let mut app = V3GatewayApp::new(runtime, state);
    let mut runner = ActiveRunner::new_with_editor(Arc::new(|_| Ok(None)));

    for (surface, item_id) in [
        (SETTINGS_MODEL_SURFACE, "openai/gpt-5.6"),
        (SETTINGS_EFFORT_SURFACE, "high"),
        (SETTINGS_ACP_SURFACE, "codex"),
        (SETTINGS_PERMISSION_SURFACE, "plan"),
        (SETTINGS_SUBAGENT_SURFACE, "enabled"),
    ] {
        assert_eq!(
            runner.execute_action(
                &mut app,
                V3Action::Select { surface: surface.to_string(), item_id: item_id.to_string() },
            ),
            RunnerControl::Continue
        );
    }

    assert_eq!(
        runner.execute_action(
            &mut app,
            V3Action::Select {
                surface: SETTINGS_ACCESS_SURFACE.to_string(),
                item_id: "full".to_string(),
            },
        ),
        RunnerControl::Continue
    );
    assert!(!app.state().session_options.full_access_enabled());
    let request_id = match app.state().overlays.active() {
        Some(UiOverlay::Confirm(confirm)) => confirm.request_id.clone(),
        other => panic!("expected full-access confirmation, got {other:?}"),
    };
    runner.execute_action(
        &mut app,
        V3Action::ResolveApproval {
            request_id: request_id.clone(),
            decision: V3ApprovalDecision::Cancel,
        },
    );
    assert!(!app.state().session_options.full_access_enabled());

    runner.execute_action(
        &mut app,
        V3Action::Select {
            surface: SETTINGS_ACCESS_SURFACE.to_string(),
            item_id: "full".to_string(),
        },
    );
    runner.execute_action(
        &mut app,
        V3Action::ResolveApproval {
            request_id,
            decision: V3ApprovalDecision::Allow,
        },
    );

    let before_request = gateway_stream_request_from_state(app.state(), &before);
    assert_eq!(before_request.model.as_deref(), Some("openai/old"));
    assert!(before_request.options.is_none());

    let after = build_prompt_submission(app.state(), "after").expect("submission after settings");
    let after_request = gateway_stream_request_from_state(app.state(), &after);
    assert_eq!(after_request.model.as_deref(), Some("openai/gpt-5.6"));
    assert_eq!(
        after_request.reasoning_effort,
        Some(vw_gateway_client::vw_api_types::ReasoningEffortDto::High)
    );
    assert_eq!(after_request.acp_agent.as_deref(), Some("codex"));
    assert_eq!(after_request.allowed_tools, Some(vec!["AgentTool".to_string()]));
    assert_eq!(after_request.options.as_ref().and_then(|value| value.get("full_access")), Some(&serde_json::json!(true)));
    assert!(app.state().session_options.acp_force_new_session());
}
