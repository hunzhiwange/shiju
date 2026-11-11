use super::catalog::rows;
use super::*;

#[test]
fn cc_mcp_t46_unsupported_applications_explain_native_distribution_rejection() {
    let catalog = McpCatalogResponse {
        records: vec![],
        bindings: vec![],
        capabilities: [AppId::ClaudeDesktop, AppId::OpenClaw]
            .map(|app_id| McpAppCapability {
                app_id,
                stdio: McpSupport::Unsupported,
                streamable_http: McpSupport::Unsupported,
                sse: McpSupport::Unsupported,
            })
            .to_vec(),
    };
    let labels = rows(&catalog).join("\n");
    assert!(
        labels.contains("上游不支持原生分发"),
        "unsupported applications need a reason: {labels}"
    );
}

fn record(id: &str) -> McpCatalogRecord {
    McpCatalogRecord {
        record: McpRecordRef { id: McpRecordId(id.into()), revision: 4 },
        scope: McpCatalogScope::Tenant {
            scope: vw_gateway_client::vw_api_types::TenantScope::new("10000", "100000001").unwrap(),
        },
        name: "中文连接 😀".into(),
        description: None,
        homepage: None,
        documentation: None,
        tags: vec![],
        transport: McpTransport::Stdio,
        connection_revision: 2,
    }
}
fn target() -> McpAppTarget {
    McpAppTarget {
        app_id: AppId::Codex,
        installation_id: vw_gateway_client::vw_api_types::agent_apps::AppInstallationId(
            "trusted-instance".into(),
        ),
        target_host_id: "host".into(),
    }
}
fn result() -> McpDistributionResult {
    McpDistributionResult {
        outcome: McpBatchOutcome::PartiallySucceeded,
        items: vec![
            McpDistributionItem {
                record: record("a").record,
                target: target(),
                enabled: false,
                desired_committed: true,
                native_outcome: McpDistributionOutcome::Applied,
            },
            McpDistributionItem {
                record: record("b").record,
                target: target(),
                enabled: false,
                desired_committed: true,
                native_outcome: McpDistributionOutcome::Failed { code: McpFailureCode::Conflict },
            },
        ],
    }
}
#[test]
fn cc_mcp_t46_partial_and_unverified_items_cannot_report_all_success() {
    let mut response = result();
    assert_ne!(summary(&response), "全部目标已应用");
    assert!(item_label(&response.items[1]).contains("Conflict"));
    assert!(item_label(&response.items[1]).contains("期望已保存"));
    response.outcome = McpBatchOutcome::Succeeded;
    for native in [
        McpDistributionOutcome::NotVerified,
        McpDistributionOutcome::Unsupported,
        McpDistributionOutcome::Cancelled,
    ] {
        response.items[1].native_outcome = native;
        assert_ne!(summary(&response), "全部目标已应用");
    }
    response.items[1].native_outcome = McpDistributionOutcome::Unchanged;
    response.items[1].desired_committed = false;
    assert_ne!(summary(&response), "全部目标已应用");
    response.items[1].desired_committed = true;
    assert_eq!(summary(&response), "全部目标已应用");
}
#[test]
fn cc_mcp_t46_fourteen_applications_and_transport_limits_are_explicit() {
    assert_eq!(APPS.iter().collect::<std::collections::HashSet<_>>().len(), 14);
    assert_eq!(APPS.iter().filter(|app| unsupported_reason(**app).is_some()).count(), 2);
    let mut records = vec![record("a")];
    let catalog = McpCatalogResponse {
        records: records.clone(),
        bindings: vec![],
        capabilities: vec![McpAppCapability {
            app_id: AppId::Agy,
            stdio: McpSupport::Supported,
            streamable_http: McpSupport::Supported,
            sse: McpSupport::Unsupported,
        }],
    };
    assert!(supported(&catalog, &records, AppId::Agy));
    records[0].transport = McpTransport::Sse;
    assert!(!supported(&catalog, &records, AppId::Agy));
    assert!(!supported(&catalog, &records, AppId::Codex));
    assert!(!supported(&catalog, &[], AppId::Agy));
    records.push(record("b"));
    records[1].scope = McpCatalogScope::Project {
        scope: vw_gateway_client::vw_api_types::ProjectScope::new(
            vw_gateway_client::vw_api_types::TenantScope::new("10000", "100000001").unwrap(),
            "project".into(),
        )
        .unwrap(),
    };
    assert!(project(&records).is_err());
    assert!(project(&[]).is_err());
    assert!(project(&vec![record("a"); 257]).is_err());
}
#[test]
fn cc_mcp_t46_old_context_and_duplicate_completion_cannot_replace_results() {
    let context = catalog::Context {
        endpoint: crate::app::state::AgentAppsContext {
            gateway: "http://localhost:1".into(),
            tenant: Default::default(),
            project: None,
        },
        host: "host".into(),
        credentials_generation: 2,
    };
    let mut old = context.clone();
    old.credentials_generation = 1;
    let mut state = BindingsState {
        context: Some(context.clone()),
        running: true,
        generation: 8,
        selected: vec![record("a")],
        result: None,
    };
    assert!(!state.finish(&context, &old, 8, Ok(result())));
    assert!(!state.finish(&context, &context, 7, Ok(result())));
    assert!(state.running);
    assert!(state.finish(&context, &context, 8, Ok(result())));
    assert!(!state.running);
    assert!(state.selected.is_empty());
    assert!(!state.finish(&context, &context, 8, Err("late error".into())));
    assert_eq!(state.result, Some(Ok(result())));
}

