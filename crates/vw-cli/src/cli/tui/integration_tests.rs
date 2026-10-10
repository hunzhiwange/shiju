use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::PathBuf;
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

use vw_gateway_client::{GatewayClient, GatewayEndpoint};
use vw_tui::{V3Action, V3ApprovalDecision};

use super::app::{ActiveRunner, V3GatewayApp};
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
            let path = request
                .lines()
                .next()
                .unwrap_or_default()
                .split_whitespace()
                .nth(1)
                .unwrap_or_default();
            paths.push(path.to_string());
            let body = match path {
                path if path.starts_with("/v1/skills") => {
                    r#"[
                    {"id":"review","title":"Project Review","description":"project copy","kind":"local","resource_count":1,"installed":true,"enabled":true,"source":"project"},
                    {"id":"review","title":"Built-in Review","description":"built-in copy","kind":"built-in","resource_count":1,"installed":true,"enabled":true,"source":"built-in"}
                ]"#
                }
                "/v1/plugins" => {
                    r#"[{"id":"demo","display_name":"Demo","version":"1.0","description":"plugin","enabled":true,"component_count":2}]"#
                }
                "/v1/mcp/inspect" => {
                    r#"{"servers":[{"name":"docs","transport":"http","enabled":true,"required":false,"connection_status":"connected","tool_count":1,"resource_count":2}]}"#
                }
                path if path.starts_with("/v1/memory") => {
                    r#"{"entries":[{"key":"project-note","title":"Note","content":"safe preview","scope":"project"}]}"#
                }
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

#[test]
fn hook_catalog_failure_isolated_from_existing_integrations() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("hook failure fixture");
    let port = listener.local_addr().expect("hook failure address").port();
    let server = thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept failed hooks request");
        let request = read_request(&mut stream);
        let body = r#"{"error":"token=must-not-leak"}"#;
        write!(
            stream,
            "HTTP/1.1 500 Internal Server Error\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
            body.len()
        )
        .expect("write hook failure");
        request
    });
    let temp = tempfile::tempdir().expect("workspace");
    let existing = plugin_item("existing-plugin");
    let mut state = TuiState::default();
    state.session.session_id = Some("integration-session".to_string());
    state.integration_catalog = vec![existing.clone()];
    let mut app = V3GatewayApp::new(runtime(port, temp.path().to_path_buf()), state);
    let mut runner = ActiveRunner::new_with_editor(std::sync::Arc::new(|_| Ok(None)));

    runner.execute_action(
        &mut app,
        V3Action::Submit { text: "/hooks".to_string(), attachments: Vec::new() },
    );
    server.join().expect("join hook failure fixture");
    runner.drain(&mut app);

    assert_eq!(app.state().integration_catalog, vec![existing]);
    assert!(app.state().messages.iter().any(|message| match message {
        super::model::UiMessage::Error(error) => {
            error.message.contains("failed to load Hook catalog")
                && !error.message.contains("must-not-leak")
        }
        _ => false,
    }));
}

