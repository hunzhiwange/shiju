use crate::app::{
    App,
    message::agent_apps_diagnostics::endpoints::{self, Event},
};
use vw_gateway_client::vw_api_types::agent_apps_diagnostics::*;
use vw_gateway_client::vw_api_types::{
    agent_apps::AppId,
    app_providers::{basic::ProviderBasic, claude::ClaudeFormKind},
};
#[test]
fn diagnostic_desktop_endpoint_selection_requires_save() {
    crate::app::config::with_settings_gateway(|_requests, _responses| {
        let mut app = App::new().0;
        app.project_id = Some("project".into());
        app.project_path = Some("/fixture".into());
        app.apps_catalog.host = "host".into();
        app.apps_catalog.token = "fixture-token".into();
        app.show_system_settings = true;
        app.system_settings_tab = crate::app::components::system_settings::SystemTab::Apps;
        let (_, scope) =
            crate::app::message::app_providers::list::current(&app, AppId::Claude).unwrap();
        app.apps_catalog.context = Some(scope.context.clone());
        app.app_providers.scope = Some(scope);
        let mut form = crate::app::state::app_providers::edit::EditForm::new(
            crate::app::state::app_providers::edit::Mode::Update,
            "provider".into(),
            ProviderBasic {
                name: "中文供应商".into(),
                category: None,
                notes: None,
                website_url: None,
            },
            None,
        );
        form.claude = Some(crate::app::state::app_providers::claude::Draft::from_view(
            ClaudeFormKind::ThirdParty {
                base_url: "https://saved.example".into(),
                api_key_header: false,
            },
            Some("model".into()),
        ));
        app.app_providers.form = Some(form);
        // The first assertion was observed failing against the unimplemented production handler.
        let opening = app.update(endpoints::message(Event::Open));
        assert!(
            app.app_providers.endpoint_speed.is_some(),
            "endpoint speed selection must open a draft without saving provider configuration"
        );
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(1)
            .enable_all()
            .build()
            .unwrap();
        let drive = |task: iced::Task<crate::app::Message>, response: serde_json::Value| {
            use iced_runtime::{Action, futures::futures::StreamExt};
            let pending = runtime.spawn(async move {
                let mut stream =
                    iced_runtime::task::into_stream(task).expect("production Iced request");
                while let Some(action) = stream.next().await {
                    if let Action::Output(message) = action {
                        return message;
                    }
                }
                panic!("missing network reply")
            });
            let request = _requests.recv_timeout(std::time::Duration::from_secs(15)).unwrap();
            _responses.send(Some(response.to_string())).unwrap();
            let reply = runtime.block_on(async {
                tokio::time::timeout(std::time::Duration::from_secs(15), pending)
                    .await
                    .unwrap()
                    .unwrap()
            });
            (reply, request)
        };
        let candidates = vec![
            DiagnosticEndpointCandidate {
                id: DiagnosticEndpointId("slow".into()),
                display_url: "https://slow.example".into(),
                sources: vec![DiagnosticEndpointSource::SavedConfig],
            },
            DiagnosticEndpointCandidate {
                id: DiagnosticEndpointId("fast".into()),
                display_url: format!("https://fast.example/中文路径/{}", "long-path/".repeat(30)),
                sources: vec![DiagnosticEndpointSource::Preset, DiagnosticEndpointSource::Custom],
            },
            DiagnosticEndpointCandidate {
                id: DiagnosticEndpointId("failed".into()),
                display_url: "https://failed.example".into(),
                sources: vec![DiagnosticEndpointSource::Custom],
            },
        ];
        let task = opening;
        let (reply, request) = drive(
            task,
            serde_json::to_value(DiagnosticEndpointCandidatesResponse {
                candidates: candidates.clone(),
            })
            .unwrap(),
        );
        assert!(request.starts_with("POST /v1/agent-apps/diagnostics/endpoint-candidates/query?"));
        assert!(!request.lines().next().unwrap().contains("https"));
        let candidate_body: serde_json::Value =
            serde_json::from_str(request.split_once("\r\n\r\n").unwrap().1).unwrap();
        assert_eq!(candidate_body["draftUrls"], serde_json::json!(["https://saved.example"]));
        drop(app.update(reply));
        let state = app.app_providers.endpoint_speed.as_ref().unwrap();
        let context = state.context.clone();
        let generation = state.generation;
        let transport = state.transport.clone();
        let job = DiagnosticJobIdentity {
            task_id: "speed-job".into(),
            correlation_id: "controlled".into(),
            scope: DiagnosticScope {
                project: vw_gateway_client::vw_api_types::ProjectScope::new(
                    context.scope.context.tenant.clone(),
                    context.project.clone(),
                )
                .unwrap(),
                target_host_id: context.scope.host.clone(),
                app_id: context.scope.app,
            },
            kind: DiagnosticProbeKind::Speed,
        };
        let task = app.update(endpoints::message(Event::Start));
        let (reply, request) = drive(task, serde_json::to_value(&job).unwrap());
        assert!(request.starts_with("POST /v1/agent-apps/diagnostics/jobs?"));
        let body: serde_json::Value =
            serde_json::from_str(request.split_once("\r\n\r\n").unwrap().1).unwrap();
        assert_eq!(body["endpointIds"], serde_json::json!(["slow", "fast", "failed"]));
        assert_eq!(body["providerId"], "provider");
        assert!(request.contains("x-agent-apps-token: fixture-token"));
        let task = app.update(reply);
        let report = |latency| DiagnosticReport {
            source: DiagnosticEvidenceSource::ControlledFixture,
            timings: vec![DiagnosticTiming {
                phase: DiagnosticPhase::ResponseHeaders,
                elapsed_ms: latency,
            }],
            outcome: DiagnosticOutcome::Succeeded {
                evidence: SpeedEvidence { http_status: 401, latency_ms: latency },
            },
        };
        let snapshot = DiagnosticJobSnapshot {
            job: job.clone(),
            state: DiagnosticJobState::Running {},
            progress: DiagnosticProgress { completed: 2, total: Some(3) },
            results: Some(vec![
                DiagnosticResult::Speed {
                    endpoint_id: DiagnosticEndpointId("fast".into()),
                    report: report(7),
                },
                DiagnosticResult::Speed {
                    endpoint_id: DiagnosticEndpointId("failed".into()),
                    report: DiagnosticReport {
                        source: DiagnosticEvidenceSource::ControlledFixture,
                        timings: vec![],
                        outcome: DiagnosticOutcome::Failed {
                            error: DiagnosticFailure {
                                phase: DiagnosticPhase::Connect,
                                code: DiagnosticFailureCode::Timeout,
                            },
                            http_status: None,
                        },
                    },
                },
            ]),
        };
        let (reply, request) = drive(task, serde_json::to_value(&snapshot).unwrap());
        assert!(request.starts_with("GET /v1/agent-apps/diagnostics/jobs/speed-job?"));
        drop(app.update(reply));
        let state = app.app_providers.endpoint_speed.as_ref().unwrap();
        assert_eq!(
            state.sorted().iter().map(|c| c.id.0.as_str()).collect::<Vec<_>>(),
            ["fast", "slow", "failed"]
        );
        assert!(
            super::endpoints_modal::report_text(
                state.report(&DiagnosticEndpointId("fast".into())).unwrap()
            )
            .contains("7ms · HTTP 401")
        );
        assert!(
            super::endpoints_modal::report_text(
                state.report(&DiagnosticEndpointId("failed".into())).unwrap()
            )
            .contains("Timeout")
        );
        runtime.block_on(render_panel(state));
        drop(app.update(endpoints::message(Event::Select(DiagnosticEndpointId("fast".into())))));
        assert_eq!(
            app.app_providers.form.as_ref().unwrap().claude.as_ref().unwrap().base_url,
            "https://saved.example"
        );
        assert!(_requests.try_recv().is_err(), "selection cannot write or start another probe");
        let mut cancelled = snapshot.clone();
        cancelled.state = DiagnosticJobState::Cancelling {};
        let task = app.update(endpoints::message(Event::Cancel));
        let (reply, request) = drive(task, serde_json::to_value(&cancelled).unwrap());
        assert!(request.starts_with("POST /v1/agent-apps/diagnostics/jobs/speed-job/cancel?"));
        let task = app.update(reply);
        assert!(app.app_providers.endpoint_speed.as_ref().unwrap().active());
        cancelled.state = DiagnosticJobState::Cancelled {};
        let (reply, _) = drive(task, serde_json::to_value(&cancelled).unwrap());
        drop(app.update(reply));
        let state = app.app_providers.endpoint_speed.as_mut().unwrap();
        assert!(!state.active());
        let mut late = cancelled.clone();
        late.state = DiagnosticJobState::Succeeded {};
        assert!(
            !state.accept(state.serial, late),
            "late success cannot replace cancelled terminal state"
        );
        assert_eq!(state.candidates.len(), 3, "failed and unfinished candidates remain visible");
        // Cancel editing discards selection and leaves the real provider draft intact.
        drop(app.update(endpoints::message(Event::Close)));
        assert!(app.app_providers.endpoint_speed.is_none());
        assert_eq!(
            app.app_providers.form.as_ref().unwrap().claude.as_ref().unwrap().base_url,
            "https://saved.example"
        );
        let task = app.update(endpoints::message(Event::Open));
        let (reply, _) = drive(
            task,
            serde_json::to_value(DiagnosticEndpointCandidatesResponse {
                candidates: candidates.clone(),
            })
            .unwrap(),
        );
        drop(app.update(reply));
        drop(app.update(endpoints::message(Event::Candidates(
            endpoints::Reply { context, generation, transport },
            Ok(DiagnosticEndpointCandidatesResponse { candidates: vec![] }),
        ))));
        assert_eq!(
            app.app_providers.endpoint_speed.as_ref().unwrap().candidates.len(),
            3,
            "previous editor generation cannot overwrite candidates"
        );
        drop(app.update(endpoints::message(Event::Select(DiagnosticEndpointId("fast".into())))));
        // Closing while submission is in flight must cancel the accepted job and await cleanup.
        let mut second_job = job.clone();
        second_job.task_id = "second-speed-job".into();
        let task = app.update(endpoints::message(Event::Start));
        let (started, _) = drive(task, serde_json::to_value(&second_job).unwrap());
        drop(app.update(endpoints::message(Event::Close)));
        assert!(app.app_providers.endpoint_speed.as_ref().unwrap().closing);
        let task = app.update(started);
        let mut stopping = cancelled.clone();
        stopping.job = second_job;
        stopping.state = DiagnosticJobState::Cancelling {};
        let (reply, request) = drive(task, serde_json::to_value(&stopping).unwrap());
        assert!(
            request.starts_with("POST /v1/agent-apps/diagnostics/jobs/second-speed-job/cancel?")
        );
        let task = app.update(reply);
        assert!(
            app.app_providers.endpoint_speed.is_some(),
            "editing closes only after cleanup confirmation"
        );
        stopping.state = DiagnosticJobState::Cancelled {};
        let (reply, _) = drive(task, serde_json::to_value(&stopping).unwrap());
        drop(app.update(reply));
        assert!(app.app_providers.endpoint_speed.is_none());
        assert_eq!(
            app.app_providers.form.as_ref().unwrap().claude.as_ref().unwrap().base_url,
            "https://saved.example"
        );
        let task = app.update(endpoints::message(Event::Open));
        let (reply, _) = drive(
            task,
            serde_json::to_value(DiagnosticEndpointCandidatesResponse {
                candidates: candidates.clone(),
            })
            .unwrap(),
        );
        drop(app.update(reply));
        // Address-book editing reuses its original form/SDK without throwing away the provider draft.
        let task = app.update(endpoints::message(Event::ManageCustom));
        let (reply, request) = drive(task, serde_json::json!([]));
        assert!(request.starts_with("GET /v1/app-providers/claude/provider/endpoints "));
        let task = app.update(reply);
        let (reply, _) = drive(
            task,
            serde_json::json!({"headersConfigured":false,"overridesConfigured":false,"application":"not_applied"}),
        );
        drop(app.update(reply));
        let task = app.update(crate::app::message::app_providers::endpoints::message(
            crate::app::message::app_providers::endpoints::Event::Cancel,
        ));
        let (reply, _) = drive(
            task,
            serde_json::to_value(DiagnosticEndpointCandidatesResponse {
                candidates: candidates.clone(),
            })
            .unwrap(),
        );
        drop(app.update(reply));
        assert!(app.app_providers.endpoints.is_none());
        assert_eq!(
            app.app_providers.form.as_ref().unwrap().claude.as_ref().unwrap().base_url,
            "https://saved.example"
        );
        drop(app.update(endpoints::message(Event::Preset("PackyCode".into()))));
        let task = app.update(endpoints::message(Event::Reload));
        let (reply, request) = drive(
            task,
            serde_json::to_value(DiagnosticEndpointCandidatesResponse {
                candidates: candidates.clone(),
            })
            .unwrap(),
        );
        let candidate_body: serde_json::Value =
            serde_json::from_str(request.split_once("\r\n\r\n").unwrap().1).unwrap();
        assert_eq!(candidate_body["presetName"], "PackyCode");
        assert_eq!(candidate_body["draftUrls"], serde_json::json!(["https://saved.example"]));
        drop(app.update(reply));
        drop(app.update(endpoints::message(Event::Select(DiagnosticEndpointId("fast".into())))));
        drop(app.update(endpoints::message(Event::Apply)));
        assert!(app.app_providers.endpoint_speed.is_none());
        assert_eq!(
            app.app_providers.form.as_ref().unwrap().claude.as_ref().unwrap().base_url,
            candidates[1].display_url
        );
        assert!(_requests.try_recv().is_err(), "applying a candidate only changes the draft");
        // The existing provider save action is the only configuration mutation.
        let task = app.update(crate::app::message::app_providers::edit::message(
            crate::app::message::app_providers::edit::EditMessage::Save,
        ));
        let (reply, request) =
            drive(task, serde_json::json!({"catalogSaved":true,"live":"not_attempted"}));
        assert!(request.starts_with("PATCH /v1/app-providers/claude/provider/form "), "{request}");
        let persisted: serde_json::Value =
            serde_json::from_str(request.split_once("\r\n\r\n").unwrap().1).unwrap();
        assert_eq!(persisted["config"]["baseUrl"], candidates[1].display_url);
        assert_eq!(persisted["config"]["apiKey"]["action"], "keep");
        assert_eq!(persisted["apply"], false);
        drop(app.update(reply));
        assert!(
            app.app_providers.form.is_none(),
            "saved acknowledgement closes the existing editor"
        );
        assert!(_requests.try_recv().is_err());
    });
}

