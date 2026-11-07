use super::*;
use crate::app::state::{AgentAppsContext, app_providers::list::ListState};
use vw_gateway_client::{GatewayEndpoint, vw_api_types::agent_apps::AppId};
fn scope() -> Scope {
    let client = GatewayClient::new(GatewayEndpoint::new("127.0.0.1", 1)).unwrap();
    Scope {
        context: AgentAppsContext {
            gateway: client.endpoint().base_url(),
            tenant: client.endpoint().tenant_scope().clone(),
            project: None,
        },
        host: "fixture".into(),
        app: AppId::Claude,
    }
}
#[test]
fn cc04_ui_list_stale_response_is_ignored() {
    let mut state = ListState::default();
    let old = scope();
    let generation = state.begin(old.clone());
    let mut new = old.clone();
    new.app = AppId::Pi;
    state.begin(new.clone());
    assert!(!state.finish(&old, generation, Err("过期错误".into())));
    assert_eq!(state.scope, Some(new));
    assert!(state.loading);
    assert!(state.error.is_none());
}
#[tokio::test]
async fn cc04_ui_list_sort_uses_sdk() {
    use std::io::{Read, Write};
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let port = listener.local_addr().unwrap().port();
    let server = std::thread::spawn(move || {
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(3);
        loop {
            match listener.accept() {
                Ok((mut stream, _)) => {
                    stream.set_read_timeout(Some(std::time::Duration::from_secs(3))).unwrap();
                    let mut headers = Vec::new();
                    while !headers.ends_with(b"\r\n\r\n") {
                        let mut b = [0];
                        stream.read_exact(&mut b).unwrap();
                        headers.push(b[0]);
                    }
                    let head = String::from_utf8(headers).unwrap();
                    assert!(head.starts_with("PUT /v1/app-providers/claude/one/presentation "));
                    let len: usize = head
                        .lines()
                        .find_map(|l| {
                            l.to_lowercase()
                                .strip_prefix("content-length:")
                                .map(|n| n.trim().parse().unwrap())
                        })
                        .unwrap();
                    let mut body = vec![0; len];
                    stream.read_exact(&mut body).unwrap();
                    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
                    assert_eq!(json["sortIndex"], 7);
                    assert_eq!(json["category"], "工作");
                    assert_eq!(json["notes"], "中文备注");
                    let body = serde_json::to_string(&json).unwrap();
                    write!(stream,"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",body.len(),body).unwrap();
                    return true;
                }
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    if std::time::Instant::now() > deadline {
                        return false;
                    }
                    std::thread::sleep(std::time::Duration::from_millis(10));
                }
                Err(e) => panic!("{e}"),
            }
        }
    });
    let value = AppProviderPresentation {
        sort_index: Some(7),
        category: Some("工作".into()),
        notes: Some("中文备注".into()),
    };
    let result = save(
        GatewayClient::new(GatewayEndpoint::new("127.0.0.1", port)).unwrap(),
        scope(),
        "one".into(),
        value.clone(),
    )
    .await
    .unwrap();
    assert_eq!(result, value);
    assert!(server.join().unwrap(), "排序未通过真实 SDK 请求保存");
}

#[test]
fn cc04_ui_list_scope_and_generation_reject_late_failures() {
    let mut state = ListState::default();
    let initial = scope();
    let first = state.begin(initial.clone());
    let second = state.begin(initial.clone());
    assert!(!state.finish(&initial, first, Err("旧刷新".into())));
    let mut foreign = initial.clone();
    foreign.host = "another-host".into();
    assert!(!state.finish(&foreign, second, Err("旧主机".into())));
    foreign = initial.clone();
    foreign.context.project = Some("another-project".into());
    assert!(!state.finish(&foreign, second, Err("旧项目".into())));
    assert!(state.finish(&initial, second, Err("读取失败".into())));
    assert!(!state.loading);
    assert_eq!(state.error.as_deref(), Some("读取失败"));
    assert!(!state.finish(&initial, second, Err("重复响应".into())));
}

#[test]
fn cc04_ui_list_current_and_membership_remain_service_observations() {
    use vw_gateway_client::vw_api_types::app_providers::catalog::*;
    let mut state = ListState::default();
    let requested = scope();
    let generation = state.begin(requested.clone());
    let catalog = ProviderCatalog {
        app_id: AppId::Claude,
        rows: vec![],
        direct: ProviderSelection::Selected { id: "managed".into() },
        in_use: ProviderSelection::Unavailable {
            reason: ProviderReadUnavailable::InvalidConfiguration,
        },
        native_read: Some(ProviderReadUnavailable::InvalidConfiguration),
    };
    assert!(state.finish(
        &requested,
        generation,
        Ok(Snapshot { catalog: catalog.clone(), presentation: vec![] })
    ));
    assert_eq!(state.snapshot.unwrap().catalog, catalog);
}
