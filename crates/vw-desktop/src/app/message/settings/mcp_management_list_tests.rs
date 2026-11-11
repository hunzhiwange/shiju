use super::*;
use vw_gateway_client::vw_api_types::{
    TenantScope,
    agent_apps::{AppId, AppInstallationId},
};
fn fixture() -> McpCatalogResponse {
    McpCatalogResponse {
        records: vec![McpCatalogRecord {
            record: McpRecordRef { id: McpRecordId("demo".into()), revision: 4 },
            scope: McpCatalogScope::Tenant {
                scope: TenantScope::new("10000", "100000001").unwrap(),
            },
            name: "中文目录".into(),
            description: None,
            homepage: None,
            documentation: None,
            tags: vec![],
            transport: McpTransport::Stdio,
            connection_revision: 2,
        }],
        bindings: vec![McpCliBinding {
            record: McpRecordRef { id: McpRecordId("demo".into()), revision: 3 },
            target: McpAppTarget {
                app_id: AppId::Codex,
                installation_id: AppInstallationId("fixture".into()),
                target_host_id: "host".into(),
            },
            enabled: true,
            native_status: McpNativeStatus::Missing,
        }],
        capabilities: vec![],
    }
}

#[test]
fn cc_mcp_t44_catalog_distinguishes_scopes_and_observed_native_state() {
    let catalog = fixture();
    let rendered = rows(&catalog).join("\n");
    assert!(rendered.contains("中文目录"), "catalog must display managed records");
    for label in ["原生应用", "识矩运行时", "ACP", "缺失", "旧修订"] {
        assert!(rendered.contains(label), "{rendered} missing {label}");
    }
}
#[test]
fn cc_mcp_t44_old_tenant_response_is_discarded() {
    let old = Context {
        endpoint: AgentAppsContext {
            gateway: "http://localhost:1".into(),
            tenant: Default::default(),
            project: None,
        },
        host: "host".into(),
        credentials_generation: 1,
    };
    let mut current = old.clone();
    current.credentials_generation = 2;
    let mut state =
        CatalogState { context: Some(current.clone()), generation: 2, loading: true, result: None };
    state.finish(
        &current,
        &old,
        1,
        Ok(McpCatalogResponse { records: vec![], bindings: vec![], capabilities: vec![] }),
    );
    assert!(state.result.is_none());
}

