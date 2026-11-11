use crate::app::message::{
    agent_apps_diagnostics as probe,
    settings::{McpMessage, SettingsMessage, mcp_management_list as catalog},
};
use crate::app::{App, Message};
use vw_gateway_client::vw_api_types::{agent_apps::*, mcp_management::*};

fn setup() -> App {
    let mut app = App::new().0;
    app.project_id = Some("project".into());
    app.apps_catalog.host = "host".into();
    app.apps_catalog.token = "fixture".into();
    app.apps_catalog.context = Some(catalog::context(&app).unwrap().endpoint);
    app.apps_catalog.rows.push(crate::app::components::system_settings_apps::catalog::CatalogRow {
        entry: AppCatalogEntry {
            id: AppId::Codex,
            name: "Codex".into(),
            cli_command: Some("codex".into()),
            runtime_kinds: vec![AppRuntimeKind::NativeCli],
        },
        observation: Ok(AppReadResponse { desktop: None, installations: vec![] }),
    });
    drop(probe::update(
        &mut app,
        probe::McpDiagnosticMessage::Select(probe::AppChoice(AppId::Codex, "Codex".into())),
    ));
    drop(catalog::update(&mut app, catalog::CatalogMessage::Refresh));
    let context = catalog::context(&app).unwrap();
    let tenant = context.endpoint.tenant.clone();
    drop(catalog::update(
        &mut app,
        catalog::CatalogMessage::Loaded {
            context,
            generation: 1,
            result: Ok(McpCatalogResponse {
                records: vec![McpCatalogRecord {
                    record: McpRecordRef { id: McpRecordId("docs".into()), revision: 4 },
                    scope: McpCatalogScope::Tenant { scope: tenant },
                    name: "中文服务🧪".into(),
                    description: None,
                    homepage: None,
                    documentation: None,
                    tags: vec![],
                    transport: McpTransport::Stdio,
                    connection_revision: 2,
                }],
                bindings: vec![],
                capabilities: vec![],
            }),
        },
    ));
    app
}

fn draw(app: &App) -> Vec<String> {
    use iced::{
        Font, Rectangle, Renderer, Size, Theme,
        advanced::{
            renderer::{Headless, Style},
            widget::{Id, Operation},
        },
        mouse,
    };
    use iced_runtime::user_interface::{Cache, UserInterface};
    #[derive(Default)]
    struct Labels(Vec<String>);
    impl Operation for Labels {
        fn traverse(&mut self, children: &mut dyn FnMut(&mut dyn Operation)) {
            children(self);
        }
        fn text(&mut self, _: Option<&Id>, _: Rectangle, text: &str) {
            self.0.push(text.into());
        }
    }
    let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
    let mut renderer =
        runtime.block_on(Renderer::new(Font::DEFAULT, 14.0.into(), Some("tiny-skia"))).unwrap();
    {
        let mut fonts = iced::advanced::graphics::text::font_system().write().unwrap();
        for font in crate::fonts::load_all() {
            fonts.load_font(font);
        }
    }
    let mut result = vec![];
    for theme in [Theme::Light, Theme::Dark] {
        let element = crate::app::components::agent_apps_diagnostics::mcp_panel::view(app);
        let mut ui =
            UserInterface::build(element, Size::new(1000., 1000.), Cache::new(), &mut renderer);
        ui.draw(
            &mut renderer,
            &theme,
            &Style { text_color: theme.palette().text },
            mouse::Cursor::Unavailable,
        );
        let mut labels = Labels::default();
        ui.operate(&renderer, &mut labels);
        result.extend(labels.0);
    }
    result
}
#[test]
fn cc_mcp_t49_catalog_load_draws_unchecked_without_starting_probe() {
    let (_lock, _home, _guard) = crate::app::config::isolated_settings();
    let app = setup();
    assert!(!app.mcp_settings.diagnostics.active());
    let labels = draw(&app);
    assert!(
        labels.iter().any(|s| s.contains("unchecked")),
        "missing explicit unchecked state: {labels:?}"
    );
}

