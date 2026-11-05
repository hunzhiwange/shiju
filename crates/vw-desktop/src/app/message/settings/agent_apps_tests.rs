use super::*;
use crate::app::state::AgentAppsState;
use vw_gateway_client::{GatewayClient, GatewayEndpoint, vw_api_types::id::TenantScope};

fn client(company: &str) -> GatewayClient {
    GatewayClient::new(
        GatewayEndpoint::new("127.0.0.1", 1)
            .with_tenant_scope(TenantScope::new("platform", company).unwrap()),
    )
    .unwrap()
}

#[test]
fn cc_base_icon_desktop_state_loading_failure_and_reread() {
    let context = context(&client("a"), Some("/project".into()));
    let mut state = AgentAppsState::default();
    let first = state.begin(context.clone());
    assert!(state.loading);
    assert!(state.finish(&context, &context, first, Err("offline".into())));
    assert!(!state.loading);
    assert_eq!(state.error.as_deref(), Some("offline"));
    let second = state.begin(context.clone());
    assert!(state.loading);
    assert!(state.error.is_none());
    assert!(!state.finish(&context, &context, first, Ok(vec![])));
    assert!(state.finish(&context, &context, second, Ok(vec![])));
    assert!(!state.loading);
    assert!(state.error.is_none());
}

#[test]
fn cc_base_icon_desktop_state_rejects_old_company_project_and_gateway() {
    let old = context(&client("a"), Some("/one".into()));
    let mut state = AgentAppsState::default();
    let generation = state.begin(old.clone());
    let other_company = context(&client("b"), Some("/one".into()));
    let other_project = context(&client("a"), Some("/two".into()));
    let other_gateway = context(
        &GatewayClient::new(GatewayEndpoint::new("127.0.0.1", 2)).unwrap(),
        Some("/one".into()),
    );
    for current in [other_company, other_project, other_gateway] {
        assert!(!state.finish(&current, &old, generation, Err("stale".into())));
        assert!(state.error.is_none());
        assert!(state.icons.is_empty());
    }
}

#[tokio::test]
async fn cc_base_icon_desktop_state_disconnected_does_not_fallback() {
    assert_eq!(load(Err("gateway unavailable".into())).await.unwrap_err(), "gateway unavailable");
    assert!(load(Ok(client("a"))).await.is_err());
}

#[tokio::test]
async fn cc_base_icon_desktop_state_refresh_uses_sdk_and_rereads() {
    use std::io::{Read, Write};
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let server = std::thread::spawn(move || {
        for body in [
            r#"[{"id":"one","name":"工作图标","data":"data:image/png;base64,aA==","createdAt":1}]"#,
            "[]",
        ] {
            let (mut stream, _) = listener.accept().unwrap();
            stream.set_read_timeout(Some(std::time::Duration::from_secs(10))).unwrap();
            let mut request = Vec::new();
            while !request.ends_with(b"\r\n\r\n") {
                let mut byte = [0];
                assert_eq!(stream.read(&mut byte).unwrap(), 1);
                request.push(byte[0]);
            }
            let request = String::from_utf8(request).unwrap().to_lowercase();
            assert!(request.starts_with("get /v1/agent-apps/custom-icons "));
            assert!(request.contains("x-company-id: company"));
            write!(stream, "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}", body.len(), body).unwrap();
        }
    });
    let client = GatewayClient::new(
        GatewayEndpoint::new("127.0.0.1", port)
            .with_tenant_scope(TenantScope::new("platform", "company").unwrap()),
    )
    .unwrap();
    let context = context(&client, None);
    let mut state = AgentAppsState::default();
    let generation = state.begin(context.clone());
    assert!(state.finish(&context, &context, generation, load(Ok(client.clone())).await));
    assert_eq!(state.icons[0].name, "工作图标");
    let generation = state.begin(context.clone());
    assert!(state.finish(&context, &context, generation, load(Ok(client)).await));
    assert!(state.icons.is_empty());
    server.join().unwrap();
}

#[test]
fn cc_base_icon_desktop_state_message_routes_loading_error_toast_and_retry() {
    let mut app = App::new().0;
    let _task =
        super::super::update(&mut app, SettingsMessage::AgentApps(AgentAppsMessage::Refresh));
    assert!(app.agent_apps.loading);
    let client = crate::app::config::gateway_client().unwrap();
    let requested = context(&client, app.project_path.clone());
    // Start a known generation to exercise the public response route.
    let generation = app.agent_apps.begin(requested.clone());
    let _task = super::super::update(
        &mut app,
        SettingsMessage::AgentApps(AgentAppsMessage::Loaded {
            context: requested.clone(),
            generation,
            result: Err("图标服务不可用".into()),
        }),
    );
    assert!(!app.agent_apps.loading);
    assert_eq!(app.agent_apps.error.as_deref(), Some("图标服务不可用"));
    let toast = app.active_toast.as_ref().unwrap();
    assert_eq!(toast.kind, crate::app::state::ToastKind::Error);
    assert_eq!(toast.message, "图标服务不可用");
    let _task =
        super::super::update(&mut app, SettingsMessage::AgentApps(AgentAppsMessage::Refresh));
    assert!(app.agent_apps.loading);
    assert!(app.agent_apps.error.is_none());
    let _task = super::super::update(
        &mut app,
        SettingsMessage::AgentApps(AgentAppsMessage::Loaded {
            context: requested,
            generation,
            result: Err("stale".into()),
        }),
    );
    assert!(app.agent_apps.loading);
    assert!(app.agent_apps.error.is_none());
    assert_eq!(app.active_toast.as_ref().unwrap().message, "图标服务不可用");
}
