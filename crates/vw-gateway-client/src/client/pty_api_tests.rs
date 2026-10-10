//! PTY HTTP 生命周期与 WebSocket 握手协议测试。

#![allow(clippy::result_large_err)]

use std::net::TcpListener;
use std::sync::mpsc;

use serde_json::json;
use tokio_tungstenite::tungstenite::handshake::server::{Request, Response};
use vw_api_types::id::TenantScope;

use super::{PtyCreateRequest, PtySizeDto, PtyStatusDto, PtyUpdateRequest};
use crate::client::test_support;
use crate::{GatewayAuth, GatewayClient, GatewayEndpoint};

#[tokio::test]
async fn pty_api_uses_http_and_websocket_routes() {
    let server = test_support::server(vec![
        (200, json!([])),
        (200, pty_info("pty-created", "running")),
        (200, pty_info("pty /one", "running")),
        (200, pty_info("pty /one", "exited")),
        (200, json!(true)),
    ]);

    assert!(server.client().pty_list(Some("/tmp/project root")).await.expect("list").is_empty());
    let created = server
        .client()
        .pty_create(
            &PtyCreateRequest {
                command: Some("bash".to_string()),
                args: Some(vec!["-lc".to_string(), "printf ok".to_string()]),
                cwd: Some("/tmp/project root".to_string()),
                title: Some("build shell".to_string()),
                env: Some(std::collections::HashMap::from([(
                    "TERM".to_string(),
                    "xterm-256color".to_string(),
                )])),
            },
            Some("/tmp/project root"),
        )
        .await
        .expect("create");
    assert_eq!(created.id, "pty-created");
    let detail =
        server.client().pty_get("pty /one", Some("/tmp/project root")).await.expect("detail");
    assert_eq!(detail.status, PtyStatusDto::Running);
    let updated = server
        .client()
        .pty_update(
            "pty /one",
            &PtyUpdateRequest {
                title: Some("renamed".to_string()),
                size: Some(PtySizeDto { rows: 32, cols: 120 }),
            },
            Some("/tmp/project root"),
        )
        .await
        .expect("update");
    assert_eq!(updated.status, PtyStatusDto::Exited);
    assert!(
        server.client().pty_delete("pty /one", Some("/tmp/project root")).await.expect("delete")
    );

    let requests = [
        server.take_request(),
        server.take_request(),
        server.take_request(),
        server.take_request(),
        server.take_request(),
    ];
    assert_eq!(requests[0].method, "GET");
    assert_eq!(requests[0].path, "/v1/pty?directory=%2Ftmp%2Fproject+root");
    assert_eq!(requests[1].method, "POST");
    assert_eq!(requests[1].path, "/v1/pty?directory=%2Ftmp%2Fproject+root");
    assert_eq!(
        requests[1].body,
        json!({
            "command":"bash",
            "args":["-lc", "printf ok"],
            "cwd":"/tmp/project root",
            "title":"build shell",
            "env":{"TERM":"xterm-256color"}
        })
    );
    assert_eq!(requests[2].method, "GET");
    assert_eq!(requests[2].path, "/v1/pty/pty%20%2Fone?directory=%2Ftmp%2Fproject+root");
    assert_eq!(requests[3].method, "PUT");
    assert_eq!(requests[3].path, "/v1/pty/pty%20%2Fone?directory=%2Ftmp%2Fproject+root");
    assert_eq!(requests[3].body, json!({"title":"renamed","size":{"rows":32,"cols":120}}));
    assert_eq!(requests[4].method, "DELETE");
    assert_eq!(requests[4].path, "/v1/pty/pty%20%2Fone?directory=%2Ftmp%2Fproject+root");
    server.join();

    let (endpoint, handshake) = websocket_server(false);
    let client = GatewayClient::new(
        endpoint
            .with_tenant_scope(TenantScope::new("platform-a", "company-b").expect("scope"))
            .with_auth(GatewayAuth { skey: Some("secret-token".to_string()) }),
    )
    .expect("websocket client");
    let mut connection = client
        .pty_connect("pty /one", Some("/tmp/project root"), Some(42))
        .await
        .expect("websocket connect");
    let event = connection.next_event().await.expect("close event").expect("server close frame");
    assert!(event.is_closed());

    let request = handshake.recv().expect("handshake request");
    assert_eq!(
        request.path,
        "/v1/pty/pty%20%2Fone/connect?directory=%2Ftmp%2Fproject+root&cursor=42"
    );
    assert_eq!(request.platform.as_deref(), Some("platform-a"));
    assert_eq!(request.company.as_deref(), Some("company-b"));
    assert_eq!(request.authorization.as_deref(), Some("Bearer secret-token"));

    let (endpoint, rejected_handshake) = websocket_server(true);
    let client = GatewayClient::new(endpoint).expect("rejected websocket client");
    let error = match client.pty_connect("pty-rejected", None, None).await {
        Ok(_) => panic!("failed WebSocket handshake must propagate"),
        Err(error) => error,
    };
    assert!(error.contains("403"), "unexpected handshake error: {error}");
    assert_eq!(
        rejected_handshake.recv().expect("rejected handshake request").path,
        "/v1/pty/pty-rejected/connect"
    );
}

fn pty_info(id: &str, status: &str) -> serde_json::Value {
    json!({
        "id": id,
        "title": "terminal",
        "command": "bash",
        "args": [],
        "cwd": "/tmp/project root",
        "status": status,
        "pid": 123
    })
}

#[derive(Debug)]
struct HandshakeRequest {
    path: String,
    platform: Option<String>,
    company: Option<String>,
    authorization: Option<String>,
}

fn websocket_server(reject: bool) -> (GatewayEndpoint, mpsc::Receiver<HandshakeRequest>) {
    let listener = TcpListener::bind(("127.0.0.1", 0)).expect("bind websocket server");
    let port = listener.local_addr().expect("websocket address").port();
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let (stream, _) = listener.accept().expect("accept websocket");
        let callback = move |request: &Request, response: Response| {
            tx.send(HandshakeRequest {
                path: request.uri().to_string(),
                platform: header(request, "x-platform-id"),
                company: header(request, "x-company-id"),
                authorization: header(request, "authorization"),
            })
            .expect("record handshake");
            if reject {
                Err(tokio_tungstenite::tungstenite::http::Response::builder()
                    .status(403)
                    .body(Some("denied".to_string()))
                    .expect("rejected handshake response"))
            } else {
                Ok(response)
            }
        };
        let accepted = tokio_tungstenite::tungstenite::accept_hdr(stream, callback);
        if reject {
            assert!(accepted.is_err(), "rejected handshake must not upgrade");
        } else {
            accepted.expect("accept websocket handshake").close(None).expect("close websocket");
        }
    });
    (GatewayEndpoint::new("127.0.0.1", port), rx)
}

fn header(request: &Request, name: &str) -> Option<String> {
    request.headers().get(name).and_then(|value| value.to_str().ok()).map(str::to_string)
}
