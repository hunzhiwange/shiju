use crate::app::message::agent_apps_diagnostics::{mcp_results, *};
use serde_json::json;
use vw_gateway_client::vw_api_types::{
    agent_apps::AppId, agent_apps_diagnostics::*, mcp_management::*,
};
fn context() -> Context {
    Context {
        records: vec![],
        connections: ["first", "second"]
            .into_iter()
            .map(|id| McpCatalogRecord {
                record: McpRecordRef { id: McpRecordId(id.into()), revision: 1 },
                scope: McpCatalogScope::Tenant { scope: Default::default() },
                name: id.into(),
                description: None,
                homepage: None,
                documentation: None,
                tags: vec![],
                transport: McpTransport::Stdio,
                connection_revision: 1,
            })
            .collect(),
        gateway: "http://localhost:1".into(),
        tenant: Default::default(),
        project_path: Some("/fixture".into()),
        credentials_generation: 1,
        query: DiagnosticScopeQuery {
            project_id: "project".into(),
            target_host_id: "host".into(),
            app_id: AppId::Codex,
        },
    }
}
fn job(context: &Context) -> DiagnosticJobIdentity {
    DiagnosticJobIdentity {
        task_id: "job".into(),
        correlation_id: "correlation".into(),
        scope: DiagnosticScope {
            project: vw_gateway_client::vw_api_types::ProjectScope::new(
                context.tenant.clone(),
                context.query.project_id.clone(),
            )
            .unwrap(),
            target_host_id: "host".into(),
            app_id: AppId::Codex,
        },
        kind: DiagnosticProbeKind::Mcp,
    }
}
fn snapshot(context: &Context, status: &str) -> DiagnosticJobSnapshot {
    DiagnosticJobSnapshot {
        job: job(context),
        state: serde_json::from_value(json!({"status": status})).unwrap(),
        progress: DiagnosticProgress { completed: 1, total: Some(2) },
        results: Some(vec![DiagnosticResult::McpConnectionCheck {
            scope: McpCatalogScope::Tenant { scope: context.tenant.clone() },
            generation: 1,
            item: McpConnectionCheckItem {
                id: McpRecordId("second".into()),
                connection_revision: 1,
                outcome: McpConnectionOutcome::Connected { tool_count: None },
            },
        }]),
    }
}
#[tokio::test]
async fn diagnostic_desktop_mcp_results_keep_server_identity() {
    let context = context();
    let mut state = McpState::default();
    state.begin(
        context.clone(),
        ["first", "second"]
            .map(|id| McpRecordRef { id: McpRecordId(id.into()), revision: 1 })
            .to_vec(),
    );
    state.job = Some(job(&context));
    assert!(state.accept(
        &context,
        &context,
        state.generation,
        state.request_serial,
        snapshot(&context, "running")
    ));
    assert_eq!(state.targets[1].1, Phase::Succeeded, "MCP results must be joined by server ID");
    assert_eq!(state.targets[0].1, Phase::Handshake);
    let mut changed = context.clone();
    changed.query.target_host_id = "other-host".into();
    assert!(!state.accept(
        &changed,
        &context,
        state.generation,
        state.request_serial,
        snapshot(&context, "succeeded")
    ));
    changed = context.clone();
    changed.credentials_generation += 1;
    assert!(!state.accept(
        &changed,
        &context,
        state.generation,
        state.request_serial,
        snapshot(&context, "succeeded")
    ));
    assert!(!state.accept(
        &context,
        &context,
        state.generation + 1,
        state.request_serial,
        snapshot(&context, "succeeded")
    ));
    let mut edited = context.clone();
    edited.records.push(McpRecordRef { id: McpRecordId("first".into()), revision: 2 });
    assert!(!state.accept(
        &edited,
        &context,
        state.generation,
        state.request_serial,
        snapshot(&context, "succeeded")
    ));
    assert!(!state.accept(
        &context,
        &context,
        state.generation,
        state.request_serial + 1,
        snapshot(&context, "succeeded")
    ));
    let mut foreign = snapshot(&context, "succeeded");
    foreign.job.task_id = "foreign-job".into();
    assert!(!state.accept(&context, &context, state.generation, state.request_serial, foreign));
    state.cancelling = true;
    assert!(state.accept(
        &context,
        &context,
        state.generation,
        state.request_serial,
        snapshot(&context, "running")
    ));
    assert_eq!(state.targets[0].1, Phase::Cancelling);
    assert!(!state.terminal, "local cancellation is not cleanup confirmation");
    assert!(state.accept(
        &context,
        &context,
        state.generation,
        state.request_serial,
        snapshot(&context, "cancelled")
    ));
    assert_eq!(state.targets[0].1, Phase::Cancelled);
    assert!(state.terminal);
    assert!(!state.accept(
        &context,
        &context,
        state.generation,
        state.request_serial,
        snapshot(&context, "running")
    ));
    assert_eq!(state.targets[0].1, Phase::Cancelled, "late results cannot undo cancellation");
    // A completed job without detailed handshake evidence remains unverified.
    let mut unknown = McpState::default();
    unknown.begin(
        context.clone(),
        vec![McpRecordRef { id: McpRecordId("first".into()), revision: 1 }],
    );
    unknown.job = Some(job(&context));
    let mut no_evidence = snapshot(&context, "succeeded");
    no_evidence.results = None;
    assert!(unknown.accept(
        &context,
        &context,
        unknown.generation,
        unknown.request_serial,
        no_evidence
    ));
    assert_eq!(unknown.targets[0].1, Phase::Unverified);
    assert!(unknown.terminal);
    let mut failed = McpState::default();
    failed.begin(
        context.clone(),
        vec![McpRecordRef { id: McpRecordId("second".into()), revision: 1 }],
    );
    failed.job = Some(job(&context));
    let mut failure = snapshot(&context, "succeeded");
    if let Some(DiagnosticResult::McpConnectionCheck { item, .. }) =
        failure.results.as_mut().and_then(|r| r.first_mut())
    {
        item.outcome = McpConnectionOutcome::Failed { code: McpFailureCode::TargetUnavailable };
    }
    assert!(failed.accept(&context, &context, failed.generation, failed.request_serial, failure));
    assert_eq!(
        failed.targets[0].1,
        Phase::Failed,
        "job completion does not establish handshake success"
    );
    assert!(failed.terminal);
    state.targets[0].0.id.0 = "中文服务器🧪".into();
    render_results(&state).await;
    protocol_roundtrip(&context).await;
}

