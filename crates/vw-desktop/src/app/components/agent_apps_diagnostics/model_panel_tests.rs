use crate::app::{
    message::agent_apps_diagnostics::model_state::*,
    state::{AgentAppsContext, app_providers::list::Scope},
};
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
use vw_api_app_provider_base::AppProviderSummary;
    use vw_api_app_provider_edit::catalog::*;
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
            crate::app::state::agent_apps::begin(&mut app.apps_catalog, scope.context.clone());
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