#[tokio::test]
async fn cc_mcp_t44_sdk_read_preserves_empty_and_http_failure() {
    use std::{
        io::{BufRead, BufReader, Write},
        net::TcpListener,
    };
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let client = GatewayClient::new(
        vw_gateway_client::GatewayEndpoint::new("127.0.0.1", listener.local_addr().unwrap().port())
            .with_tenant_scope(TenantScope::new("10000", "100000001").unwrap()),
    )
    .unwrap();
    let server = std::thread::spawn(move || {
        for (status, body) in [
            (200, serde_json::to_string(&fixture()).unwrap()),
            (
                200,
                serde_json::to_string(&McpCatalogResponse {
                    records: vec![],
                    bindings: vec![],
                    capabilities: vec![],
                })
                .unwrap(),
            ),
            (
                501,
                serde_json::to_string(&McpFailureResponse {
                    code: McpFailureCode::TargetUnavailable,
                })
                .unwrap(),
            ),
        ] {
            let (mut stream, _) = listener.accept().unwrap();
            stream.set_read_timeout(Some(std::time::Duration::from_secs(5))).unwrap();
            let mut reader = BufReader::new(&mut stream);
            let mut request = String::new();
            loop {
                let mut line = String::new();
                reader.read_line(&mut line).unwrap();
                if line == "\r\n" {
                    break;
                }
                request.push_str(&line);
            }
            assert!(request.starts_with("GET /v1/agent-apps/mcp?targetHostId=host "));
            assert!(request.to_lowercase().contains("x-agent-apps-token: fixture"));
            assert!(request.to_lowercase().contains("x-platform-id: 10000"));
            assert!(request.to_lowercase().contains("x-company-id: 100000001"));
            write!(stream, "HTTP/1.1 {status} Result\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()).unwrap();
        }
    });
    let catalog = load(client.clone(), "host", "fixture").await.unwrap();
    assert_eq!(catalog, fixture());
    assert!(load(client.clone(), "host", "fixture").await.unwrap().records.is_empty());
    let error = load(client, "host", "fixture").await.unwrap_err();
    assert_eq!(
        crate::app::components::system_settings_apps::catalog::error_label(&error),
        "不支持"
    );
    server.join().unwrap();
    assert!(rows(&catalog).join("\n").contains("缺失"));
}

#[tokio::test]
async fn cc_mcp_t44_chinese_catalog_shapes_and_draws_in_both_themes() {
    use iced::{
        Font, Renderer, Size, Theme,
        advanced::renderer::{Headless, Style},
        mouse,
    };
    use iced_runtime::user_interface::{Cache, UserInterface};
    {
        let mut fonts = iced::advanced::graphics::text::font_system().write().unwrap();
        for font in crate::fonts::load_all() {
            fonts.load_font(font);
        }
    }
    let mut renderer =
        Renderer::new(Font::with_name("Noto Sans CJK SC"), 14.0.into(), Some("tiny-skia"))
            .await
            .expect("native renderer");
    let empty = Ok(McpCatalogResponse { records: vec![], bindings: vec![], capabilities: vec![] });
    let populated = Ok(fixture());
    let error = Err("目标主机不支持目录 😀".into());
    for theme in [Theme::Light, Theme::Dark] {
        for (result, loading) in
            [(None, true), (Some(&empty), false), (Some(&error), false), (Some(&populated), false)]
        {
            let element = crate::app::components::system_settings_mcp::managed_catalog_content(
                result, loading,
            );
            let mut ui =
                UserInterface::build(element, Size::new(800., 700.), Cache::new(), &mut renderer);
            ui.draw(
                &mut renderer,
                &theme,
                &Style { text_color: theme.palette().text },
                mouse::Cursor::Unavailable,
            );
        }
    }
}

#[test]
fn cc_mcp_t44_stale_response_preserves_new_gateway_result() {
    let current = Context {
        endpoint: AgentAppsContext {
            gateway: "http://localhost:1".into(),
            tenant: Default::default(),
            project: None,
        },
        host: "host".into(),
        credentials_generation: 2,
    };
    let mut variants = vec![current.clone(); 4];
    variants[0].endpoint.gateway = "http://localhost:2".into();
    variants[1].endpoint.tenant =
        vw_gateway_client::vw_api_types::TenantScope::new("other", "tenant").unwrap();
    variants[2].host = "other".into();
    variants[3].credentials_generation = 1;
    for old in variants {
        let mut state = CatalogState {
            context: Some(current.clone()),
            generation: 2,
            loading: false,
            result: Some(Ok(fixture())),
        };
        state.finish(&current, &old, 1, Err("stale failure".into()));
        assert_eq!(state.result, Some(Ok(fixture())));
    }
}

#[test]
fn cc_mcp_t44_native_statuses_and_transport_capabilities_remain_explicit() {
    for (status, label) in [
        (McpNativeStatus::Present, "存在"),
        (McpNativeStatus::Missing, "缺失"),
        (McpNativeStatus::Unreadable, "无法读取"),
        (McpNativeStatus::NotVerified, "未验证"),
        (McpNativeStatus::Unsupported, "不支持"),
    ] {
        let mut catalog = fixture();
        catalog.bindings[0].native_status = status;
        catalog.bindings[0].enabled = false;
        catalog.capabilities.push(McpAppCapability {
            app_id: vw_gateway_client::vw_api_types::agent_apps::AppId::Agy,
            stdio: McpSupport::Supported,
            streamable_http: McpSupport::NotVerified,
            sse: McpSupport::Unsupported,
        });
        let text = rows(&catalog).join("\n");
        for expected in [label, "期望禁用", "stdio 支持", "HTTP 未验证", "SSE 不支持"] {
            assert!(text.contains(expected), "{text}");
        }
    }
}
