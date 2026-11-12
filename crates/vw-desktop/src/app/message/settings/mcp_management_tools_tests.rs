use crate::app::{App, Message};
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
    fn text(&mut self, _: Option<&Id>, _: Rectangle, value: &str) {
        self.0.push(value.into());
    }
}
fn draw(element: iced::Element<'_, Message>, theme: Theme) -> Vec<String> {
    let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
    let mut renderer =
        runtime.block_on(Renderer::new(Font::DEFAULT, 14.0.into(), Some("tiny-skia"))).unwrap();
    {
        let mut fonts = iced::advanced::graphics::text::font_system().write().unwrap();
        for font in crate::fonts::load_all() {
            fonts.load_font(font);
        }
    }
    let mut ui =
        UserInterface::build(element, Size::new(1200., 3000.), Cache::new(), &mut renderer);
    ui.draw(
        &mut renderer,
        &theme,
        &Style { text_color: theme.palette().text },
        mouse::Cursor::Unavailable,
    );
    let mut labels = Labels::default();
    ui.operate(&renderer, &mut labels);
    labels.0
}
#[test]
fn cc_mcp_t50_settings_exposes_tool_details_without_connecting() {
    let (_lock, _home, _guard) = crate::app::config::isolated_settings();
    let app = App::new().0;
    for theme in [Theme::Light, Theme::Dark] {
        let labels = draw(crate::app::components::system_settings_mcp::view(&app), theme);
        assert!(
            labels.iter().any(|label| label == "工具详情与使用范围"),
            "missing managed tool detail entry: {labels:?}"
        );
    }
}

use super::{
    McpMessage, SettingsMessage, mcp_management_list as catalog, mcp_management_tools as tools,
};
use crate::app::message::agent_apps_diagnostics as probe;
use vw_gateway_client::vw_api_types::{
    agent_apps::*, agent_apps_diagnostics::*, mcp_management::*, tools::*,
};
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