async fn render_panel(state: &crate::app::message::agent_apps_diagnostics::endpoint_state::State) {
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
        let mut element =
            super::endpoints_modal::overlay(state, iced::widget::text("应用管理").into());
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
fn diagnostic_desktop_endpoint_selection_requires_save_other_forms() {
    use crate::app::message::agent_apps_diagnostics::endpoint_state;
    use crate::app::state::app_providers::{
        codex,
        edit::{EditForm, Mode},
        gemini,
    };
    let basic =
        ProviderBasic { name: "provider".into(), category: None, notes: None, website_url: None };
    let mut form = EditForm::new(Mode::Update, "provider".into(), basic, None);
    form.codex = Some(codex::Draft::new());
    form.codex.as_mut().unwrap().toml = iced::widget::text_editor::Content::with_text(
        "# keep this comment\nmodel_provider='vendor'\nmodel='kept'\n[model_providers.vendor]\nbase_url='https://draft.example' # keep URL comment\nwire_api='responses'\n[model_providers.other]\nbase_url='https://other.example'\n",
    );
    let query = endpoint_state::query(&form, "PackyCode").unwrap();
    assert_eq!(query.draft_urls, ["https://draft.example"]);
    assert_eq!(query.preset_name.as_deref(), Some("PackyCode"));
    // Selection applies through the same pending editor state used by the UI, no SDK writes.
    let scope = crate::app::state::app_providers::list::Scope {
        context: crate::app::state::AgentAppsContext {
            gateway: "http://localhost:1".into(),
            tenant: Default::default(),
            project: None,
        },
        host: "host".into(),
        app: AppId::Codex,
    };
    let client = vw_gateway_client::GatewayClient::new(vw_gateway_client::GatewayEndpoint::new(
        "127.0.0.1",
        1,
    ))
    .unwrap();
    let id = DiagnosticEndpointId("selected".into());
    let mut state = endpoint_state::State {
        context: crate::app::message::agent_apps_diagnostics::model_state::ModelContext {
            scope,
            project: "project".into(),
            credentials_generation: 0,
            provider: "provider".into(),
        },
        generation: 0,
        transport: endpoint_state::Transport { client, token: "fixture".into() },
        candidates: vec![DiagnosticEndpointCandidate {
            id: id.clone(),
            display_url: "https://selected.example".into(),
            sources: vec![DiagnosticEndpointSource::Draft],
        }],
        selected: None,
        preset_name: String::new(),
        loading: false,
        starting: false,
        cancelling: false,
        closing: false,
        serial: 0,
        job: None,
        snapshot: None,
        error: None,
    };
    let before = form.codex.as_ref().unwrap().toml.text();
    state.choose(id);
    assert_eq!(form.codex.as_ref().unwrap().toml.text(), before);
    state.apply(&mut form).unwrap();
    assert!(form.codex.as_ref().unwrap().toml.text().contains("# keep this comment"));
    assert!(form.codex.as_ref().unwrap().toml.text().contains("# keep URL comment"));
    let doc: toml::Value = toml::from_str(&form.codex.as_ref().unwrap().toml.text()).unwrap();
    assert_eq!(
        doc["model_providers"]["vendor"]["base_url"].as_str(),
        Some("https://selected.example")
    );
    assert_eq!(doc["model_providers"]["other"]["base_url"].as_str(), Some("https://other.example"));
    assert_eq!(doc["model"].as_str(), Some("kept"));
    form.codex.as_mut().unwrap().toml =
        iced::widget::text_editor::Content::with_text("model_provider='missing'");
    assert!(
        state.apply(&mut form).is_err(),
        "missing selected table cannot silently write an ignored root URL"
    );
    form.codex = None;
    form.gemini = Some(gemini::Draft::from_view(
        vw_gateway_client::vw_api_types::app_providers::gemini::GeminiFormKind::ThirdParty {
            base_url: "https://gemini.example".into(),
        },
        None,
    ));
    assert_eq!(endpoint_state::query(&form, "").unwrap().draft_urls, ["https://gemini.example"]);
    state.apply(&mut form).unwrap();
    assert_eq!(form.gemini.as_ref().unwrap().base_url, "https://selected.example");
}
