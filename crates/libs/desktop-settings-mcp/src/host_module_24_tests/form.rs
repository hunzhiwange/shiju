use crate::management_context as catalog;
use crate::management_form::*;
use desktop_provider_apps::AgentAppsContext;
use std::collections::BTreeMap;
use vw_api_mcp_management::*;
use vw_gateway_client::GatewayClient;
use {vw_api_app_auth::AuthService, vw_api_transport::TenantScope};
fn context() -> catalog::Context {
    catalog::Context {
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
#[test]
fn cc_mcp_t45_preset_is_only_a_draft_with_no_consumer_enablement() {
    let mut draft = draft();
    let connection = draft.connection().unwrap();
    draft.presets.push(McpPresetDraftView {
        id: "preset".into(),
        name: "预设".into(),
        tags: vec![],
        homepage: String::new(),
        documentation: String::new(),
        connection: connection.clone(),
        enabled: false,
    });
    draft.preset(0);
    assert!(draft.connection().unwrap() == connection);
    assert!(draft.record.is_none());
    assert!(!draft.saving);
    assert_eq!(draft.id, "preset");
}

#[tokio::test]
async fn cc_mcp_t45_modal_shapes_and_draws_chinese_and_errors_in_light_and_dark() {
    use iced::{
        Font, Renderer, Size, Theme,
        advanced::renderer::{Headless, Style},
        mouse,
    };
    use iced_runtime::user_interface::{Cache, UserInterface};
    {
        let mut fonts = iced::advanced::graphics::text::font_system().write().unwrap();
        for font in desktop_assets::fonts::load_all() {
            fonts.load_font(font);
        }
    }
    let mut renderer =
        Renderer::new(Font::with_name("Noto Sans CJK SC"), 14.0.into(), Some("tiny-skia"))
            .await
            .unwrap();
    let mut draft = draft();
    for json in [false, true] {
        draft.toggle_json(json).unwrap();
        draft.error = Some("第 2 行，第 3 列：中文错误 😀".into());
        for theme in [Theme::Light, Theme::Dark] {
            let element = crate::management_form::panel(&draft);
            let mut ui =
                UserInterface::build(element, Size::new(760., 700.), Cache::new(), &mut renderer);
            ui.draw(
                &mut renderer,
                &theme,
                &Style { text_color: theme.palette().text },
                mouse::Cursor::Unavailable,
            );
        }
    }
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
#[tokio::test]
async fn cc_mcp_t45_real_sdk_writes_identical_dto_and_keeps_conflict_draft() {
    use std::{
        io::{BufRead, BufReader, Read, Write},
        net::TcpListener,
    };
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let client = GatewayClient::new(
        gateway_endpoint::GatewayEndpoint::new("127.0.0.1", listener.local_addr().unwrap().port())
            .with_tenant_scope(TenantScope::new("10000", "100000001").unwrap()),
    )
    .unwrap();
    let server = std::thread::spawn(move || {
        let mut requests = vec![];
        for status in [201, 201, 409] {
            let (mut stream, _) = listener.accept().unwrap();
            stream.set_read_timeout(Some(std::time::Duration::from_secs(5))).unwrap();
            let mut reader = BufReader::new(&mut stream);
            let mut request = String::new();
            let mut length = 0;
            loop {
                let mut line = String::new();
                reader.read_line(&mut line).unwrap();
                if line == "\r\n" {
                    break;
                }
                if let Some(value) = line.to_lowercase().strip_prefix("content-length:") {
                    length = value.trim().parse().unwrap();
                }
                request.push_str(&line);
            }
            assert!(request.to_lowercase().contains("x-agent-apps-token: fixture"));
            assert!(request.to_lowercase().contains("x-platform-id: 10000"));
            assert!(request.to_lowercase().contains("x-company-id: 100000001"));
            let mut bytes = vec![0; length];
            reader.read_exact(&mut bytes).unwrap();
            let value: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
            if status == 409 {
                assert!(request.starts_with("PUT /v1/agent-apps/mcp/demo "));
                let update: McpUpdateRequest = serde_json::from_value(value.clone()).unwrap();
                assert_eq!(update.expected_revision, 7);
                assert_eq!(update.target_host_id, "host");
            } else {
                assert!(request.starts_with("POST /v1/agent-apps/mcp "));
                let create: McpCreateRequest = serde_json::from_value(value.clone()).unwrap();
                assert_eq!(create.id, "demo");
                assert_eq!(create.target_host_id, "host");
            }
            requests.push(value);
            let body = if status == 409 {
                serde_json::to_string(&McpFailureResponse { code: McpFailureCode::StaleRevision })
                    .unwrap()
            } else {
                serde_json::to_string(&record()).unwrap()
            };
            write!(stream, "HTTP/1.1 {status} Result\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()).unwrap();
        }
        requests
    });
    let mut draft = draft();
    assert_eq!(save(&client, "fixture", draft.clone()).await.unwrap(), record());
    draft.toggle_json(true).unwrap();
    assert_eq!(save(&client, "fixture", draft.clone()).await.unwrap(), record());
    let mut existing = record();
    existing.record.revision = 7;
    existing.connection_revision = 5;
    draft.record = Some(existing);
    draft.saving = true;
    let result = save(&client, "fixture", draft.clone()).await;
    assert!(result.as_ref().unwrap_err().contains("修订冲突"));
    let before = draft.editor.text();
    draft.finish(&result);
    assert_eq!(draft.editor.text(), before);
    assert!(draft.record.is_some());
    let requests = server.join().unwrap();
    assert_eq!(requests[0], requests[1]);
    assert_eq!(requests[0]["connection"], requests[2]["connection"]);
    assert_eq!(requests[0]["connection"]["args"][0], "a,b");
    assert_eq!(
        requests[0]["connection"]["env"]["TOKEN"]["reference"]["accountId"],
        "fixture-account"
    );
}

#[test]
fn cc_mcp_t45_graphical_values_edit_literals_and_references_without_losing_keys() {
    let mut draft = draft();
    draft.edit_values(FormMessage::ValueAdd).unwrap();
    draft.edit_values(FormMessage::ValueName("KEY_1".into(), "PLAIN".into())).unwrap();
    draft.edit_values(FormMessage::ValueText("PLAIN".into(), "value, with spaces".into())).unwrap();
    assert!(draft.edit_values(FormMessage::ValueName("PLAIN".into(), "TOKEN".into())).is_err());
    draft.edit_values(FormMessage::ValueKind("PLAIN".into(), true)).unwrap();
    draft.edit_values(FormMessage::ValueService("PLAIN".into(), "claude_official".into())).unwrap();
    draft.edit_values(FormMessage::ValueText("PLAIN".into(), "second-account".into())).unwrap();
    let connection = draft.connection().unwrap();
    draft.toggle_json(true).unwrap();
    draft.toggle_json(false).unwrap();
    assert!(draft.connection().unwrap() == connection);
    let values: BTreeMap<String, McpConfigValue> = serde_json::from_str(&draft.values).unwrap();
    assert!(
        matches!(&values["PLAIN"], McpConfigValue::Credential { reference } if reference.account_id == "second-account" && reference.service == AuthService::ClaudeOfficial)
    );
    assert!(values.contains_key("TOKEN"));
    draft.edit_values(FormMessage::ValueRemove("PLAIN".into())).unwrap();
    assert!(!draft.values.contains("PLAIN"));
}

#[test]
fn cc_mcp_t45_transport_change_does_not_move_environment_credentials_into_headers() {
    let mut state = crate::management_state::McpSettingsState::default();
    state.connection_form = Some(draft());
    let mut app = crate::management_update::Update {
        state: &mut state,
        context: Some(context()),
        authorized: true,
        token: "fixture".into(),
        client: Err("unused".into()),
        targets: vec![],
        effects: vec![],
    };
    let _ = crate::management_form_update::update(
        &mut app,
        FormMessage::Field("transport", "streamable_http".into()),
    );
    let draft = app.state.connection_form.as_mut().unwrap();
    assert_eq!(
        draft.values, "{}",
        "changing transport must not send environment credentials as HTTP headers"
    );
    draft.values = r#"{"Authorization":{"kind":"credential","reference":{"service":"codex_oauth","accountId":"fixture-account"}}}"#.into();
    let original = draft.values.clone();
    let _ = crate::management_form_update::update(
        &mut app,
        FormMessage::Field("transport", "sse".into()),
    );
    assert_eq!(app.state.connection_form.as_ref().unwrap().values, original);
    let _ = crate::management_form_update::update(
        &mut app,
        FormMessage::Field("transport", "stdio".into()),
    );
    assert_eq!(app.state.connection_form.as_ref().unwrap().values, "{}");
}
