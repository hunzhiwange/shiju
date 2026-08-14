use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::PathBuf;
use std::thread;

use vw_config_types::skills::SkillsDirectoryProvider;
use vw_gateway_client::{GatewayClient, GatewayEndpoint};

use super::super::commands::registry::{CommandAction, CommandRegistry, CommandSource};
use super::super::completion::{CompletionCatalog, CompletionKind, sync_skill_completion_catalog};
use super::gateway::{GatewaySessionSeed, GatewayUiRuntime};
use super::integrations::{
    IntegrationAction, IntegrationItem, IntegrationKind, IntegrationLoadState, IntegrationPanel,
    IntegrationRuntime, integration_command_specs, refresh_integrations, switch_skill_provider,
    toggle_selected_integration,
};

struct RecordingIntegrations {
    result: Result<Vec<IntegrationItem>, String>,
}

#[derive(Default)]
struct ToggleRecordingIntegrations {
    calls: usize,
}

impl IntegrationRuntime for ToggleRecordingIntegrations {
    fn load(&mut self, _: IntegrationKind) -> Result<Vec<IntegrationItem>, String> {
        Ok(Vec::new())
    }

    fn set_enabled(&mut self, _: IntegrationKind, _: &str, _: bool) -> Result<(), String> {
        self.calls += 1;
        Ok(())
    }
}

impl IntegrationRuntime for RecordingIntegrations {
    fn load(&mut self, _: IntegrationKind) -> Result<Vec<IntegrationItem>, String> {
        self.result.clone()
    }
}

fn item(id: &str, detail: &str) -> IntegrationItem {
    IntegrationItem {
        id: id.to_string(),
        title: format!("Integration {id}"),
        detail: detail.to_string(),
        enabled: true,
        actions: vec![
            IntegrationAction { label: "刷新".to_string(), enabled: true, disabled_reason: None },
            IntegrationAction {
                label: "OAuth".to_string(),
                enabled: false,
                disabled_reason: Some("VibeWindow Gateway 未提供 Claude 私有 OAuth".to_string()),
            },
        ],
    }
}

#[test]
fn tui_integration_commands_show_real_capabilities_and_explicit_errors() {
    let mut panel = IntegrationPanel::new(IntegrationKind::Mcp);
    let mut runtime = RecordingIntegrations {
        result: Ok(vec![item("one", "tools=3 token=should-not-render"), item("two", "tools=1")]),
    };
    refresh_integrations(&mut panel, &mut runtime);
    assert_eq!(panel.state, IntegrationLoadState::Ready);
    assert!(panel.lines().join("\n").contains("[redacted]"));
    assert!(!panel.lines().join("\n").contains("should-not-render"));
    panel.selected = 1;

    runtime.result = Err("gateway disconnected authorization=secret".to_string());
    refresh_integrations(&mut panel, &mut runtime);
    assert_eq!(panel.items.len(), 2, "refresh errors preserve stale verified data");
    let IntegrationLoadState::Error(error) = &panel.state else {
        panic!("expected visible error");
    };
    assert!(error.contains("[redacted]"));

    runtime.result = Ok(vec![item("one", "tools=4")]);
    refresh_integrations(&mut panel, &mut runtime);
    assert_eq!(panel.selected, 0, "selection resets when selected item disappears");

    runtime.result = Ok(Vec::new());
    refresh_integrations(&mut panel, &mut runtime);
    assert_eq!(panel.state, IntegrationLoadState::Empty);
}

#[test]
fn tui_parity_golden_integration_overlays() {
    let mut panel = IntegrationPanel::new(IntegrationKind::Plugin);
    panel.finish_refresh(Ok(vec![item("formatter", "source=workspace hooks=2")]));
    assert_eq!(
        panel.lines().join("\n"),
        "Plugin · Ready\n> Integration formatter · enabled · source=workspace hooks=2"
    );
}

#[test]
fn integration_refresh_merges_partial_sources_and_registers_only_safe_dynamic_commands() {
    let skills = vec![IntegrationItem {
        id: "review-code".to_string(),
        title: "Review Code".to_string(),
        detail: "Review changes".to_string(),
        enabled: true,
        actions: Vec::new(),
    }];
    let specs = integration_command_specs(IntegrationKind::Skill, &skills);
    assert_eq!(specs.len(), 1);
    assert!(matches!(specs[0].action, CommandAction::PromptExpansion(_)));

    let registry = CommandRegistry::builtin()
        .replace_source(CommandSource::Skill, Ok(specs))
        .replace_source(CommandSource::Mcp, Err("mcp token=secret is offline".to_string()));
    assert!(registry.resolve("help").is_some(), "builtin survives partial refresh failure");
    assert!(registry.resolve("skill:review-code").is_some());
    assert!(registry.errors().iter().any(|error| error.contains("Mcp")));
    assert!(!registry.errors().join("\n").contains("secret"));
}

