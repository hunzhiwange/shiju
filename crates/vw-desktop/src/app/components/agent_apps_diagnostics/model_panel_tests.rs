use crate::app::{
    message::agent_apps_diagnostics::model_state::*,
    state::{AgentAppsContext, app_providers::list::Scope},
};
use vw_gateway_client::vw_api_types::{ProjectScope, agent_apps::AppId, agent_apps_diagnostics::*};
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
    use crate::app::message::agent_apps_diagnostics::model;
    use std::{
        io::{BufRead, BufReader, Read, Write},
        net::TcpListener,
        time::{Duration, Instant},
    };
    use vw_gateway_client::{GatewayClient, GatewayEndpoint};
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
    let status = client
        .diagnostic_status(&run.context.query(), &identity.task_id, "fixture-token")
        .await
        .unwrap();
    assert!(run.accept(1, status));
    let status = client
        .diagnostic_cancel(&run.context.query(), &identity.task_id, "fixture-token")
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
    let final_state = client
        .diagnostic_status(&run.context.query(), &identity.task_id, "fixture-token")
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
#[test]
fn diagnostic_desktop_cancel_keeps_terminal_state_routes_and_resumes() {
    use crate::app::{
        App, Message,
        message::{
            SettingsMessage,
            agent_apps_diagnostics::model::{self, Event},
            app_providers::list::ListMessage,
        },
    };
    use std::time::Duration;
    use vw_gateway_client::vw_api_types::app_providers::{AppProviderSummary, catalog::*};
    crate::app::config::with_settings_gateway(|requests, responses| {
        let rt = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(1)
            .enable_all()
            .build()
            .unwrap();
        let mut app = App::new().0;
        app.project_id = Some("project".into());
        app.project_path = Some("/fixture".into());
        app.apps_catalog.host = "host".into();
        app.apps_catalog.token = "fixture-token".into();
        app.show_system_settings = true;
        app.system_settings_tab = crate::app::components::system_settings::SystemTab::Apps;
        let (_, scope) =
            crate::app::message::app_providers::list::current(&app, AppId::Codex).unwrap();
        app.apps_catalog.context = Some(scope.context.clone());
        app.app_providers.scope = Some(scope.clone());
        app.app_providers.snapshot = Some(crate::app::state::app_providers::list::Snapshot {
            catalog: ProviderCatalog {
                app_id: AppId::Codex,
                rows: vec![ProviderCatalogRow {
                    provider: AppProviderSummary {
                        id: "provider".into(),
                        app_id: AppId::Codex,
                        name: "受控供应商".into(),
                        configured: true,
                    },
                    native_member: ProviderMembership::Unknown,
                }],
                direct: ProviderSelection::Unconfigured,
                in_use: ProviderSelection::Unconfigured,
                native_read: None,
            },
            presentation: vec![],
        });
        let drive = |task: iced::Task<Message>, response: String| {
            use iced_runtime::{Action, futures::futures::StreamExt};
            let pending = rt.spawn(async move {
                let mut stream =
                    iced_runtime::task::into_stream(task).expect("real Iced network task");
                while let Some(action) = stream.next().await {
                    if let Action::Output(message) = action {
                        return message;
                    }
                }
                panic!("missing diagnostic message")
            });
            let request = requests.recv_timeout(Duration::from_secs(10)).unwrap();
            responses.send(Some(response)).unwrap();
            (
                rt.block_on(async {
                    tokio::time::timeout(Duration::from_secs(10), pending).await.unwrap().unwrap()
                }),
                request,
            )
        };
        for (index, kind) in [
            DiagnosticProbeKind::Reachability,
            DiagnosticProbeKind::Model,
            DiagnosticProbeKind::Streaming,
        ]
        .into_iter()
        .enumerate()
        {
            drop(app.update(model::message(Event::Open("provider".into()))));
            drop(app.update(model::message(Event::Model("中文模型🧪".into()))));
            assert!(requests.try_recv().is_err(), "opening and model input cannot start a probe");
            let task = app.update(model::message(Event::Start(kind)));
            let mut expected = run(kind);
            expected.context = app.apps_catalog.model_diagnostics.open.clone().unwrap();
            let job = expected.job.as_mut().unwrap();
            job.scope.project =
                ProjectScope::new(scope.context.tenant.clone(), "project".into()).unwrap();
            job.task_id = format!("job-{index}").into();
            let (reply, request) =
                drive(task, serde_json::to_string(expected.job.as_ref().unwrap()).unwrap());
            assert!(request.starts_with("POST /v1/agent-apps/diagnostics/jobs?"));
            let task = app.update(reply);
            let running = snapshot(&expected, DiagnosticJobState::Running {});
            let (reply, request) = drive(task, serde_json::to_string(&running).unwrap());
            assert!(request.starts_with("GET /v1/agent-apps/diagnostics/jobs/"));
            drop(app.update(reply));
            let task = app.update(model::message(Event::Cancel(index)));
            if kind == DiagnosticProbeKind::Model {
                // Close before the cancellation task is even polled by the runtime.
                // The remote POST must still be sent and allowed to finish.
                drop(app.update(model::message(Event::Close)));
            }
            let cancelling = snapshot(&expected, DiagnosticJobState::Cancelling {});
            let (reply, request) = drive(task, serde_json::to_string(&cancelling).unwrap());
            assert!(request.contains(&format!("/jobs/job-{index}/cancel?")));
            drop(app.update(reply));
            assert!(!app.apps_catalog.model_diagnostics.runs[index].terminal());
            // The actual navigation hook aborts polling and preserves accepted jobs.
            if kind != DiagnosticProbeKind::Model {
                let handle = app.apps_catalog.model_diagnostics.runs[index].poll.clone().unwrap();
                drop(app.update(Message::Settings(SettingsMessage::SystemTabSelected(
                    crate::app::components::system_settings::SystemTab::General,
                ))));
                assert!(handle.is_aborted());
            }
            assert!(app.apps_catalog.model_diagnostics.open.is_none());
            app.system_settings_tab = crate::app::components::system_settings::SystemTab::Apps;
            app.apps_catalog.begin(scope.context.clone());
            let task = app.update(model::message(Event::Open("provider".into())));
            let cancelled = snapshot(&expected, DiagnosticJobState::Cancelled {});
            let (reply, request) = drive(task, serde_json::to_string(&cancelled).unwrap());
            assert!(request.starts_with("GET "), "re-entry must query the retained task");
            drop(app.update(reply));
            let serial = app.apps_catalog.model_diagnostics.runs[index].serial;
            let late = snapshot(&expected, DiagnosticJobState::Succeeded {});
            drop(app.update(Message::Settings(SettingsMessage::AppProviders(
                ListMessage::ModelDiagnostics(Event::Snapshot(index, serial, Ok(late))),
            ))));
            assert!(matches!(
                app.apps_catalog.model_diagnostics.runs[index].snapshot.as_ref().unwrap().state,
                DiagnosticJobState::Cancelled {}
            ));
        }
        assert!(requests.try_recv().is_err(), "no automatic resubmit or extra probe requests");
    });
}