#[test]
fn hook_mutations_require_catalog_membership_and_confirmation() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("hook mutation fixture");
    let port = listener.local_addr().expect("hook mutation address").port();
    let (request_sender, request_receiver) = mpsc::channel();
    let server = thread::spawn(move || {
        let mut requests = Vec::new();
        for response in [
            r#"[{"plugin_id":"safe-plugin","event":"session.start","command":"ignored","args":[],"working_dir":"/workspace","mode":"blocking","hook_hash":"abc123","trusted":false}]"#,
            r#"{"plugin_id":"safe-plugin","hook_hash":"abc123","trusted":true}"#,
            r#"[{"plugin_id":"safe-plugin","event":"session.start","command":"ignored","args":[],"working_dir":"/workspace","mode":"blocking","hook_hash":"abc123","trusted":true}]"#,
            r#"[]"#,
        ] {
            let (mut stream, _) = listener.accept().expect("accept hook mutation request");
            let request = read_request(&mut stream);
            requests.push(request.clone());
            request_sender.send(request).expect("record hook mutation request");
            write_json(&mut stream, response);
        }
        requests
    });
    let temp = tempfile::tempdir().expect("workspace");
    let mut app = V3GatewayApp::new(runtime(port, temp.path().to_path_buf()), TuiState::default());
    let mut runner = ActiveRunner::new_with_editor(std::sync::Arc::new(|_| Ok(None)));

    runner.execute_action(&mut app, submit("/hooks"));
    request_receiver.recv_timeout(Duration::from_secs(2)).expect("initial Hook catalog");
    drain_until(&mut runner, &mut app, |runner, _| {
        runner.hook_catalog_contains("safe-plugin", "abc123", false)
    });

    runner.execute_action(&mut app, submit("/hooks trust outside abc123"));
    assert!(!matches!(app.state().overlays.active(), Some(UiOverlay::Confirm(_))));

    runner.execute_action(&mut app, submit("/hooks trust safe-plugin abc123"));
    let UiOverlay::Confirm(cancel_confirmation) =
        app.state().overlays.active().expect("Hook confirmation")
    else {
        panic!("expected Hook confirmation");
    };
    assert!(cancel_confirmation.body.contains("safe-plugin"));
    assert!(cancel_confirmation.body.contains("abc123"));
    assert_eq!(
        cancel_confirmation.intent,
        UiConfirmIntent::HookMutation {
            plugin_id: "safe-plugin".to_string(),
            hook_hash: "abc123".to_string(),
            trusted: true,
        }
    );
    let cancel_request_id = cancel_confirmation.request_id.clone();
    runner.execute_action(
        &mut app,
        V3Action::ResolveApproval {
            request_id: cancel_request_id,
            decision: V3ApprovalDecision::Cancel,
        },
    );
    assert!(request_receiver.recv_timeout(Duration::from_millis(100)).is_err());

    runner.execute_action(&mut app, submit("/hooks trust safe-plugin abc123"));
    let UiOverlay::Confirm(confirm) = app.state().overlays.active().expect("Hook confirmation")
    else {
        panic!("expected Hook confirmation");
    };
    let confirm_request_id = confirm.request_id.clone();
    runner.execute_action(
        &mut app,
        V3Action::ResolveApproval {
            request_id: confirm_request_id,
            decision: V3ApprovalDecision::Allow,
        },
    );
    for _ in 0..3 {
        request_receiver.recv_timeout(Duration::from_secs(2)).expect("mutation refresh request");
    }
    drain_until(&mut runner, &mut app, |runner, _| {
        runner.hook_catalog_contains("safe-plugin", "abc123", true)
    });

    let requests = server.join().expect("join hook mutation fixture");
    assert!(requests[0].starts_with("GET /v1/hooks "));
    assert!(requests[1].starts_with("POST /v1/hooks/trust "));
    assert!(requests[1].contains(r#"{"plugin_id":"safe-plugin","hook_hash":"abc123"}"#));
    assert!(requests[2].starts_with("GET /v1/hooks "));
    assert!(requests[3].starts_with("GET /v1/hooks/audit "));
}

#[test]
fn hook_mutation_refreshes_only_after_gateway_success() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("failed hook mutation fixture");
    let port = listener.local_addr().expect("failed hook mutation address").port();
    let (request_sender, request_receiver) = mpsc::channel();
    let server = thread::spawn(move || {
        let (mut catalog_stream, _) = listener.accept().expect("accept Hook catalog");
        let catalog_request = read_request(&mut catalog_stream);
        request_sender.send(catalog_request.clone()).expect("record Hook catalog");
        write_json(
            &mut catalog_stream,
            r#"[{"plugin_id":"safe-plugin","event":"session.start","command":"ignored","args":[],"working_dir":"/workspace","mode":"blocking","hook_hash":"abc123","trusted":true}]"#,
        );

        let (mut mutation_stream, _) = listener.accept().expect("accept failed Hook mutation");
        let mutation_request = read_request(&mut mutation_stream);
        request_sender.send(mutation_request.clone()).expect("record failed Hook mutation");
        let body = r#"{"error":"token=must-not-leak"}"#;
        write!(
            mutation_stream,
            "HTTP/1.1 500 Internal Server Error\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
            body.len()
        )
        .expect("write failed Hook mutation");
        (catalog_request, mutation_request)
    });
    let temp = tempfile::tempdir().expect("workspace");
    let mut app = V3GatewayApp::new(runtime(port, temp.path().to_path_buf()), TuiState::default());
    let mut runner = ActiveRunner::new_with_editor(std::sync::Arc::new(|_| Ok(None)));

    runner.execute_action(&mut app, submit("/hooks"));
    request_receiver.recv_timeout(Duration::from_secs(2)).expect("initial Hook catalog");
    drain_until(&mut runner, &mut app, |runner, _| {
        runner.hook_catalog_contains("safe-plugin", "abc123", true)
    });
    runner.execute_action(&mut app, submit("/hooks revoke safe-plugin abc123"));
    let UiOverlay::Confirm(confirm) = app.state().overlays.active().expect("Hook confirmation")
    else {
        panic!("expected Hook confirmation");
    };
    let confirm_request_id = confirm.request_id.clone();
    runner.execute_action(
        &mut app,
        V3Action::ResolveApproval {
            request_id: confirm_request_id,
            decision: V3ApprovalDecision::Allow,
        },
    );
    request_receiver.recv_timeout(Duration::from_secs(2)).expect("failed Hook POST");
    drain_until(&mut runner, &mut app, |runner, app| {
        !runner.hook_mutation_in_flight()
            && app.state().messages.iter().any(|message| match message {
                super::model::UiMessage::Error(error) => {
                    error.message.contains("failed to revoke Hook")
                        && !error.message.contains("must-not-leak")
                }
                _ => false,
            })
    });
    assert!(runner.hook_catalog_contains("safe-plugin", "abc123", true));
    assert!(request_receiver.recv_timeout(Duration::from_millis(100)).is_err());

    let (catalog_request, mutation_request) = server.join().expect("join failed Hook fixture");
    assert!(catalog_request.starts_with("GET /v1/hooks "));
    assert!(mutation_request.starts_with("POST /v1/hooks/revoke "));
    assert!(mutation_request.contains(r#"{"plugin_id":"safe-plugin","hook_hash":"abc123"}"#));
}

#[test]
fn memory_search_rejects_stale_results() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("memory search fixture");
    let port = listener.local_addr().expect("memory search address").port();
    let (request_sender, request_receiver) = mpsc::channel();
    let server = thread::spawn(move || {
        let mut workers = Vec::new();
        for _ in 0..2 {
            let (mut stream, _) = listener.accept().expect("accept memory query");
            let request = read_request(&mut stream);
            request_sender.send(request.clone()).expect("record memory query");
            workers.push(thread::spawn(move || {
                let path = request
                    .lines()
                    .next()
                    .unwrap_or_default()
                    .split_whitespace()
                    .nth(1)
                    .unwrap_or_default();
                let (delay, body) = if path.contains("rust+slow") {
                    (
                        Duration::from_millis(150),
                        r#"{"entries":[{"key":"stale","title":"Stale","content":"old result","scope":"project"}]}"#,
                    )
                } else {
                    (
                        Duration::ZERO,
                        r#"{"entries":[{"key":"fresh","title":"Fresh","content":"new\nresult","scope":"project"}]}"#,
                    )
                };
                thread::sleep(delay);
                write_json(&mut stream, body);
                request
            }));
        }
        workers
            .into_iter()
            .map(|worker| worker.join().expect("join memory response"))
            .collect::<Vec<_>>()
    });
    let temp = tempfile::tempdir().expect("workspace");
    let plugin = plugin_item("existing-plugin");
    let mut state = TuiState::default();
    state.session.session_id = Some("integration-session".to_string());
    state.integration_catalog = vec![plugin.clone(), memory_item("startup", "startup result")];
    let mut app = V3GatewayApp::new(runtime(port, temp.path().to_path_buf()), state);
    let mut runner = ActiveRunner::new_with_editor(std::sync::Arc::new(|_| Ok(None)));

    runner.execute_action(&mut app, submit("/memory rust slow"));
    runner.execute_action(&mut app, submit("/memory fresh async"));
    for _ in 0..2 {
        request_receiver.recv_timeout(Duration::from_secs(2)).expect("memory query request");
    }
    drain_until(&mut runner, &mut app, |_, app| {
        app.state()
            .integration_catalog
            .iter()
            .any(|item| item.kind == IntegrationKind::Memory && item.id == "fresh")
    });
    thread::sleep(Duration::from_millis(200));
    runner.drain(&mut app);

    assert_eq!(
        app.state()
            .integration_catalog
            .iter()
            .filter(|item| item.kind == IntegrationKind::Memory)
            .map(|item| (item.id.as_str(), item.detail.as_str()))
            .collect::<Vec<_>>(),
        vec![("fresh", "new result")],
        "the stale response must not replace the latest sanitized result"
    );
    assert!(app.state().integration_catalog.contains(&plugin));
    let requests = server.join().expect("join memory fixture");
    assert!(
        requests.iter().any(|request| { request.starts_with("GET /v1/memory?query=rust+slow ") })
    );
    assert!(
        requests.iter().any(|request| { request.starts_with("GET /v1/memory?query=fresh+async ") })
    );
}

#[test]
fn memory_search_failure_preserves_existing_integrations() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("memory failure fixture");
    let port = listener.local_addr().expect("memory failure address").port();
    let server = thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept failed memory query");
        let request = read_request(&mut stream);
        let body = r#"{"error":"token=must-not-leak"}"#;
        write!(
            stream,
            "HTTP/1.1 500 Internal Server Error\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
            body.len()
        )
        .expect("write memory failure");
        request
    });
    let temp = tempfile::tempdir().expect("workspace");
    let original = vec![plugin_item("existing-plugin"), memory_item("startup", "startup result")];
    let mut state = TuiState::default();
    state.session.session_id = Some("integration-session".to_string());
    state.integration_catalog.clone_from(&original);
    let mut app = V3GatewayApp::new(runtime(port, temp.path().to_path_buf()), state);
    let mut runner = ActiveRunner::new_with_editor(std::sync::Arc::new(|_| Ok(None)));

    runner.execute_action(&mut app, submit("/memory unavailable"));
    let request = server.join().expect("join memory failure fixture");
    drain_until(&mut runner, &mut app, |_, app| {
        app.state().messages.iter().any(|message| match message {
            super::model::UiMessage::Error(error) => {
                error.message.contains("failed to query memory")
            }
            _ => false,
        })
    });

    assert!(request.starts_with("GET /v1/memory?query=unavailable "));
    assert_eq!(app.state().integration_catalog, original);
    assert!(app.state().messages.iter().all(|message| match message {
        super::model::UiMessage::Error(error) => !error.message.contains("must-not-leak"),
        _ => true,
    }));
}