#[test]
fn skill_refresh_updates_completion_catalog_and_preserves_safe_stale_items_on_error() {
    let installed_action = |installed| IntegrationAction {
        label: "启用/停用".to_string(),
        enabled: installed,
        disabled_reason: (!installed).then(|| "未安装".to_string()),
    };
    let enabled = IntegrationItem {
        id: "review-code".to_string(),
        title: "Review Code".to_string(),
        detail: "Review changes".to_string(),
        enabled: true,
        actions: vec![installed_action(true)],
    };
    let disabled = IntegrationItem {
        id: "disabled".to_string(),
        title: "Disabled".to_string(),
        detail: "Disabled skill".to_string(),
        enabled: false,
        actions: vec![installed_action(true)],
    };
    let mut runtime = RecordingIntegrations { result: Ok(vec![enabled, disabled]) };
    let mut panel = IntegrationPanel::new(IntegrationKind::Skill);
    let mut catalog = CompletionCatalog::default();

    refresh_integrations(&mut panel, &mut runtime);
    sync_skill_completion_catalog(&mut catalog, &panel);
    let skills =
        catalog.items.iter().filter(|item| item.kind == CompletionKind::Skill).collect::<Vec<_>>();
    assert_eq!(skills.len(), 1);
    assert_eq!(skills[0].replacement, "$review-code");

    runtime.result = Err("gateway authorization=secret unavailable".to_string());
    refresh_integrations(&mut panel, &mut runtime);
    sync_skill_completion_catalog(&mut catalog, &panel);
    assert_eq!(catalog.items.iter().filter(|item| item.kind == CompletionKind::Skill).count(), 1);
    let error = catalog.source_errors.get(&CompletionKind::Skill).expect("safe skill error");
    assert!(error.contains("[redacted]"));
    assert!(!error.contains("secret"));
}

