use super::VerifiedRouteBinding;
use crate::app_routing::{ListenerError, RoutingListener};
use std::{net::SocketAddr, time::Duration};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
    time::timeout,
};
use vw_api_types::{
    agent_apps::AppId,
    id::{ProjectScope, TenantScope},
};

fn binding(app: AppId) -> VerifiedRouteBinding {
    VerifiedRouteBinding {
        project: ProjectScope::new(
            TenantScope::new("platform", "company").unwrap(),
            "project".into(),
        )
        .unwrap(),
        host: "host".into(),
        app,
        account: "account".into(),
        account_generation: 1,
    }
}

async fn assert_response(address: SocketAddr, request: &str, status: &str, case: &str) {
    let response = timeout(Duration::from_secs(5), async {
        let mut connection = TcpStream::connect(address).await.unwrap();
        connection.write_all(request.as_bytes()).await.unwrap();
        let mut response = Vec::new();
        connection.read_to_end(&mut response).await.unwrap();
        response
    })
    .await
    .unwrap_or_else(|_| panic!("{case}: listener waited for the unsent body"));
    let response = String::from_utf8(response).unwrap();
    let status_line = response.lines().next().unwrap_or_default();
    assert_eq!(status_line, status, "{case}: expected a final response before sending any body");
}

fn post(target: &str, headers: &str) -> String {
    format!(
        "POST {target} HTTP/1.1\r\n{headers}Content-Length: 1024\r\nExpect: 100-continue\r\n\r\n"
    )
}

#[tokio::test]
async fn cc_route_009_browser_guard_expected_behavior() {
    let listener = RoutingListener::default();
    let address = listener.start_on("127.0.0.1", 0).unwrap();
    let issued = listener.auth.issue(binding(AppId::Claude)).unwrap();
    let native = format!("Host: {address}\r\nAuthorization: Bearer {}\r\n", issued.token);

    for origin in ["https://evil.example", "null", ""] {
        let request = post("/v1/messages", &format!("{native}Origin: {origin}\r\n"));
        assert_response(address, &request, "HTTP/1.1 403 Forbidden", "browser POST").await;
    }
    let websocket = format!(
        "GET /v1/responses HTTP/1.1\r\n{native}Origin: https://evil.example\r\nConnection: Upgrade\r\nUpgrade: websocket\r\n\r\n"
    );
    assert_response(address, &websocket, "HTTP/1.1 403 Forbidden", "browser upgrade").await;
    listener.stop().await.unwrap();
}

#[tokio::test]
async fn cc_route_009_browser_guard_rejects_rebinding_and_forged_authority() {
    let listener = RoutingListener::default();
    let address = listener.start_on("127.0.0.1", 0).unwrap();
    let issued = listener.auth.issue(binding(AppId::Claude)).unwrap();
    let auth = format!("Authorization: Bearer {}\r\n", issued.token);
    let forged = [
        format!("Host: rebind.evil.example:{}\r\n", address.port()),
        format!("Host: localhost.evil.example:{}\r\n", address.port()),
        "Host: 192.0.2.10\r\n".into(),
        "Host: localhost:0\r\n".into(),
        "Host: localhost:bad\r\n".into(),
        "Host: user@localhost\r\n".into(),
        "Host: localhost\r\nHost: localhost\r\n".into(),
        String::new(),
    ];
    for host in forged {
        let headers = format!("{host}{auth}X-Forwarded-Host: localhost\r\n");
        let request = post("/v1/messages", &headers);
        assert_response(address, &request, "HTTP/1.1 403 Forbidden", &format!("{host:?}")).await;
    }
    let request =
        post("http://rebind.evil.example/v1/messages", &format!("Host: {address}\r\n{auth}"));
    assert_response(address, &request, "HTTP/1.1 403 Forbidden", "absolute request target").await;
    listener.stop().await.unwrap();
}

#[tokio::test]
async fn cc_route_009_browser_guard_requires_a_live_desktop_credential_for_origin() {
    let listener = RoutingListener::default();
    let address = listener.start_on("127.0.0.1", 0).unwrap();
    let native = listener.auth.issue(binding(AppId::Claude)).unwrap();
    let desktop = listener.auth.issue(binding(AppId::ClaudeDesktop)).unwrap();
    let local = format!("Host: {address}\r\nOrigin: app://claude\r\n");
    let desktop_auth = format!("x-api-key: {}\r\n", desktop.token);
    let target = "/claude-desktop/v1/messages";

    for (headers, status, case) in [
        (local.clone(), "HTTP/1.1 401 Unauthorized", "missing credential"),
        (
            format!("{local}x-api-key: fixture-management-skey\r\n"),
            "HTTP/1.1 401 Unauthorized",
            "non-route credential",
        ),
        (
            format!("{local}Authorization: Bearer {}\r\n", native.token),
            "HTTP/1.1 403 Forbidden",
            "other app's route credential",
        ),
        (
            format!("{local}{desktop_auth}"),
            "HTTP/1.1 503 Service Unavailable",
            "valid Desktop route credential passed the guard",
        ),
        (
            format!("Host: rebind.evil.example\r\nOrigin: app://claude\r\n{desktop_auth}"),
            "HTTP/1.1 403 Forbidden",
            "Desktop credential with a forged Host",
        ),
    ] {
        assert_response(address, &post(target, &headers), status, case).await;
    }
    assert_response(
        address,
        &post("/v1/messages", &format!("{local}{desktop_auth}")),
        "HTTP/1.1 403 Forbidden",
        "Desktop credential outside Desktop namespace",
    )
    .await;
    listener.auth.revoke(&desktop.revocation);
    assert_response(
        address,
        &post(target, &format!("{local}{desktop_auth}")),
        "HTTP/1.1 401 Unauthorized",
        "revoked Desktop credential",
    )
    .await;
    listener.stop().await.unwrap();
}

#[tokio::test]
async fn cc_route_009_browser_guard_preserves_local_access_and_authorization() {
    for ip in ["127.0.0.1", "::1"] {
        let listener = RoutingListener::default();
        assert!(matches!(listener.start_on("0.0.0.0", 0), Err(ListenerError::NonLoopbackDenied)));
        let address = listener.start_on(ip, 0).unwrap();
        let issued = listener.auth.issue(binding(AppId::Claude)).unwrap();
        let auth = format!("Authorization: Bearer {}\r\n", issued.token);
        for host in [address.to_string(), format!("LOCALHOST.:{}", address.port())] {
            let headers = format!("Host: {host}\r\n");
            assert_response(
                address,
                &post("/v1/messages", &headers),
                "HTTP/1.1 401 Unauthorized",
                "local host still requires a route credential",
            )
            .await;
            assert_response(
                address,
                &post("/v1/messages", &format!("{headers}{auth}")),
                "HTTP/1.1 503 Service Unavailable",
                "authenticated local host passed the guard",
            )
            .await;
        }
        assert_response(
            address,
            &format!("GET /v1/models HTTP/1.0\r\n{auth}\r\n"),
            "HTTP/1.0 503 Service Unavailable",
            "legacy request without a Host header",
        )
        .await;
        listener.stop().await.unwrap();
    }
}