#[test]
fn memory_mutations_confirm_overwrite_and_delete() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("memory mutation fixture");
    let port = listener.local_addr().expect("memory mutation address").port();
    let (request_sender, request_receiver) = mpsc::channel();
    let server = thread::spawn(move || {
        let mut requests = Vec::new();
        for index in 0..5 {
            let (mut stream, _) = listener.accept().expect("accept memory mutation request");
            let request = read_request(&mut stream);
            request_sender.send(request.clone()).expect("record memory mutation request");
            requests.push(request);
            let body = match index {
                0 => {
                    r#"{"entries":[{"key":"owned/key","title":"Owned","content":"old value","scope":"project"}]}"#
                }
                1 | 3 => r#"{}"#,
                2 => {
                    r#"{"entries":[{"key":"owned/key","title":"Owned","content":"new value","scope":"project"}]}"#
                }
                4 => r#"{"entries":[]}"#,
                _ => unreachable!(),
            };
            write_json(&mut stream, body);
        }
        requests
    });
    let temp = tempfile::tempdir().expect("workspace");
    let mut app = V3GatewayApp::new(runtime(port, temp.path().to_path_buf()), TuiState::default());
    let mut runner = ActiveRunner::new_with_editor(std::sync::Arc::new(|_| Ok(None)));

    runner.execute_action(&mut app, submit("/memory owned"));
    request_receiver.recv_timeout(Duration::from_secs(2)).expect("initial memory query");
    drain_until(&mut runner, &mut app, |_, app| {
        app.state()
            .integration_catalog
            .iter()
            .any(|item| item.kind == IntegrationKind::Memory && item.id == "owned/key")
    });

    const SECRET_CONTENT: &str = "sensitive-memory-content";
    runner.execute_action(
        &mut app,
        submit(format!("/memory remember owned/key {SECRET_CONTENT}").as_str()),
    );
    let UiOverlay::Confirm(store) = app.state().overlays.active().expect("store confirmation")
    else {
        panic!("expected typed memory store confirmation");
    };
    assert!(store.destructive, "store may overwrite an unseen server value");
    assert!(!store.body.contains(SECRET_CONTENT), "confirmation must not repeat memory content");
    assert_eq!(
        store.intent,
        UiConfirmIntent::MemoryStore {
            key: "owned/key".to_string(),
            content: SECRET_CONTENT.to_string(),
        }
    );
    let store_request_id = store.request_id.clone();
    runner.execute_action(
        &mut app,
        V3Action::ResolveApproval {
            request_id: store_request_id,
            decision: V3ApprovalDecision::Cancel,
        },
    );
    assert!(request_receiver.recv_timeout(Duration::from_millis(100)).is_err());

    runner.execute_action(
        &mut app,
        submit(format!("/memory remember owned/key {SECRET_CONTENT}").as_str()),
    );
    let UiOverlay::Confirm(store) = app.state().overlays.active().expect("store confirmation")
    else {
        panic!("expected typed memory store confirmation");
    };
    let store_request_id = store.request_id.clone();
    runner.execute_action(
        &mut app,
        V3Action::ResolveApproval {
            request_id: store_request_id,
            decision: V3ApprovalDecision::Allow,
        },
    );
    for _ in 0..2 {
        request_receiver.recv_timeout(Duration::from_secs(2)).expect("store and refresh request");
    }
    drain_until(&mut runner, &mut app, |runner, app| {
        !runner.memory_mutation_in_flight()
            && app
                .state()
                .integration_catalog
                .iter()
                .any(|item| item.kind == IntegrationKind::Memory && item.detail == "new value")
    });

    runner.execute_action(&mut app, submit("/memory forget unknown"));
    assert!(!matches!(app.state().overlays.active(), Some(UiOverlay::Confirm(_))));
    assert!(request_receiver.recv_timeout(Duration::from_millis(100)).is_err());

    runner.execute_action(&mut app, submit("/memory forget owned/key"));
    let UiOverlay::Confirm(delete) = app.state().overlays.active().expect("delete confirmation")
    else {
        panic!("expected typed memory delete confirmation");
    };
    assert!(delete.destructive);
    assert_eq!(delete.intent, UiConfirmIntent::MemoryDelete { key: "owned/key".to_string() });
    let delete_request_id = delete.request_id.clone();
    runner.execute_action(
        &mut app,
        V3Action::ResolveApproval {
            request_id: delete_request_id,
            decision: V3ApprovalDecision::Cancel,
        },
    );
    assert!(request_receiver.recv_timeout(Duration::from_millis(100)).is_err());

    runner.execute_action(&mut app, submit("/memory forget owned/key"));
    let UiOverlay::Confirm(delete) = app.state().overlays.active().expect("delete confirmation")
    else {
        panic!("expected typed memory delete confirmation");
    };
    let delete_request_id = delete.request_id.clone();
    runner.execute_action(
        &mut app,
        V3Action::ResolveApproval {
            request_id: delete_request_id,
            decision: V3ApprovalDecision::Allow,
        },
    );
    for _ in 0..2 {
        request_receiver.recv_timeout(Duration::from_secs(2)).expect("delete and refresh request");
    }
    drain_until(&mut runner, &mut app, |runner, app| {
        !runner.memory_mutation_in_flight()
            && app
                .state()
                .integration_catalog
                .iter()
                .all(|item| item.kind != IntegrationKind::Memory)
    });

    let requests = server.join().expect("join memory mutation fixture");
    assert!(requests[0].starts_with("GET /v1/memory?query=owned "));
    assert!(requests[1].starts_with("POST /v1/memory "));
    assert_eq!(
        request_body(&requests[1]),
        r#"{"category":"core","content":"sensitive-memory-content","key":"owned/key"}"#
    );
    assert!(requests[2].starts_with("GET /v1/memory?query=owned "));
    assert!(requests[3].starts_with("DELETE /v1/memory/owned%2Fkey "));
    assert!(requests[4].starts_with("GET /v1/memory?query=owned "));
    assert!(app.state().messages.iter().all(|message| match message {
        super::model::UiMessage::Error(error) => !error.message.contains(SECRET_CONTENT),
        _ => true,
    }));
}

