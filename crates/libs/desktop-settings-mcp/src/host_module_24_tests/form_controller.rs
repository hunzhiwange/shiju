use super::fixture::catalog;
use super::fixture::form::*;
use super::fixture::*;
use desktop_provider_apps::AgentAppsContext;
use std::collections::BTreeMap;
use vw_api_mcp_management::*;
use {vw_api_app_auth::AuthService, vw_api_transport::TenantScope};
fn context() -> crate::management_context::Context {
    crate::management_context::Context {
        endpoint: AgentAppsContext {
            gateway: "http://localhost:1".into(),
            tenant: TenantScope::new("10000", "100000001").unwrap(),
            project: None,
        },
        host: "host".into(),
        credentials_generation: 1,
    }
}
fn draft() -> Draft {
    let mut draft = Draft::new(context(), None);
    draft.id = "demo".into();
    draft.name = "中文连接 😀".into();
    draft.command = "fixture-command".into();
    draft.args = r#"["a,b","空 格",""]"#.into();
    draft.values = serde_json::to_string(&BTreeMap::from([(
        "TOKEN",
        McpConfigValue::Credential {
            reference: McpCredentialRef {
                service: AuthService::CodexOauth,
                account_id: "fixture-account".into(),
            },
        },
    )]))
    .unwrap();
    draft
}
fn record() -> McpCatalogRecord {
    McpCatalogRecord {
        record: McpRecordRef { id: McpRecordId("demo".into()), revision: 1 },
        scope: McpCatalogScope::Tenant { scope: TenantScope::new("10000", "100000001").unwrap() },
        name: "中文连接 😀".into(),
        description: None,
        homepage: None,
        documentation: None,
        tags: vec![],
        transport: McpTransport::Stdio,
        connection_revision: 1,
    }
}
#[test]
fn cc_mcp_t45_save_completion_always_rereads_catalog_and_failure_keeps_draft() {
    use iced_runtime::{Action, futures::futures::StreamExt};
    use std::time::Duration;
    super::fixture::with_settings_gateway(|requests, responses| {
        let mut app = Fixture::default();
        app.authority.host = "host".into();
        app.authority.token = "fixture".into();
        let context = catalog::context(&app).unwrap();
        app.authority.context = Some(context.endpoint.clone());
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(1)
            .enable_all()
            .build()
            .unwrap();
        for success in [false, true] {
            let mut draft = draft();
            draft.context = context.clone();
            draft.command = "keep-on-failure".into();
            app.state.connection_form = Some(draft);
            let task = update(&mut app, FormMessage::Save);
            let saving = runtime.spawn(async move {
                let mut stream = iced_runtime::task::into_stream(task).unwrap();
                while let Some(action) = stream.next().await {
                    if let Action::Output(McpMessage::ConnectionForm(event)) = action {
                        return event;
                    }
                }
                panic!("missing save response")
            });
            let request = requests.recv_timeout(Duration::from_secs(5)).unwrap();
            assert!(request.starts_with("POST /v1/agent-apps/mcp "));
            responses.send(success.then(|| serde_json::to_string(&record()).unwrap())).unwrap();
            let event = runtime.block_on(saving).unwrap();
            let task = update(&mut app, event);
            assert!(catalog::loading(&app), "both outcomes must schedule a fresh catalog read");
            if success {
                assert!(app.state.connection_form.is_none());
            } else {
                let draft = app.state.connection_form.as_ref().unwrap();
                assert_eq!(draft.command, "keep-on-failure");
                assert!(!draft.saving);
            }
            let completion = runtime.spawn(async move {
                let mut stream = iced_runtime::task::into_stream(task).unwrap();
                while let Some(action) = stream.next().await {
                    if let Action::Output(McpMessage::Catalog(event)) = action {
                        return event;
                    }
                }
                panic!("missing refresh response")
            });
            let request = requests.recv_timeout(Duration::from_secs(5)).unwrap();
            assert!(request.starts_with("GET /v1/agent-apps/mcp?targetHostId=host "));
            responses
                .send(Some(
                    serde_json::to_string(&McpCatalogResponse {
                        records: vec![record()],
                        bindings: vec![],
                        capabilities: vec![],
                    })
                    .unwrap(),
                ))
                .unwrap();
            let event = runtime.block_on(completion).unwrap();
            let _ = catalog::update(&mut app, event);
            assert_eq!(catalog::visible(&app).unwrap().as_ref().unwrap().records, vec![record()]);
        }
    });
}

#[test]
fn cc_mcp_t45_old_preset_response_does_not_replace_a_reopened_draft() {
    let (_lock, _home, _guard) = super::fixture::isolated_settings();
    let mut app = Fixture::default();
    let old = draft();
    let current = draft();
    let generation = current.generation;
    app.state.connection_form = Some(current);
    let response = || McpPresetsResponse {
        presets: vec![McpPresetDraftView {
            id: "fixture".into(),
            name: "预设".into(),
            tags: vec![],
            homepage: String::new(),
            documentation: String::new(),
            connection: old.connection().unwrap(),
            enabled: false,
        }],
    };
    let _ =
        update(&mut app, FormMessage::Presets(old.context.clone(), old.generation, Ok(response())));
    assert!(app.state.connection_form.as_ref().unwrap().presets.is_empty());
    let _ = update(&mut app, FormMessage::Presets(old.context.clone(), generation, Ok(response())));
    assert_eq!(app.state.connection_form.as_ref().unwrap().presets.len(), 1);
}