fn runtime() -> tokio::runtime::Runtime {
    tokio::runtime::Builder::new_multi_thread().worker_threads(1).enable_all().build().unwrap()
}
fn run(
    task: iced::Task<Message>,
    rt: &tokio::runtime::Runtime,
) -> tokio::task::JoinHandle<tools::ToolsMessage> {
    use iced_runtime::{Action, futures::futures::StreamExt};
    rt.spawn(async move {
        let mut stream = iced_runtime::task::into_stream(task).expect("network task");
        while let Some(action) = stream.next().await {
            if let Action::Output(Message::Settings(SettingsMessage::Mcp(McpMessage::Tools(
                event,
            )))) = action
            {
                return event;
            }
        }
        panic!("missing tool detail response")
    })
}
fn job(context: &tools::Context) -> DiagnosticJobIdentity {
    DiagnosticJobIdentity {
        task_id: "tool-job /?".into(),
        correlation_id: "details".into(),
        kind: DiagnosticProbeKind::Mcp,
        scope: DiagnosticScope {
            project: vw_gateway_client::vw_api_types::ProjectScope::new(
                context.diagnostics.tenant.clone(),
                context.diagnostics.query.project_id.clone(),
            )
            .unwrap(),
            target_host_id: context.diagnostics.query.target_host_id.clone(),
            app_id: context.diagnostics.query.app_id,
        },
    }
}
fn details(context: &tools::Context, generation: u64) -> McpToolDetails {
    McpToolDetails {
        request: McpToolDetailsRequest {
            scope: context.record.scope.clone(),
            record: context.record.record.clone(),
            connection_revision: context.record.connection_revision,
            generation,
        },
        tools: vec![ToolSpecDto {
            identity: ToolIdentity::new(Some("docs".into()), "search").unwrap(),
            source: ToolSource::Mcp,
            exposure: ToolExposure::Deferred,
            id: "mcp__docs__search".into(),
            display_name: "搜索中文🧪".into(),
            description: "实际服务发现规格".into(),
            input_schema: serde_json::json!({"type":"object","description":"中文参数说明"}),
            aliases: vec![],
            read_only: true,
            destructive: false,
            concurrency_safe: true,
            requires_user_interaction: false,
            strict: true,
        }],
        connection_error: None,
        tools_error: None,
        runtime: Some(McpRuntimeUsage::Selected {
            record: context.record.record.clone(),
            disabled: false,
            generation: 3,
            current: true,
        }),
        runtime_error: None,
    }
}
fn snapshot(context: &tools::Context, generation: u64) -> DiagnosticJobSnapshot {
    DiagnosticJobSnapshot {
        job: job(context),
        state: DiagnosticJobState::Succeeded {},
        progress: DiagnosticProgress { completed: 1, total: Some(1) },
        results: Some(vec![DiagnosticResult::McpToolDetails {
            details: details(context, generation),
        }]),
    }
}
fn send(tx: &std::sync::mpsc::Sender<Option<String>>, value: &impl serde::Serialize) {
    tx.send(Some(serde_json::to_string(value).unwrap())).unwrap();
}
fn record(app: &App) -> McpCatalogRecord {
    catalog::visible(app).unwrap().as_ref().unwrap().records[0].clone()
}
#[test]
fn cc_mcp_t50_real_details_request_draws_discovered_specs_and_independent_runtime_usage() {
    crate::app::config::with_settings_gateway(|requests, responses| {
        let rt = runtime();
        let mut app = setup();
        assert!(requests.try_recv().is_err(), "loading must not discover");
        let selected = record(&app);
        let start = run(tools::update(&mut app, tools::ToolsMessage::Open(selected.clone())), &rt);
        let context = app.mcp_settings.tools.context.clone().unwrap();
        let generation = app.mcp_settings.tools.generation;
        let request = requests.recv_timeout(std::time::Duration::from_secs(10)).unwrap();
        assert!(request.starts_with("POST /v1/agent-apps/mcp/tools/discover?"), "{request}");
        assert!(
            request.contains("targetHostId=host")
                && request.contains("projectId=project")
                && request.contains("appId=codex")
        );
        assert!(request.to_lowercase().contains("x-agent-apps-token: fixture"));
        let body: serde_json::Value =
            serde_json::from_str(request.split("\r\n\r\n").nth(1).unwrap()).unwrap();
        assert_eq!(body, serde_json::to_value(&details(&context, generation).request).unwrap());
        send(&responses, &job(&context));
        let poll = run(tools::update(&mut app, rt.block_on(start).unwrap()), &rt);
        let request = requests.recv_timeout(std::time::Duration::from_secs(10)).unwrap();
        assert!(request.starts_with("GET /v1/agent-apps/diagnostics/jobs/tool-job%20%2F%3F?"));
        send(&responses, &snapshot(&context, generation));
        drop(tools::update(&mut app, rt.block_on(poll).unwrap()));
        assert_eq!(app.mcp_settings.tools.result.as_ref().unwrap().tools.len(), 1);
        for theme in [Theme::Light, Theme::Dark] {
            let labels =
                draw(crate::app::components::system_settings_mcp::tools::card(&app), theme);
            assert!(labels.iter().any(|s| s == "搜索中文🧪"), "{labels:?}");
            assert!(labels.iter().any(|s| s.contains("识矩运行时：已选择")), "{labels:?}");
            assert!(labels.iter().any(|s| s.contains("调用授权：未在此检查")), "{labels:?}");
            assert!(labels.iter().any(|s| s.contains("中文参数说明")), "{labels:?}");
        }
        assert!(requests.try_recv().is_err(), "rendering must not start new I/O");
        app.apps_catalog.generation += 1;
        assert!(tools::visible(&app).is_none(), "old credential generation must hide details");
        drop(tools::invalidate(&mut app));
        assert!(app.mcp_settings.tools.context.is_none());
    });
}
#[test]
fn cc_mcp_t50_details_reject_stale_or_duplicate_evidence_and_keep_errors_separate() {
    let (_lock, _home, _guard) = crate::app::config::isolated_settings();
    for case in 0..7 {
        let mut app = setup();
        let selected = record(&app);
        drop(tools::update(&mut app, tools::ToolsMessage::Open(selected)));
        let state = &mut app.mcp_settings.tools;
        let context = state.context.clone().unwrap();
        let generation = state.generation;
        state.job = Some(job(&context));
        state.serial = 1;
        let mut response = snapshot(&context, generation);
        match case {
            0 => {
                if let Some(DiagnosticResult::McpToolDetails { details }) =
                    response.results.as_mut().unwrap().first_mut()
                {
                    details.request.generation += 1;
                }
            }
            1 => {
                if let Some(DiagnosticResult::McpToolDetails { details }) =
                    response.results.as_mut().unwrap().first_mut()
                {
                    details.request.connection_revision += 1;
                }
            }
            2 => {
                if let Some(DiagnosticResult::McpToolDetails { details }) =
                    response.results.as_mut().unwrap().first_mut()
                {
                    details.request.record.revision += 1;
                }
            }
            3 => response.job.task_id = "foreign-job".into(),
            4 => {
                let duplicate = response.results.as_ref().unwrap()[0].clone();
                response.results.as_mut().unwrap().push(duplicate);
            }
            5 => {
                if let Some(DiagnosticResult::McpToolDetails { details }) =
                    response.results.as_mut().unwrap().first_mut()
                {
                    details.request.scope =
                        McpCatalogScope::Project { scope: response.job.scope.project.clone() };
                }
            }
            _ => (),
        }
        let serial = if case == 6 { 0 } else { 1 };
        drop(tools::update(
            &mut app,
            tools::ToolsMessage::Snapshot(context, generation, serial, Ok(response)),
        ));
        assert!(app.mcp_settings.tools.result.is_none(), "case {case} accepted invalid evidence");
    }
    let mut app = setup();
    let selected = record(&app);
    drop(tools::update(&mut app, tools::ToolsMessage::Open(selected)));
    let state = &mut app.mcp_settings.tools;
    let context = state.context.clone().unwrap();
    let generation = state.generation;
    state.job = Some(job(&context));
    state.serial = 1;
    let mut response = snapshot(&context, generation);
    if let Some(DiagnosticResult::McpToolDetails { details }) =
        response.results.as_mut().unwrap().first_mut()
    {
        details.tools.clear();
        details.tools_error = Some(McpFailureCode::ToolDiscoveryFailed);
    }
    drop(tools::update(
        &mut app,
        tools::ToolsMessage::Snapshot(context, generation, 1, Ok(response)),
    ));
    for theme in [Theme::Light, Theme::Dark] {
        let labels = draw(crate::app::components::system_settings_mcp::tools::card(&app), theme);
        assert!(labels.iter().any(|s| s.contains("工具发现失败")));
        assert!(labels.iter().any(|s| s.contains("识矩运行时：已选择")));
    }
}
#[test]
fn cc_mcp_t50_close_before_registration_cancels_with_the_original_authority() {
    crate::app::config::with_settings_gateway(|requests, responses| {
        use iced_runtime::{Action, futures::futures::StreamExt};
        let rt = runtime();
        let mut app = setup();
        let selected = record(&app);
        let start = run(tools::update(&mut app, tools::ToolsMessage::Open(selected)), &rt);
        let context = app.mcp_settings.tools.context.clone().unwrap();
        let _request = requests.recv_timeout(std::time::Duration::from_secs(10)).unwrap();
        drop(tools::update(&mut app, tools::ToolsMessage::Close));
        app.apps_catalog.token = "new-authority".into();
        app.apps_catalog.generation += 1;
        send(&responses, &job(&context));
        let cleanup = tools::update(&mut app, rt.block_on(start).unwrap());
        let cleanup = rt.spawn(async move {
            let mut stream = iced_runtime::task::into_stream(cleanup).expect("cleanup task");
            while let Some(action) = stream.next().await {
                if let Action::Output(_) = action {
                    return;
                }
            }
            panic!("cleanup did not finish")
        });
        let request = requests.recv_timeout(std::time::Duration::from_secs(10)).unwrap();
        assert!(
            request.starts_with("POST /v1/agent-apps/diagnostics/jobs/tool-job%20%2F%3F/cancel?")
        );
        assert!(request.to_lowercase().contains("x-agent-apps-token: fixture"));
        assert!(!request.contains("new-authority"));
        let mut response = snapshot(&context, 0);
        response.state = DiagnosticJobState::Cancelling {};
        response.results = None;
        send(&responses, &response);
        let request = requests.recv_timeout(std::time::Duration::from_secs(10)).unwrap();
        assert!(request.starts_with("GET /v1/agent-apps/diagnostics/jobs/tool-job%20%2F%3F?"));
        response.state = DiagnosticJobState::Cancelled {};
        send(&responses, &response);
        rt.block_on(cleanup).unwrap();
        assert!(app.mcp_settings.tools.context.is_none());
    });
}
#[test]
fn cc_mcp_t50_acp_usage_reads_exact_session_and_rejects_late_or_foreign_identity() {
    crate::app::config::with_settings_gateway(|requests, responses| {
        let rt = runtime();
        let mut app = setup();
        let selected = record(&app);
        drop(tools::update(&mut app, tools::ToolsMessage::Open(selected.clone())));
        let context = app.mcp_settings.tools.context.clone().unwrap();
        drop(tools::update(&mut app, tools::ToolsMessage::AcpSessionChanged("acp /?".into())));
        let read = run(tools::update(&mut app, tools::ToolsMessage::ReadAcp), &rt);
        let request = requests.recv_timeout(std::time::Duration::from_secs(10)).unwrap();
        assert!(request.starts_with("GET /v1/agent-apps/mcp/acp-mounts/query?"), "{request}");
        assert!(
            request.contains("sessionId=acp+%2F%3F")
                && request.contains("projectId=project")
                && request.contains("targetHostId=host")
        );
        assert!(request.to_lowercase().contains("x-agent-apps-token: fixture"));
        let usage = McpAcpUsage {
            target_host_id: "host".into(),
            project: job(&context).scope.project,
            session_id: "acp /?".into(),
            mount_revision: 7,
            mounts: vec![McpAcpUsageMount {
                scope: selected.scope,
                record: selected.record,
                failure: None,
            }],
        };
        send(&responses, &usage);
        drop(tools::update(&mut app, rt.block_on(read).unwrap()));
        for theme in [Theme::Light, Theme::Dark] {
            let labels =
                draw(crate::app::components::system_settings_mcp::tools::card(&app), theme);
            assert!(labels.iter().any(|s| s.contains("挂载修订 7")), "{labels:?}");
            assert!(labels.iter().any(|s| s.contains("ACP：修订 4 · 有效引用")), "{labels:?}");
        }
        assert!(
            app.mcp_settings.tools.result.is_none(),
            "ACP read must not imply discovery or runtime selection"
        );
        let read = run(tools::update(&mut app, tools::ToolsMessage::ReadAcp), &rt);
        let _ = requests.recv_timeout(std::time::Duration::from_secs(10)).unwrap();
        drop(tools::update(
            &mut app,
            tools::ToolsMessage::AcpSessionChanged("other-session".into()),
        ));
        send(&responses, &usage);
        drop(tools::update(&mut app, rt.block_on(read).unwrap()));
        assert!(app.mcp_settings.tools.acp.is_none(), "old input response must not reappear");
        let read = run(tools::update(&mut app, tools::ToolsMessage::ReadAcp), &rt);
        let _ = requests.recv_timeout(std::time::Duration::from_secs(10)).unwrap();
        send(&responses, &usage);
        drop(tools::update(&mut app, rt.block_on(read).unwrap()));
        assert!(
            matches!(app.mcp_settings.tools.acp, Some(Err(_))),
            "foreign session identity must be refused"
        );
    });
}
