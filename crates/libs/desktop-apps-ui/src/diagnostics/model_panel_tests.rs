use gateway_app_diagnostics::GatewayAppDiagnosticsClient;
use super::model_state::*;
use crate::{AgentAppsContext, ProviderScope as Scope};
use {
    vw_api_transport::ProjectScope,
    vw_api_app_identity::AppId,
    vw_api_app_diagnostics::*,
};
fn context() -> ModelContext {
    ModelContext {
        scope: Scope {
            context: AgentAppsContext {
                gateway: "http://localhost:1".into(),
                tenant: Default::default(),
                project: Some("/fixture".into()),
            },
            host: "host".into(),
            app: AppId::Codex,
        },
        project: "project".into(),
        credentials_generation: 1,
        provider: "provider".into(),
    }
}
fn run(kind: DiagnosticProbeKind) -> ModelRun {
    let context = context();
    let job = DiagnosticJobIdentity {
        task_id: "job".into(),
        correlation_id: "correlation".into(),
        scope: DiagnosticScope {
            project: ProjectScope::new(
                context.scope.context.tenant.clone(),
                context.project.clone(),
            )
            .unwrap(),
            target_host_id: "host".into(),
            app_id: AppId::Codex,
        },
        kind,
    };
    ModelRun {
        context,
        model: "中文模型🧪".into(),
        kind,
        serial: 1,
        job: Some(job),
        snapshot: None,
        starting: false,
        cancelling: false,
        error: None,
        poll: None,
    }
}
fn snapshot(run: &ModelRun, state: DiagnosticJobState) -> DiagnosticJobSnapshot {
    DiagnosticJobSnapshot {
        job: run.job.clone().unwrap(),
        state,
        progress: DiagnosticProgress { completed: 1, total: Some(1) },
        results: None,
    }
}
#[tokio::test]
async fn diagnostic_desktop_cancel_keeps_terminal_state() {
    for kind in [
        DiagnosticProbeKind::Reachability,
        DiagnosticProbeKind::Model,
        DiagnosticProbeKind::Streaming,
    ] {
        let mut run = run(kind);
        assert!(run.context.matches(run.job.as_ref().unwrap(), kind));
        run.cancelling = true;
        let running = snapshot(&run, DiagnosticJobState::Running {});
        assert!(run.accept(1, running));
        assert!(
            !run.cancelling,
            "a fresh running snapshot must permit retrying unconfirmed cancellation"
        );
        let cancelled = snapshot(&run, DiagnosticJobState::Cancelled {});
        assert!(run.accept(1, cancelled));
        let late = snapshot(&run, DiagnosticJobState::Succeeded {});
        assert!(!run.accept(1, late), "late success cannot replace confirmed cancellation");
        assert!(matches!(run.snapshot.as_ref().unwrap().state, DiagnosticJobState::Cancelled {}));
        let retained = protocol_roundtrip(kind).await;
        let mut state = ModelState {
            access_generation: 1,
            open: Some(retained.context.clone()),
            model: retained.model.clone(),
            runs: vec![retained],
        };
        render_panel(&state).await;
        // All public evidence types are rendered, including failure, timing and source labels.
        let run = &mut state.runs[0];
        let report = serde_json::json!({
            "source": "controlled_fixture", "timings": [{"phase":"response_headers","elapsedMs":7}],
            "outcome": {"status":"succeeded", "evidence": match kind {
                DiagnosticProbeKind::Reachability => serde_json::json!({"httpStatus":401,"slow":false}),
                DiagnosticProbeKind::Model => serde_json::json!({"httpStatus":200,"actualModelId":"中文模型🧪"}),
                _ => serde_json::json!({"httpStatus":200,"actualModelId":"中文模型🧪","firstEventMs":3,"maxEventGapMs":5})
            }}
        });
        let evidence: DiagnosticResult = serde_json::from_value(serde_json::json!({
            "kind": match kind { DiagnosticProbeKind::Reachability => "reachability", DiagnosticProbeKind::Model => "model", _ => "streaming" },
            "providerId": "provider", "report": report,
        }).as_object().map(|object| {
            let mut object = object.clone();
            if kind != DiagnosticProbeKind::Reachability {
                object.insert("modelId".into(), "中文模型🧪".into());
                object.insert("mayIncurCost".into(), true.into());
            }
            serde_json::Value::Object(object)
        }).unwrap()).unwrap();
        let text = super::model_panel::evidence_text(&evidence);
        assert!(text.contains("受控夹具（非真实供应商）") && text.contains("7ms"));
        if kind == DiagnosticProbeKind::Reachability {
            assert!(text.contains("401") && text.contains("鉴权/模型/流式未验证"));
        }
        run.snapshot = None;
        let mut successful = snapshot(run, DiagnosticJobState::Succeeded {});
        successful.results = Some(vec![evidence.clone()]);
        let mut foreign = successful.clone();
        match foreign.results.as_mut().unwrap().first_mut().unwrap() {
            DiagnosticResult::Reachability { provider_id, .. }
            | DiagnosticResult::Model { provider_id, .. }
            | DiagnosticResult::Streaming { provider_id, .. } => *provider_id = "foreign".into(),
            _ => unreachable!(),
        }
        assert!(
            !run.accept(run.serial, foreign),
            "foreign provider evidence cannot render as this run"
        );
        assert!(run.accept(run.serial, successful));
        render_panel(&state).await;
    }
}
async fn protocol_roundtrip(kind: DiagnosticProbeKind) -> ModelRun {
    use super::model;
    use std::{
        io::{BufRead, BufReader, Read, Write},
        net::TcpListener,
        time::{Duration, Instant},
    };
    use {gateway_endpoint::{GatewayEndpoint}, {vw_gateway_client::GatewayClient}};
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let port = listener.local_addr().unwrap().port();
    let client = GatewayClient::new(
        GatewayEndpoint::new("127.0.0.1", port).with_tenant_scope(context().scope.context.tenant),
    )
    .unwrap();
    let mut run = run(kind);
    if kind == DiagnosticProbeKind::Reachability {
        run.model.clear();
    }
    let identity = run.job.clone().unwrap();
    let responses = [
        serde_json::to_string(&identity).unwrap(),
        serde_json::to_string(&snapshot(&run, DiagnosticJobState::Running {})).unwrap(),
        serde_json::to_string(&snapshot(&run, DiagnosticJobState::Cancelling {})).unwrap(),
        serde_json::to_string(&snapshot(&run, DiagnosticJobState::Cancelled {})).unwrap(),
    ];
    let server = std::thread::spawn(move || {
        let mut requests = Vec::new();
        let deadline = Instant::now() + Duration::from_secs(20);
        for (index, body) in responses.into_iter().enumerate() {
            let (mut stream, _) = loop {
                match listener.accept() {
                    Ok(connection) => break connection,
                    Err(e)
                        if e.kind() == std::io::ErrorKind::WouldBlock
                            && Instant::now() < deadline =>
                    {
                        std::thread::sleep(Duration::from_millis(5))
                    }
                    Err(e) => panic!("local fixture did not finish: {e}"),
                }
            };
            stream.set_nonblocking(false).unwrap();
            stream.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
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
                    line.to_lowercase()
                        .strip_prefix("content-length:")
                        .map(|v| v.trim().parse::<usize>().unwrap())
                })
                .unwrap_or(0);
            let mut payload = vec![0; length];
            reader.read_exact(&mut payload).unwrap();
            requests.push((headers, payload));
            let status = if index == 0 { "202 Accepted" } else { "200 OK" };
            write!(stream,"HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",body.len()).unwrap();
        }
        requests
    });
    // This is the production submit helper, through the real SDK and isolated HTTP transport.
    assert_eq!(model::submit(&client, &run, "fixture-token").await.unwrap(), identity);
    let status = GatewayAppDiagnosticsClient::new(client.transport().clone()).diagnostic_status(&run.context.query(), &identity.task_id, "fixture-token")
        .await
        .unwrap();
    assert!(run.accept(1, status));
    let status = GatewayAppDiagnosticsClient::new(client.transport().clone()).diagnostic_cancel(&run.context.query(), &identity.task_id, "fixture-token")
        .await
        .unwrap();
    assert!(run.accept(1, status));
    assert!(!run.terminal(), "cancelling does not confirm cleanup");
    // Page close aborts its poll handle and invalidates late callbacks, retaining the host job.
    let (task, handle) = iced::Task::<()>::future(std::future::pending()).abortable();
    drop(task);
    run.poll = Some(handle.clone());
    let mut state = ModelState {
        access_generation: 1,
        open: Some(run.context.clone()),
        model: run.model.clone(),
        runs: vec![run],
    };
    let late = snapshot(&state.runs[0], DiagnosticJobState::Succeeded {});
    state.close();
    assert!(handle.is_aborted());
    assert!(state.runs[0].poll.is_none());
    assert_eq!(state.runs[0].job.as_ref(), Some(&identity));
    assert!(!state.runs[0].accept(1, late));
    state.open = Some(state.runs[0].context.clone());
    let run = &mut state.runs[0];
    let final_state = GatewayAppDiagnosticsClient::new(client.transport().clone()).diagnostic_status(&run.context.query(), &identity.task_id, "fixture-token")
        .await
        .unwrap();
    assert!(run.accept(run.serial, final_state));
    assert!(run.terminal());
    let requests = server.join().unwrap();
    assert_eq!(requests.len(), 4, "one submit, status, cancel, re-entry status; no resubmission");
    let payload: serde_json::Value = serde_json::from_slice(&requests[0].1).unwrap();
    assert_eq!(payload["providerId"], "provider");
    assert_eq!(
        payload["kind"],
        match kind {
            DiagnosticProbeKind::Reachability => "reachability",
            DiagnosticProbeKind::Model => "model",
            _ => "streaming",
        }
    );
    if kind != DiagnosticProbeKind::Reachability {
        assert_eq!(payload["modelId"], "中文模型🧪");
    }
    assert!(requests[0].0.starts_with("POST /v1/agent-apps/diagnostics/jobs?"));
    assert!(requests[2].0.starts_with("POST /v1/agent-apps/diagnostics/jobs/job/cancel?"));
    for (headers, _) in &requests {
        assert!(
            headers.contains("projectId=project")
                && headers.contains("targetHostId=host")
                && headers.contains("appId=codex")
        );
        assert!(headers.contains("x-agent-apps-token: fixture-token"));
    }
    state.runs.pop().unwrap()
}

async fn render_panel(state: &ModelState) {
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
        let mut element = super::model_panel::overlay(state, iced::widget::text("应用管理").into());
        let mut tree = Tree::new(element.as_widget());
        let size = Size::new(800.0, 720.0);
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
        // Exercise the actual modal overlay as well as the root; it owns the form text.
        if let Some(mut overlay) = element.as_widget_mut().overlay(
            &mut tree,
            Layout::new(&node),
            &renderer,
            &Rectangle::with_size(size),
            iced::Vector::ZERO,
        ) {
            let overlay_node = overlay.as_overlay_mut().layout(&renderer, size);
            overlay.as_overlay().draw(
                &mut renderer,
                &theme,
                &renderer::Style { text_color: theme.palette().text },
                Layout::new(&overlay_node),
                mouse::Cursor::Unavailable,
            );
        }
        let pixels = renderer.screenshot(Size::new(800, 720), 1.0, theme.palette().background);
        assert!(!pixels.is_empty(), "Chinese and dynamic model text must shape in the real modal");
    }
}