async fn render_results(state: &McpState) {
    use iced::{
        Font, Pixels, Rectangle, Renderer, Size, Theme,
        advanced::{
            Layout, Renderer as _, layout,
            renderer::{self, Headless},
            widget::Tree,
        },
        mouse,
    };
    let mut renderer =
        <Renderer as Headless>::new(Font::DEFAULT, Pixels(14.0), Some("tiny-skia")).await.unwrap();
    for theme in [Theme::Dark, Theme::Light] {
        let mut element: iced::Element<'_, ()> = mcp_results::view(state);
        let mut tree = Tree::new(element.as_widget());
        let size = Size::new(640.0, 300.0);
        let node = element.as_widget_mut().layout(
            &mut tree,
            &renderer,
            &layout::Limits::new(Size::ZERO, size),
        );
        renderer.reset(Rectangle::with_size(size));
        element.as_widget().draw(
            &tree,
            &mut renderer,
            &theme,
            &renderer::Style { text_color: theme.palette().text },
            Layout::new(&node),
            mouse::Cursor::Unavailable,
            &Rectangle::with_size(size),
        );
        let bytes = renderer.screenshot(Size::new(640, 300), 1.0, theme.palette().background);
        assert!(!bytes.is_empty(), "Chinese dynamic results must shape and render");
    }
}
async fn protocol_roundtrip(context: &Context) {
    use std::{
        io::{BufRead, BufReader, Read, Write},
        net::TcpListener,
        time::Duration,
    };
    use vw_gateway_client::{GatewayClient, GatewayEndpoint};
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let client = GatewayClient::new(
        GatewayEndpoint::new("127.0.0.1", listener.local_addr().unwrap().port())
            .with_tenant_scope(context.tenant.clone()),
    )
    .unwrap();
    let identity = job(context);
    let running = snapshot(context, "running");
    let cancelling = snapshot(context, "cancelling");
    let cancelled = snapshot(context, "cancelled");
    let responses = [
        serde_json::to_string(&identity).unwrap(),
        serde_json::to_string(&running).unwrap(),
        serde_json::to_string(&cancelling).unwrap(),
        serde_json::to_string(&cancelled).unwrap(),
        serde_json::to_string(&identity).unwrap(),
    ];
    let server = std::thread::spawn(move || {
        let mut requests = Vec::new();
        for body in responses {
            let (mut stream, _) = listener.accept().unwrap();
            stream.set_read_timeout(Some(Duration::from_secs(10))).unwrap();
            let mut reader = BufReader::new(&mut stream);
            let mut headers = String::new();
            loop {
                let mut line = String::new();
                reader.read_line(&mut line).unwrap();
                if line == "\r\n" {
                    break;
                }
                assert!(!line.is_empty());
                headers.push_str(&line);
            }
            let length = headers
                .lines()
                .find_map(|line| {
                    line.to_ascii_lowercase()
                        .strip_prefix("content-length:")
                        .map(|value| value.trim().parse::<usize>().unwrap())
                })
                .unwrap_or(0);
            let mut payload = vec![0; length];
            reader.read_exact(&mut payload).unwrap();
            requests.push((headers, payload));
            write!(stream, "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}", body.len(), body).unwrap();
        }
        requests
    });
    let ids = vec![McpRecordId("first".into()), McpRecordId("second".into())];
    assert_eq!(
        submit(client.clone(), context, ids.clone(), "fixture-token", 1).await.unwrap(),
        identity
    );
    assert_eq!(
        client.diagnostic_status(&context.query, &identity.task_id, "fixture-token").await.unwrap(),
        running
    );
    assert_eq!(
        client.diagnostic_cancel(&context.query, &identity.task_id, "fixture-token").await.unwrap(),
        cancelling
    );
    assert_eq!(
        client.diagnostic_status(&context.query, &identity.task_id, "fixture-token").await.unwrap(),
        cancelled
    );
    submit(client, context, vec![ids[0].clone()], "fixture-token", 1).await.unwrap();
    let requests = server.join().unwrap();
    assert_eq!(requests.len(), 5, "only submit, status and cancellation requests are allowed");
    assert!(requests[0].0.starts_with("POST /v1/agent-apps/mcp/checks?"));
    assert!(requests[2].0.starts_with("POST /v1/agent-apps/diagnostics/jobs/job/cancel?"));
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&requests[0].1).unwrap(),
        json!({"generation":1, "serverIds":["first", "second"]})
    );
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&requests[4].1).unwrap(),
        json!({"generation":1, "serverIds":["first"]})
    );
    assert!(
        requests.iter().all(|(headers, _)| headers.contains("x-agent-apps-token: fixture-token"))
    );
}