#[test]
fn memory_mutation_failure_preserves_snapshot_and_redacts_content() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("failed memory mutation fixture");
    let port = listener.local_addr().expect("failed memory mutation address").port();
    let server = thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept failed memory store");
        let request = read_request(&mut stream);
        let body = r#"{"error":"server-secret-must-not-leak"}"#;
        write!(
            stream,
            "HTTP/1.1 500 Internal Server Error\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
            body.len()
        )
        .expect("write failed memory store");
        request
    });
    let temp = tempfile::tempdir().expect("workspace");
    let original = vec![memory_item("owned", "original value")];
    let mut state = TuiState::default();
    state.integration_catalog.clone_from(&original);
    let mut app = V3GatewayApp::new(runtime(port, temp.path().to_path_buf()), state);
    let mut runner = ActiveRunner::new_with_editor(std::sync::Arc::new(|_| Ok(None)));

    const SECRET_CONTENT: &str = "client-secret-memory-content";
    runner.execute_action(
        &mut app,
        submit(format!("/memory remember new-key {SECRET_CONTENT}").as_str()),
    );
    let UiOverlay::Confirm(confirm) = app.state().overlays.active().expect("store confirmation")
    else {
        panic!("expected memory store confirmation");
    };
    let confirm_request_id = confirm.request_id.clone();
    runner.execute_action(
        &mut app,
        V3Action::ResolveApproval {
            request_id: confirm_request_id,
            decision: V3ApprovalDecision::Allow,
        },
    );
    drain_until(&mut runner, &mut app, |runner, app| {
        !runner.memory_mutation_in_flight()
            && app.state().messages.iter().any(|message| match message {
                super::model::UiMessage::Error(error) => {
                    error.message.contains("failed to store memory")
                }
                _ => false,
            })
    });

    let request = server.join().expect("join failed memory mutation fixture");
    assert!(request.starts_with("POST /v1/memory "));
    assert_eq!(app.state().integration_catalog, original);
    assert!(app.state().messages.iter().all(|message| match message {
        super::model::UiMessage::Error(error) => {
            !error.message.contains(SECRET_CONTENT)
                && !error.message.contains("server-secret-must-not-leak")
        }
        _ => true,
    }));
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

