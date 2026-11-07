use super::desktop;
use crate::app::state::{
    AgentAppsContext,
    app_providers::{desktop::Form, list::Scope},
};
use std::io::{Read, Write};
use vw_gateway_client::{GatewayClient, GatewayEndpoint, vw_api_types::agent_apps::AppId};
fn fixture(
    path: &'static str,
    status: u16,
    response: &'static str,
) -> (GatewayClient, Scope, std::thread::JoinHandle<Option<serde_json::Value>>) {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let client = GatewayClient::new(GatewayEndpoint::new(
        "127.0.0.1",
        listener.local_addr().unwrap().port(),
    ))
    .unwrap();
    let scope = Scope {
        context: AgentAppsContext {
            gateway: client.endpoint().base_url(),
            tenant: client.endpoint().tenant_scope().clone(),
            project: None,
        },
        host: "fixture-host".into(),
        app: AppId::ClaudeDesktop,
    };
    let server = std::thread::spawn(move || {
        listener.set_nonblocking(true).unwrap();
        let start = std::time::Instant::now();
        let mut stream = loop {
            match listener.accept() {
                Ok((stream, _)) => break stream,
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    if start.elapsed().as_secs() >= 3 {
                        return None;
                    }
                    std::thread::sleep(std::time::Duration::from_millis(10));
                }
                Err(e) => panic!("{e}"),
            }
        };
        stream.set_read_timeout(Some(std::time::Duration::from_secs(3))).unwrap();
        let mut head = Vec::new();
        while !head.ends_with(b"\r\n\r\n") {
            let mut byte = [0];
            stream.read_exact(&mut byte).unwrap();
            head.push(byte[0]);
        }
        let head = String::from_utf8(head).unwrap();
        assert!(head.starts_with(path), "{head}");
        assert!(head.to_lowercase().contains("x-agent-apps-token: fake-host-token"));
        let len = head
            .lines()
            .find_map(|l| {
                l.to_lowercase()
                    .strip_prefix("content-length:")
                    .map(|s| s.trim().parse::<usize>().unwrap())
            })
            .unwrap();
        let mut bytes = vec![0; len];
        stream.read_exact(&mut bytes).unwrap();
        let value = serde_json::from_slice(&bytes).unwrap();
        write!(stream,"HTTP/1.1 {status} Result\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{response}",response.len()).unwrap();
        Some(value)
    });
    (client, scope, server)
}
#[tokio::test]
async fn cc04_ui_desktop_mapping_validation_is_visible() {
    let (client, scope, server) =
        fixture("PUT /v1/app-providers/claude-desktop/fixture/model-mapping ", 422, "{}");
    let mut form = Form::new("fixture".into());
    form.mapping.routes.insert(
        "bad-route".into(),
        vw_gateway_client::vw_api_types::app_providers::desktop_mapping::DesktopModelRoute {
            model: "invalid model".into(),
            label_override: Some("中文动态标签".into()),
            supports_1m: false,
        },
    );
    form.busy = true;
    let result = desktop::save(client, scope, form.clone(), "fake-host-token".into()).await;
    let body = server.join().unwrap().expect("映射必须通过 SDK 送达执行端校验");
    assert_eq!(body["targetHostId"], "fixture-host");
    assert_eq!(body["mapping"]["routes"]["bad-route"]["labelOverride"], "中文动态标签");
    form.finish(result);
    assert!(form.notice.as_deref().unwrap().contains("兼容校验"));
    assert_eq!(form.mapping.routes["bad-route"].model, "invalid model");
    assert!(!form.busy);
}
#[tokio::test]
async fn cc04_ui_desktop_import_uses_compatibility_api() {
    let (client, scope, server) = fixture(
        "POST /v1/app-providers/claude-desktop/providers/import-claude ",
        200,
        r#"{"imported":2}"#,
    );
    let result = desktop::import(client, scope, "fake-host-token".into()).await;
    let body = server.join().unwrap().expect("导入必须调用兼容性 API");
    assert_eq!(body, serde_json::json!({"targetHostId":"fixture-host"}));
    let notice = result.unwrap();
    assert!(notice.contains('2'));
    assert!(notice.contains("未激活"));
}