#[tokio::test]
async fn cc_mcp_t46_single_and_bulk_use_sdk_with_revisions_and_preserve_each_target_result() {
    use std::{
        io::{BufRead, BufReader, Read, Write},
        net::TcpListener,
    };
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let client = GatewayClient::new(
        vw_gateway_client::GatewayEndpoint::new("127.0.0.1", listener.local_addr().unwrap().port())
            .with_tenant_scope(
                vw_gateway_client::vw_api_types::TenantScope::new("10000", "100000001").unwrap(),
            ),
    )
    .unwrap();
    let server = std::thread::spawn(move || {
        for index in 0..4 {
            let (mut stream, _) = listener.accept().unwrap();
            stream.set_read_timeout(Some(std::time::Duration::from_secs(10))).unwrap();
            let mut reader = BufReader::new(&mut stream);
            let mut headers = String::new();
            let mut size = 0;
            loop {
                let mut line = String::new();
                assert!(reader.read_line(&mut line).unwrap() > 0);
                if line == "\r\n" {
                    break;
                }
                if let Some(value) = line.to_lowercase().strip_prefix("content-length:") {
                    size = value.trim().parse::<usize>().unwrap();
                }
                headers.push_str(&line);
            }
            let mut body = vec![0; size];
            reader.read_exact(&mut body).unwrap();
            let body: serde_json::Value = serde_json::from_slice(&body).unwrap();
            assert!(headers.to_lowercase().contains("x-agent-apps-token: fixture"));
            assert!(headers.to_lowercase().contains("x-platform-id: 10000"));
            assert!(headers.to_lowercase().contains("x-company-id: 100000001"));
            let mut response = result();
            if index == 0 {
                assert!(headers.starts_with("PUT /v1/agent-apps/mcp/a/bindings "));
                let request: McpBindingRequest = serde_json::from_value(body).unwrap();
                assert_eq!(request.expected_revision, 4);
                assert!(request.enabled);
                assert_eq!(request.project_id, None);
                assert_eq!(request.target, target());
                response.items.truncate(1);
                response.items[0].enabled = true;
                response.outcome = McpBatchOutcome::Succeeded;
            } else {
                assert!(headers.starts_with("POST /v1/agent-apps/mcp/bulk "));
                let request: McpBulkRequest = serde_json::from_value(body).unwrap();
                assert_eq!(request.records, vec![record("a").record, record("b").record]);
                assert!(!request.enabled);
                assert_eq!(request.target, target());
                assert_eq!(request.project_id, None);
            }
            if index == 2 {
                response.items.truncate(1);
            }
            if index == 3 {
                response.items[1].target.target_host_id = "foreign".into();
            }
            let body = serde_json::to_string(&response).unwrap();
            write!(stream, "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()).unwrap();
        }
    });
    let single =
        apply(client.clone(), vec![record("a")], target(), true, "fixture".into()).await.unwrap();
    assert_eq!(summary(&single), "全部目标已应用");
    let partial =
        apply(client.clone(), vec![record("a"), record("b")], target(), false, "fixture".into())
            .await
            .unwrap();
    assert_eq!(partial, result());
    assert_ne!(summary(&partial), "全部目标已应用");
    for _ in 0..2 {
        assert!(
            apply(
                client.clone(),
                vec![record("a"), record("b")],
                target(),
                false,
                "fixture".into()
            )
            .await
            .is_err()
        );
    }
    server.join().unwrap();
}

#[test]
fn cc_mcp_t46_matrix_draws_chinese_results_and_preserves_runtime_and_acp_selections() {
    use crate::app::components::system_settings_apps::catalog::CatalogRow;
    use iced::{
        Font, Renderer, Size, Theme,
        advanced::renderer::{Headless, Style},
        mouse,
    };
    use iced_runtime::user_interface::{Cache, UserInterface};
    use vw_gateway_client::vw_api_types::agent_apps::*;
    crate::app::config::with_settings_gateway(|_, _| {
        let mut app = App::new().0;
        app.apps_catalog.host = "host".into();
        app.apps_catalog.token = "fixture".into();
        let context = catalog::context(&app).unwrap();
        app.apps_catalog.context = Some(context.endpoint.clone());
        let before_runtime = app.mcp_settings.runtime_overrides.clone();
        let before_acp = app.acp_settings.enabled.clone();
        let record = record("a");
        let catalog_response = McpCatalogResponse {
            records: vec![record.clone()],
            bindings: vec![],
            capabilities: vec![McpAppCapability {
                app_id: AppId::Codex,
                stdio: McpSupport::Supported,
                streamable_http: McpSupport::Supported,
                sse: McpSupport::Supported,
            }],
        };
        drop(catalog::update(&mut app, catalog::CatalogMessage::Refresh));
        drop(catalog::update(
            &mut app,
            catalog::CatalogMessage::Loaded {
                context: context.clone(),
                generation: 1,
                result: Ok(catalog_response),
            },
        ));
        app.apps_catalog.rows.push(CatalogRow {
            entry: AppCatalogEntry {
                id: AppId::Codex,
                name: "Codex".into(),
                cli_command: Some("codex".into()),
                runtime_kinds: vec![AppRuntimeKind::NativeCli],
            },
            observation: Ok(AppReadResponse {
                desktop: None,
                installations: vec![AppInstallation {
                    id: target().installation_id,
                    app_id: AppId::Codex,
                    target_host_id: "host".into(),
                    environment: AppInstallationEnvironment::Macos,
                    path: "/fixture/codex".into(),
                    real_path: None,
                    source: AppInstallationSource::Native,
                    is_path_default: true,
                    version: Some("1".into()),
                    runtime_status: AppInstallationRuntimeStatus::Runnable,
                }],
            }),
        });
        let mut obsolete = record.clone();
        obsolete.record.revision = 3;
        app.mcp_settings.bindings.selected.push(obsolete);
        drop(update(&mut app, BindingsMessage::Select(record.clone(), true)));
        assert_eq!(app.mcp_settings.bindings.selected, vec![record]);
        let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
        let mut renderer = runtime
            .block_on(Renderer::new(
                Font::with_name("Noto Sans CJK SC"),
                14.0.into(),
                Some("tiny-skia"),
            ))
            .unwrap();
        {
            let mut fonts = iced::advanced::graphics::text::font_system().write().unwrap();
            for font in crate::fonts::load_all() {
                fonts.load_font(font);
            }
        }
        for response in [Some(Ok(result())), Some(Err("中文目标无法写入 😀".into())), None]
        {
            app.mcp_settings.bindings.result = response;
            for theme in [Theme::Light, Theme::Dark] {
                let element = crate::app::components::system_settings_mcp::view(&app);
                let mut ui = UserInterface::build(
                    element,
                    Size::new(1200., 3000.),
                    Cache::new(),
                    &mut renderer,
                );
                ui.draw(
                    &mut renderer,
                    &theme,
                    &Style { text_color: theme.palette().text },
                    mouse::Cursor::Unavailable,
                );
            }
        }
        let mut invalid_target = target();
        invalid_target.installation_id.0 = "untrusted".into();
        drop(update(&mut app, BindingsMessage::Apply(invalid_target, true)));
        assert!(!app.mcp_settings.bindings.running);
        drop(update(&mut app, BindingsMessage::Apply(target(), true)));
        assert!(app.mcp_settings.bindings.running);
        let generation = app.mcp_settings.bindings.generation;
        drop(update(
            &mut app,
            BindingsMessage::Finished { context, generation, result: Ok(result()) },
        ));
        assert!(!app.mcp_settings.bindings.running);
        assert_eq!(app.mcp_settings.bindings.result, Some(Ok(result())));
        assert_eq!(app.mcp_settings.runtime_overrides, before_runtime);
        assert_eq!(app.acp_settings.enabled, before_acp);
    });
}
