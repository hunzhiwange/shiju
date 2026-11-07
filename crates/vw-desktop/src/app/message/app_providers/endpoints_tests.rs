use super::endpoints;
use crate::app::state::{
    AgentAppsContext,
    app_providers::{endpoints::Form, list::Scope},
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
async fn cc04_ui_endpoints_invalid_response_stays_in_modal() {
    let (client, scope, server) =
        fixture("POST /v1/app-providers/claude-desktop/fixture/endpoints ", 422, "{}");
    let mut form = Form::new("fixture".into());
    form.url = "非法地址中文".into();
    form.busy = true;
    let result = endpoints::add(client, scope, form.clone()).await;
    let body = server.join().unwrap().expect("校验应由真实 SDK 请求执行端");
    assert_eq!(body["url"], "非法地址中文");
    form.finish(result);
    assert_eq!(form.url, "非法地址中文");
    assert!(form.notice.as_deref().unwrap().contains("422"));
    assert!(!form.busy);
}
#[tokio::test]
async fn cc04_ui_endpoints_submit_keeps_full_url_choice() {
    let (client, scope, server) = fixture(
        "POST /v1/app-providers/claude-desktop/fixture/endpoints ",
        200,
        r#"[{"url":"https://fixture.example/v1/messages/","kind":"fullUrl","addedAt":42,"lastUsed":43}]"#,
    );
    let mut form = Form::new("fixture".into());
    form.url = "https://fixture.example/v1/messages/".into();
    form.full_url = true;
    let result = endpoints::add(client, scope, form.clone()).await;
    let body = server.join().unwrap().expect("端点必须经 SDK 提交");
    assert_eq!(body["kind"], "fullUrl");
    assert_eq!(body["url"], form.url);
    form.finish(result);
    assert_eq!(form.rows[0].url, form.url);
    assert_eq!(form.rows[0].last_used, Some(43));
}

#[tokio::test]
async fn cc04_ui_endpoints_json_field_cannot_inject_sibling_options() {
    let client = GatewayClient::new(GatewayEndpoint::new("127.0.0.1", 1)).unwrap();
    let scope = Scope {
        context: AgentAppsContext {
            gateway: client.endpoint().base_url(),
            tenant: client.endpoint().tenant_scope().clone(),
            project: None,
        },
        host: "fixture-host".into(),
        app: AppId::Claude,
    };
    let mut form = Form::new("fixture".into());
    form.headers = r#"{},"apiKeyField":"ANTHROPIC_API_KEY""#.into();
    let result = endpoints::save_options(client, scope, form.clone(), "fake-token".into()).await;
    assert!(result.unwrap_err().contains("单个 JSON 对象"));
    assert_eq!(form.headers, r#"{},"apiKeyField":"ANTHROPIC_API_KEY""#);
}