#[test]
fn skill_platform_switch_reloads_only_installed_skills() {
    let skills = serde_json::json!([
        {
            "id": "installed",
            "title": "Installed",
            "description": "available",
            "kind": "skill",
            "resource_count": 1,
            "installed": true,
            "enabled": true,
            "source": "project",
            "source_path": "/repo/.codex/skills/installed"
        },
        {
            "id": "catalog-only",
            "title": "Catalog only",
            "description": "not installed",
            "kind": "skill",
            "resource_count": 1,
            "installed": false,
            "enabled": true,
            "source": "builtin",
            "source_path": null
        }
    ])
    .to_string();
    let codex_skills = serde_json::json!([
        {
            "id": "codex-installed",
            "title": "Codex Installed",
            "description": "codex available",
            "kind": "skill",
            "resource_count": 1,
            "installed": true,
            "enabled": true,
            "source": "global",
            "source_path": "/home/.codex/skills/codex-installed"
        }
    ])
    .to_string();
    let (port, server) = spawn_json_server(vec![
        ("200 OK", r#"{"skills":{"directory_provider":"vibewindow"}}"#.to_string()),
        ("200 OK", skills),
        ("200 OK", "{}".to_string()),
        ("200 OK", codex_skills),
    ]);
    let mut runtime = runtime_for_port(port);
    let mut panel = IntegrationPanel::new(IntegrationKind::Skill);
    let mut catalog = CompletionCatalog::default();

    refresh_integrations(&mut panel, &mut runtime);
    assert_eq!(panel.skill_provider, SkillsDirectoryProvider::Vibewindow);
    assert_eq!(panel.items.iter().map(|item| item.id.as_str()).collect::<Vec<_>>(), ["installed"]);
    switch_skill_provider(&mut panel, &mut runtime, SkillsDirectoryProvider::Codex)
        .expect("switch to codex");
    sync_skill_completion_catalog(&mut catalog, &panel);
    let requests = server.join().expect("join skills server");

    assert!(requests[0].starts_with("GET /v1/config?directory=%2Frepo "));
    assert!(requests[1].starts_with("GET /v1/skills?project_path=%2Frepo "));
    assert!(requests[2].starts_with("PATCH /v1/global/config "));
    assert_eq!(
        request_json(&requests[2]),
        serde_json::json!({"skills": {"directory_provider": "codex"}})
    );
    assert!(requests[3].starts_with("GET /v1/skills?project_path=%2Frepo "));
    assert_eq!(panel.skill_provider, SkillsDirectoryProvider::Codex);
    assert_eq!(
        panel.items.iter().map(|item| item.id.as_str()).collect::<Vec<_>>(),
        ["codex-installed"]
    );
    assert_eq!(
        catalog
            .items
            .iter()
            .filter(|item| item.kind == CompletionKind::Skill)
            .map(|item| item.replacement.as_str())
            .collect::<Vec<_>>(),
        ["$codex-installed"]
    );

    let (failure_port, failure_server) = spawn_json_server(vec![(
        "500 Internal Server Error",
        r#"{"error":"readonly"}"#.to_string(),
    )]);
    let mut failure_runtime = runtime_for_port(failure_port);
    let previous = panel.clone();
    assert!(
        switch_skill_provider(&mut panel, &mut failure_runtime, SkillsDirectoryProvider::Claude)
            .is_err()
    );
    failure_server.join().expect("join failed switch server");
    assert_eq!(panel, previous, "failed persistence must preserve the prior snapshot");
}

#[test]
fn unsupported_skill_platform_ignores_toggle_and_keeps_installed_skills_selectable() {
    let skills = serde_json::json!([
        {
            "id": "external-installed",
            "title": "External Installed",
            "description": "available without a disabled marker",
            "kind": "skill",
            "resource_count": 1,
            "installed": true,
            "enabled": false,
            "source": "global",
            "source_path": "/home/.codex/skills/external-installed"
        }
    ])
    .to_string();
    let (port, server) = spawn_json_server(vec![
        ("200 OK", r#"{"skills":{"directory_provider":"codex"}}"#.to_string()),
        ("200 OK", skills),
    ]);
    let mut runtime = runtime_for_port(port);
    let mut panel = IntegrationPanel::new(IntegrationKind::Skill);
    let mut catalog = CompletionCatalog::default();

    refresh_integrations(&mut panel, &mut runtime);
    sync_skill_completion_catalog(&mut catalog, &panel);
    let requests = server.join().expect("join unsupported platform server");

    assert_eq!(requests.len(), 2, "loading must not emit a toggle request");
    assert!(!panel.can_toggle, "Codex does not support VibeWindow disabled markers");
    assert!(panel.items[0].actions.is_empty());
    assert!(panel.items[0].enabled);
    assert!(catalog.items.iter().any(|item| item.replacement == "$external-installed"));
    let mut toggle_runtime = ToggleRecordingIntegrations::default();
    assert!(!toggle_selected_integration(&panel, &mut toggle_runtime).expect("ignore toggle"));
    assert_eq!(toggle_runtime.calls, 0, "unsupported platforms emit no toggle request");
}

fn runtime_for_port(port: u16) -> GatewayUiRuntime {
    GatewayUiRuntime::new(
        GatewayClient::new(GatewayEndpoint::new("127.0.0.1", port)).expect("gateway client"),
        GatewaySessionSeed::new(PathBuf::from("/repo")),
    )
}

fn spawn_json_server(
    responses: Vec<(&'static str, String)>,
) -> (u16, thread::JoinHandle<Vec<String>>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind integration fixture");
    let port = listener.local_addr().expect("fixture address").port();
    let server = thread::spawn(move || {
        responses
            .into_iter()
            .map(|(status, body)| {
                let (mut stream, _) = listener.accept().expect("accept integration request");
                let request = read_request(&mut stream);
                let response = format!(
                    "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                );
                stream.write_all(response.as_bytes()).expect("write integration response");
                request
            })
            .collect()
    });
    (port, server)
}

fn read_request(stream: &mut std::net::TcpStream) -> String {
    let mut request = Vec::new();
    let mut buffer = [0_u8; 1024];
    let mut expected_len = None;
    loop {
        let read = stream.read(&mut buffer).expect("read integration request");
        if read == 0 {
            break;
        }
        request.extend_from_slice(&buffer[..read]);
        if expected_len.is_none()
            && let Some(header_end) = request.windows(4).position(|window| window == b"\r\n\r\n")
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
                .unwrap_or_default();
            expected_len = Some(header_end + 4 + content_length);
        }
        if expected_len.is_some_and(|expected| request.len() >= expected) {
            break;
        }
    }
    String::from_utf8(request).expect("integration request utf8")
}

fn request_json(request: &str) -> serde_json::Value {
    let (_, body) = request.split_once("\r\n\r\n").expect("request body");
    serde_json::from_str(body).expect("request JSON")
}