fn memory_item(id: &str, detail: &str) -> IntegrationItem {
    IntegrationItem {
        kind: IntegrationKind::Memory,
        id: id.to_string(),
        title: id.to_string(),
        detail: detail.to_string(),
        source: "gateway:memory".to_string(),
        enabled: true,
        can_toggle: false,
        can_delete: false,
        prompt_prefix: None,
        actions: Vec::new(),
    }
}

fn submit(text: &str) -> V3Action {
    V3Action::Submit { text: text.to_string(), attachments: Vec::new() }
}

fn drain_until(
    runner: &mut ActiveRunner,
    app: &mut V3GatewayApp,
    done: impl Fn(&ActiveRunner, &V3GatewayApp) -> bool,
) {
    let deadline = Instant::now() + Duration::from_secs(2);
    while !done(runner, app) {
        assert!(Instant::now() < deadline, "timed out draining Hook operation");
        runner.drain(app);
        thread::sleep(Duration::from_millis(10));
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
                .find_map(|line| {
                    line.to_ascii_lowercase()
                        .strip_prefix("content-length:")
                        .map(str::trim)
                        .and_then(|value| value.parse::<usize>().ok())
                })
                .unwrap_or_default();
            if bytes.len() >= header_end + 4 + content_length {
                break;
            }
        }
    }
    String::from_utf8(bytes).expect("utf8 request")
}

fn request_body(request: &str) -> &str {
    request.split_once("\r\n\r\n").map_or("", |(_, body)| body)
}

fn write_json(stream: &mut std::net::TcpStream, body: &str) {
    let response = format!(
        "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
        body.len()
    );
    stream.write_all(response.as_bytes()).expect("write response");
}