use iced::Task;
use vw_gateway_client::vw_api_types::{ProjectScope, agent_apps_diagnostics::*};
fn runtime() -> tokio::runtime::Runtime {
    tokio::runtime::Builder::new_multi_thread().worker_threads(1).enable_all().build().unwrap()
}
fn run(
    task: Task<Message>,
    rt: &tokio::runtime::Runtime,
) -> tokio::task::JoinHandle<probe::McpDiagnosticMessage> {
    use iced_runtime::{Action, futures::futures::StreamExt};
    rt.spawn(async move {
        let mut stream = iced_runtime::task::into_stream(task).expect("network task");
        while let Some(action) = stream.next().await {
            if let Action::Output(Message::Settings(SettingsMessage::Mcp(
                McpMessage::Diagnostic(event),
            ))) = action
            {
                return event;
            }
        }
        panic!("missing diagnostic response")
    })
}
fn job(context: &probe::Context) -> DiagnosticJobIdentity {
    DiagnosticJobIdentity {
        task_id: "job /?".into(),
        correlation_id: "correlation".into(),
        scope: DiagnosticScope {
            project: ProjectScope::new(context.tenant.clone(), context.query.project_id.clone())
                .unwrap(),
            target_host_id: context.query.target_host_id.clone(),
            app_id: context.query.app_id,
        },
        kind: DiagnosticProbeKind::Mcp,
    }
}
fn snapshot(
    context: &probe::Context,
    generation: u64,
    revision: u64,
    state: DiagnosticJobState,
    outcome: McpConnectionOutcome,
) -> DiagnosticJobSnapshot {
    DiagnosticJobSnapshot {
        job: job(context),
        state,
        progress: DiagnosticProgress { completed: 1, total: Some(1) },
        results: Some(vec![DiagnosticResult::McpConnectionCheck {
            scope: context.connections[0].scope.clone(),
            generation,
            item: McpConnectionCheckItem {
                id: McpRecordId("docs".into()),
                connection_revision: revision,
                outcome,
            },
        }]),
    }
}
fn send(tx: &std::sync::mpsc::Sender<Option<String>>, value: &impl serde::Serialize) {
    tx.send(Some(serde_json::to_string(value).unwrap())).unwrap();
}
#[test]
fn cc_mcp_t49_real_check_uses_generation_and_revision_then_edit_clears_connected() {
    crate::app::config::with_settings_gateway(|requests, responses| {
        let rt = runtime();
        let mut app = setup();
        let context = probe::context(&app).unwrap();
        assert!(requests.try_recv().is_err(), "loading/selecting must not probe");
        let task = probe::update(
            &mut app,
            probe::McpDiagnosticMessage::Start(vec![McpRecordId("docs".into())]),
        );
        let generation = app.mcp_settings.diagnostics.generation;
        assert!(draw(&app).iter().any(|s| s.contains("checking")));
        let start = run(task, &rt);
        let request = requests.recv_timeout(std::time::Duration::from_secs(10)).unwrap();
        assert!(request.starts_with("POST /v1/agent-apps/mcp/checks?"), "{request}");
        assert!(
            request.contains("projectId=project")
                && request.contains("targetHostId=host")
                && request.contains("appId=codex")
        );
        assert!(request.to_lowercase().contains("x-agent-apps-token: fixture"));
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(request.split("\r\n\r\n").nth(1).unwrap())
                .unwrap(),
            serde_json::json!({"serverIds":["docs"],"generation":generation})
        );
        send(&responses, &job(&context));
        let poll = run(probe::update(&mut app, rt.block_on(start).unwrap()), &rt);
        let request = requests.recv_timeout(std::time::Duration::from_secs(10)).unwrap();
        assert!(request.starts_with("GET /v1/agent-apps/diagnostics/jobs/job%20%2F%3F?"));
        send(
            &responses,
            &snapshot(
                &context,
                generation,
                2,
                DiagnosticJobState::Succeeded {},
                McpConnectionOutcome::Connected { tool_count: Some(3) },
            ),
        );
        drop(probe::update(&mut app, rt.block_on(poll).unwrap()));
        assert!(draw(&app).iter().any(|s| s.contains("connected")));
        let mut response = catalog::visible(&app).unwrap().as_ref().unwrap().clone();
        response.records[0].connection_revision += 1;
        drop(catalog::update(&mut app, catalog::CatalogMessage::Refresh));
        let ctx = catalog::context(&app).unwrap();
        drop(catalog::update(
            &mut app,
            catalog::CatalogMessage::Loaded { context: ctx, generation: 2, result: Ok(response) },
        ));
        assert!(draw(&app).iter().any(|s| s.contains("unchecked")));
        assert!(!draw(&app).iter().any(|s| s.contains("connected")));
        assert!(requests.try_recv().is_err(), "edit/refresh must not recheck");
    });
}
#[test]
fn cc_mcp_t49_late_registration_after_leaving_is_revoked_with_captured_authority() {
    crate::app::config::with_settings_gateway(|requests, responses| {
        let rt = runtime();
        let mut app = setup();
        let context = probe::context(&app).unwrap();
        let task = probe::update(
            &mut app,
            probe::McpDiagnosticMessage::Start(vec![McpRecordId("docs".into())]),
        );
        let start = run(task, &rt);
        assert!(
            requests
                .recv_timeout(std::time::Duration::from_secs(10))
                .unwrap()
                .starts_with("POST /v1/agent-apps/mcp/checks?")
        );
        app.show_system_settings = false;
        drop(app.update(Message::Settings(SettingsMessage::Mcp(McpMessage::TabSelected(
            crate::app::state::McpSettingsTab::Servers,
        )))));
        // Even if the current credential changes, revocation uses the original authority.
        app.apps_catalog.token = "replacement".into();
        app.apps_catalog.generation += 1;
        send(&responses, &job(&context));
        let cleanup = run(probe::update(&mut app, rt.block_on(start).unwrap()), &rt);
        let cancel = requests.recv_timeout(std::time::Duration::from_secs(10)).unwrap();
        assert!(
            cancel.starts_with("POST /v1/agent-apps/diagnostics/jobs/job%20%2F%3F/cancel?"),
            "{cancel}"
        );
        assert!(cancel.to_lowercase().contains("x-agent-apps-token: fixture"));
        send(
            &responses,
            &snapshot(
                &context,
                1,
                2,
                DiagnosticJobState::Cancelling {},
                McpConnectionOutcome::Cancelled,
            ),
        );
        let poll = requests.recv_timeout(std::time::Duration::from_secs(10)).unwrap();
        assert!(poll.starts_with("GET /v1/agent-apps/diagnostics/jobs/"));
        send(
            &responses,
            &snapshot(
                &context,
                1,
                2,
                DiagnosticJobState::Cancelled {},
                McpConnectionOutcome::Cancelled,
            ),
        );
        assert!(matches!(
            rt.block_on(cleanup).unwrap(),
            probe::McpDiagnosticMessage::Cleaned(Ok(()))
        ));
        assert!(app.mcp_settings.diagnostics.targets.is_empty());
        assert!(requests.try_recv().is_err());
    });
}
#[test]
fn cc_mcp_t49_stale_generation_revision_scope_job_and_duplicate_results_are_unchecked() {
    let (_lock, _home, _guard) = crate::app::config::isolated_settings();
    let mut app = setup();
    let context = probe::context(&app).unwrap();
    for case in 0..5 {
        let state = &mut app.mcp_settings.diagnostics;
        state.begin(context.clone(), context.records.clone());
        state.job = Some(job(&context));
        let generation = state.generation;
        let mut snap = snapshot(
            &context,
            generation,
            2,
            DiagnosticJobState::Succeeded {},
            McpConnectionOutcome::Connected { tool_count: None },
        );
        match case {
            0 => {
                if let Some(DiagnosticResult::McpConnectionCheck { generation, .. }) =
                    snap.results.as_mut().and_then(|r| r.first_mut())
                {
                    *generation += 1;
                }
            }
            1 => {
                if let Some(DiagnosticResult::McpConnectionCheck { item, .. }) =
                    snap.results.as_mut().and_then(|r| r.first_mut())
                {
                    item.connection_revision += 1;
                }
            }
            2 => {
                if let Some(DiagnosticResult::McpConnectionCheck { scope, .. }) =
                    snap.results.as_mut().and_then(|r| r.first_mut())
                {
                    *scope = McpCatalogScope::Project { scope: snap.job.scope.project.clone() };
                }
            }
            3 => {
                snap.job.task_id = "foreign".into();
            }
            _ => {
                let values = snap.results.as_mut().unwrap();
                values.push(values[0].clone());
            }
        }
        state.accept(&context, &context, generation, state.request_serial, snap);
        assert_ne!(state.targets[0].1, probe::Phase::Succeeded, "case {case}");
    }
}
#[test]
fn cc_mcp_t49_cancel_before_registration_waits_and_error_cancelled_states_draw() {
    let (_lock, _home, _guard) = crate::app::config::isolated_settings();
    let mut app = setup();
    let context = probe::context(&app).unwrap();
    let task = probe::update(
        &mut app,
        probe::McpDiagnosticMessage::Start(vec![McpRecordId("docs".into())]),
    );
    drop(task);
    drop(probe::update(&mut app, probe::McpDiagnosticMessage::Cancel));
    assert!(app.mcp_settings.diagnostics.cancelling);
    assert!(!app.mcp_settings.diagnostics.terminal, "cancel request is not cleanup confirmation");
    for (outcome, label) in [
        (McpConnectionOutcome::Failed { code: McpFailureCode::TargetUnavailable }, "error"),
        (McpConnectionOutcome::Cancelled, "cancelled"),
    ] {
        let state = &mut app.mcp_settings.diagnostics;
        state.begin(context.clone(), context.records.clone());
        state.job = Some(job(&context));
        let snap =
            snapshot(&context, state.generation, 2, DiagnosticJobState::Succeeded {}, outcome);
        assert!(state.accept(&context, &context, state.generation, state.request_serial, snap));
        assert!(draw(&app).iter().any(|s| s.contains(label)));
    }
}
#[test]
fn cc_mcp_t49_cancel_while_registering_and_leaving_registered_job_both_revoke() {
    for leave in [false, true] {
        crate::app::config::with_settings_gateway(|requests, responses| {
            let rt = runtime();
            let mut app = setup();
            let context = probe::context(&app).unwrap();
            let task = probe::update(
                &mut app,
                probe::McpDiagnosticMessage::Start(vec![McpRecordId("docs".into())]),
            );
            let start = run(task, &rt);
            requests.recv_timeout(std::time::Duration::from_secs(10)).unwrap();
            if !leave {
                drop(probe::update(&mut app, probe::McpDiagnosticMessage::Cancel));
            }
            send(&responses, &job(&context));
            let next = probe::update(&mut app, rt.block_on(start).unwrap());
            let pending = if leave {
                drop(next); // No poll runs before the navigation event.
                app.show_system_settings = false;
                run(
                    app.update(Message::Settings(SettingsMessage::Mcp(McpMessage::TabSelected(
                        crate::app::state::McpSettingsTab::Servers,
                    )))),
                    &rt,
                )
            } else {
                run(next, &rt)
            };
            let request = requests.recv_timeout(std::time::Duration::from_secs(10)).unwrap();
            assert!(
                request.starts_with("POST /v1/agent-apps/diagnostics/jobs/job%20%2F%3F/cancel?")
            );
            send(
                &responses,
                &snapshot(
                    &context,
                    1,
                    2,
                    DiagnosticJobState::Cancelled {},
                    McpConnectionOutcome::Cancelled,
                ),
            );
            let event = rt.block_on(pending).unwrap();
            if leave {
                assert!(matches!(event, probe::McpDiagnosticMessage::Cleaned(Ok(()))));
            } else {
                drop(probe::update(&mut app, event));
                assert!(app.mcp_settings.diagnostics.terminal);
                assert!(draw(&app).iter().any(|s| s.contains("cancelled")));
            }
        });
    }
}
#[test]
fn cc_mcp_t49_definition_save_and_binding_mutation_clear_completed_results() {
    let (_lock, _home, _guard) = crate::app::config::isolated_settings();
    let mut app = setup();
    app.show_system_settings = true;
    app.system_settings_tab = crate::app::components::system_settings::SystemTab::Agents;
    app.agents_settings.active_config_tab = crate::app::state::AgentConfigTab::Mcp;
    let context = probe::context(&app).unwrap();
    for binding in [false, true] {
        let state = &mut app.mcp_settings.diagnostics;
        state.begin(context.clone(), context.records.clone());
        state.job = Some(job(&context));
        let snap = snapshot(
            &context,
            state.generation,
            2,
            DiagnosticJobState::Succeeded {},
            McpConnectionOutcome::Connected { tool_count: None },
        );
        state.accept(&context, &context, state.generation, state.request_serial, snap);
        app.mcp_settings.saving = !binding;
        app.mcp_settings.bindings.running = binding;
        drop(app.update(Message::Settings(SettingsMessage::Mcp(McpMessage::TabSelected(
            crate::app::state::McpSettingsTab::Servers,
        )))));
        assert!(app.mcp_settings.diagnostics.targets.is_empty());
        assert!(draw(&app).iter().any(|s| s.contains("unchecked")));
    }
}
#[test]
fn cc_mcp_t49_cancelling_snapshot_preserves_completed_rows_until_cleanup_terminal() {
    let (_lock, _home, _guard) = crate::app::config::isolated_settings();
    let app = setup();
    let mut context = probe::context(&app).unwrap();
    let mut second = context.connections[0].clone();
    second.record.id = McpRecordId("second".into());
    context.records.push(second.record.clone());
    context.connections.push(second);
    let mut state = probe::McpState::default();
    state.begin(context.clone(), context.records.clone());
    state.job = Some(job(&context));
    let mut snap = snapshot(
        &context,
        state.generation,
        2,
        DiagnosticJobState::Running {},
        McpConnectionOutcome::Connected { tool_count: Some(1) },
    );
    snap.progress.total = Some(2);
    state.accept(&context, &context, state.generation, state.request_serial, snap.clone());
    assert_eq!(state.targets[0].1, probe::Phase::Succeeded);
    state.cancelling = true;
    snap.state = DiagnosticJobState::Cancelling {};
    snap.results = None;
    assert!(state.accept(&context, &context, state.generation, state.request_serial, snap.clone()));
    assert_eq!(state.targets[0].1, probe::Phase::Succeeded);
    assert_eq!(state.targets[1].1, probe::Phase::Cancelling);
    assert!(!state.terminal);
    snap.state = DiagnosticJobState::Cancelled {};
    assert!(state.accept(&context, &context, state.generation, state.request_serial, snap));
    assert!(state.terminal);
    assert_eq!(state.targets[0].1, probe::Phase::Succeeded);
    assert_eq!(state.targets[1].1, probe::Phase::Cancelled);
}
