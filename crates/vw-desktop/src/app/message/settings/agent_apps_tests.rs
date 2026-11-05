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

fn rename_app() -> App {
    let mut app = App::new().0;
    let client = crate::app::config::gateway_client().unwrap();
    let requested = context(&client, app.project_path.clone());
    let generation = app.agent_apps.begin(requested.clone());
    app.agent_apps.finish(
        &requested,
        &requested,
        generation,
        Ok(vec![CustomIconEntry {
            id: "icon".into(),
            name: "工作".into(),
            data: String::new(),
            created_at: 0,
        }]),
    );
    let _ = update(&mut app, AgentAppsMessage::RenameOpen("icon".into()));
    app
}

#[test]
fn cc_base_icon_desktop_rename_close_does_not_submit() {
    let mut app = rename_app();
    assert_eq!(app.agent_apps.rename.as_ref().unwrap().name, "工作");
    assert_eq!(update(&mut app, AgentAppsMessage::RenameClose).units(), 0);
    assert!(app.agent_apps.rename.is_none());
    assert!(!app.agent_apps.loading);
    assert_eq!(app.agent_apps.icons[0].name, "工作");
}

#[test]
fn cc_base_icon_desktop_rename_failure_preserves_input_and_blocks_duplicate() {
    let mut app = rename_app();
    let name = "中".repeat(65);
    let _ = update(&mut app, AgentAppsMessage::RenameInput(name.clone()));
    assert_eq!(update(&mut app, AgentAppsMessage::RenameSubmit).units(), 1);
    let pending = app.agent_apps.rename.clone().unwrap();
    assert!(pending.saving);
    assert_eq!(update(&mut app, AgentAppsMessage::RenameSubmit).units(), 0);
    assert_eq!(update(&mut app, AgentAppsMessage::RenameClose).units(), 0);
    assert!(app.agent_apps.rename.is_some());
    let _ = update(&mut app, AgentAppsMessage::RenameInput("changed".into()));
    assert_eq!(app.agent_apps.rename.as_ref().unwrap().name, name);
    let _ = update(
        &mut app,
        AgentAppsMessage::Renamed {
            context: pending.context,
            generation: pending.generation,
            result: Err("网络错误".into()),
        },
    );
    let form = app.agent_apps.rename.as_ref().unwrap();
    assert!(!form.saving);
    assert_eq!(form.name, name);
    assert_eq!(form.error.as_deref(), Some("网络错误"));
    assert_eq!(app.active_toast.as_ref().unwrap().message, "网络错误");
}

#[test]
fn cc_base_icon_desktop_rename_success_rereads_and_stale_response_is_ignored() {
    let mut app = rename_app();
    let _ = update(&mut app, AgentAppsMessage::RenameInput("中".repeat(64)));
    let _ = update(&mut app, AgentAppsMessage::RenameSubmit);
    let pending = app.agent_apps.rename.clone().unwrap();
    let mut stale = pending.context.clone();
    stale.project = Some("/other".into());
    let _ = update(
        &mut app,
        AgentAppsMessage::Renamed {
            context: stale,
            generation: pending.generation,
            result: Ok(()),
        },
    );
    assert!(app.agent_apps.rename.as_ref().unwrap().saving);
    let _ = update(
        &mut app,
        AgentAppsMessage::Renamed {
            context: pending.context,
            generation: pending.generation,
            result: Ok(()),
        },
    );
    assert!(app.agent_apps.rename.is_none());
    assert!(app.agent_apps.loading);
    assert_eq!(app.agent_apps.icons[0].name, "工作");
}

/// Only the remote HTTP transport is substituted; verify SDK serialization and re-read.
#[tokio::test]
async fn cc_base_icon_desktop_rename_sdk_chinese_64_65_and_reread() {
    use std::io::{Read, Write};
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    listener.set_nonblocking(true).unwrap();
    let server = std::thread::spawn(move || {
        let entry =
            serde_json::json!({"id":"icon", "name":"中".repeat(64), "data":"", "createdAt":0});
        for index in 0..3 {
            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(15);
            let mut stream = loop {
                match listener.accept() {
                    Ok((stream, _)) => break stream,
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        assert!(std::time::Instant::now() < deadline, "SDK request timed out");
                        std::thread::sleep(std::time::Duration::from_millis(5));
                    }
                    Err(error) => panic!("{error}"),
                }
            };
            stream.set_read_timeout(Some(std::time::Duration::from_secs(10))).unwrap();
            let mut headers = Vec::new();
            while !headers.ends_with(b"\r\n\r\n") {
                let mut byte = [0];
                assert_eq!(stream.read(&mut byte).unwrap(), 1);
                headers.push(byte[0]);
            }
            let headers = String::from_utf8(headers).unwrap().to_lowercase();
            assert!(headers.contains("x-company-id: company"));
            let length: usize = headers
                .lines()
                .find_map(|line| line.strip_prefix("content-length:").map(str::trim))
                .unwrap_or("0")
                .parse()
                .unwrap();
            let mut body = vec![0; length];
            stream.read_exact(&mut body).unwrap();
            let (status, response) = if index == 1 {
                assert!(headers.starts_with("get /v1/agent-apps/custom-icons "));
                ("200 OK", serde_json::json!([entry]).to_string())
            } else {
                assert!(headers.starts_with("patch /v1/agent-apps/custom-icons/icon "));
                let body: serde_json::Value = serde_json::from_slice(&body).unwrap();
                assert_eq!(body["name"], "中".repeat(if index == 0 { 64 } else { 65 }));
                if index == 0 {
                    ("200 OK", entry.to_string())
                } else {
                    ("400 Bad Request", r#"{"error":"名称过长"}"#.into())
                }
            };
            write!(stream, "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{response}", response.len()).unwrap();
        }
    });
    let client = GatewayClient::new(
        GatewayEndpoint::new("127.0.0.1", port)
            .with_tenant_scope(TenantScope::new("platform", "company").unwrap()),
    )
    .unwrap();
    let mut form = IconRenameState {
        id: "icon".into(),
        name: "中".repeat(64),
        saving: true,
        error: None,
        context: context(&client, None),
        generation: 1,
    };
    rename(client.clone(), form.clone()).await.unwrap();
    assert_eq!(load(Ok(client.clone())).await.unwrap()[0].name, form.name);
    form.name.push('中');
    assert!(rename(client, form).await.is_err());
    server.join().unwrap();
}

#[test]
fn cc_base_icon_desktop_rename_context_change_never_writes_or_applies() {
    let mut app = rename_app();
    let pending = app.agent_apps.rename.clone().unwrap();
    app.project_path = Some("/different-project".into());
    let _ = update(&mut app, AgentAppsMessage::RenameSubmit);
    assert!(app.agent_apps.rename.is_none());
    assert!(app.agent_apps.loading);
    let _ = update(
        &mut app,
        AgentAppsMessage::Renamed {
            context: pending.context,
            generation: pending.generation,
            result: Err("stale".into()),
        },
    );
    assert!(app.agent_apps.error.is_none());
}
