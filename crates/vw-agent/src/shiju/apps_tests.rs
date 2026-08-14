use std::io::{Read, Write};
use std::net::TcpListener;
use std::thread;

use super::*;

const CREDENTIAL: &str = "fixture-app-credential-secret";
const INSTALLATION_ID: &str = "01K1APP0000000000000000001";

struct FixtureResponse {
    status: u16,
    headers: Vec<(&'static str, String)>,
    body: String,
}

fn fixture(responses: Vec<FixtureResponse>) -> (String, thread::JoinHandle<Vec<String>>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind SHIJU apps fixture");
    let address = listener.local_addr().expect("fixture address");
    let handle = thread::spawn(move || {
        responses
            .into_iter()
            .map(|response| {
                let (mut stream, _) = listener.accept().expect("accept SHIJU apps request");
                let mut request = Vec::new();
                let mut buffer = [0_u8; 4096];
                loop {
                    let read = stream.read(&mut buffer).expect("read SHIJU apps request");
                    request.extend_from_slice(&buffer[..read]);
                    if read == 0 || request.windows(4).any(|window| window == b"\r\n\r\n") {
                        break;
                    }
                }
                let reason = match response.status {
                    200 => "OK",
                    204 => "No Content",
                    _ => "Internal Server Error",
                };
                let headers = response
                    .headers
                    .into_iter()
                    .map(|(name, value)| format!("{name}: {value}\r\n"))
                    .collect::<String>();
                let wire = format!(
                    "HTTP/1.1 {} {reason}\r\nContent-Type: application/json\r\n{headers}Content-Length: {}\r\nConnection: close\r\n\r\n{}",
                    response.status,
                    response.body.len(),
                    response.body
                );
                stream.write_all(wire.as_bytes()).expect("write SHIJU apps response");
                String::from_utf8(request).expect("request UTF-8")
            })
            .collect()
    });
    (format!("http://{address}/app-platform/api/"), handle)
}

fn ok(items: serde_json::Value) -> FixtureResponse {
    FixtureResponse {
        status: 200,
        headers: Vec::new(),
        body: serde_json::json!({"$schema": "fixture", "items": items}).to_string(),
    }
}

fn installed_app(
    installation_id: &str,
    reconcile_status: &str,
    runtime_status: &str,
    runtime_url: &str,
) -> serde_json::Value {
    serde_json::json!({
        "installationId": installation_id,
        "appId": "knowledge-assistant",
        "title": "知识助手",
        "version": "1.2.3",
        "origin": "catalog",
        "reconcileStatus": reconcile_status,
        "runtimeStatus": runtime_status,
        "runtimeUrl": runtime_url,
        "installedAt": "2026-07-30T09:30:00+08:00",
        "updatedAt": "2026-07-30T09:31:00+08:00"
    })
}

fn assert_api_key(request: &str) {
    let api_key = request
        .lines()
        .find(|line| line.to_ascii_lowercase().starts_with("x-api-key:"))
        .expect("X-Api-Key header");
    assert_eq!(api_key, format!("x-api-key: {CREDENTIAL}"));
    assert!(!request.lines().any(|line| line.to_ascii_lowercase().starts_with("cookie:")));
}

#[tokio::test(flavor = "multi_thread")]
async fn shiju_apps_adapter_maps_catalog_and_installed_lists() {
    let catalog = serde_json::json!([{
        "catalogAppId": "knowledge-assistant",
        "title": "知识助手",
        "description": "企业知识检索",
        "iconUrl": null,
        "currentListedVersion": "1.2.3",
        "installed": true,
        "installationId": INSTALLATION_ID,
        "installedByUserId": "101027",
        "installedAt": "2026-07-30T09:30:00+08:00",
        "canUse": true,
        "canManage": true
    }]);
    let installed = serde_json::json!([installed_app(
        INSTALLATION_ID,
        "ready",
        "ready",
        &format!("https://app.shiju.cloud/app/{INSTALLATION_ID}/")
    )]);
    let (base, requests) = fixture(vec![
        ok(catalog),
        ok(installed.clone()),
        ok(installed),
        FixtureResponse {
            status: 200,
            headers: Vec::new(),
            body: r#"{"binding":"fixture-sensitive-binding"}"#.to_string(),
        },
        FixtureResponse {
            status: 204,
            headers: vec![(
                "Set-Cookie",
                "app_runtime_session=fixture-runtime-session-secret; Path=/; Max-Age=3600; HttpOnly; Secure; SameSite=None"
                    .to_string(),
            )],
            body: String::new(),
        },
    ]);
    let client = ShijuAppClient::with_base_url(&base).expect("client");

    let catalog = client.catalog(CREDENTIAL).await.expect("catalog");
    let apps = client.installed(CREDENTIAL).await.expect("installed");
    let launch = client.prepare_launch(CREDENTIAL, INSTALLATION_ID).await.expect("prepare launch");

    assert_eq!(catalog.apps[0].app_id, "knowledge-assistant");
    assert!(catalog.apps[0].permissions.is_empty());
    assert_eq!(catalog.apps[0].user_id.as_deref(), Some("101027"));
    assert_eq!(apps.apps[0].source, "catalog");
    assert_eq!(apps.apps[0].updated_at.as_deref(), Some("2026-07-30T09:31:00+08:00"));
    assert_eq!(launch.installation_id, INSTALLATION_ID);
    assert_eq!(launch.name, "知识助手");
    assert!(
        launch
            .runtime_set_cookie
            .starts_with("app_runtime_session=fixture-runtime-session-secret;")
    );
    assert_eq!(launch.runtime_url, format!("https://app.shiju.cloud/app/{INSTALLATION_ID}/"));
    assert!(!format!("{launch:?}").contains("fixture-runtime-session-secret"));

    let requests = requests.join().expect("join fixture");
    assert_eq!(requests.len(), 5);
    assert!(requests[0].starts_with("GET /app-platform/api/catalog HTTP/1.1\r\n"));
    assert!(requests[1].starts_with("GET /app-platform/api/apps HTTP/1.1\r\n"));
    assert!(requests[2].starts_with("GET /app-platform/api/apps HTTP/1.1\r\n"));
    assert!(requests[3].starts_with(&format!(
        "GET /app-platform/api/tenant/installations/{INSTALLATION_ID}/api-key-binding HTTP/1.1\r\n"
    )));
    assert!(requests[4].starts_with("POST /app-platform/api/runtime/browser-session HTTP/1.1\r\n"));
    for request in &requests {
        assert_api_key(request);
        assert!(!request.lines().next().unwrap().contains(CREDENTIAL));
    }
}

async fn assert_prepare_rejected(app: serde_json::Value, installation_id: &str) {
    let (base, requests) = fixture(vec![ok(serde_json::json!([app]))]);
    let client = ShijuAppClient::with_base_url(&base).expect("client");

    let error = client
        .prepare_launch(CREDENTIAL, installation_id)
        .await
        .expect_err("launch must be rejected");

    assert_eq!(error.kind(), ShijuAppErrorKind::InvalidInput);
    let rendered = format!("{error:?} {error}");
    assert!(!rendered.contains(CREDENTIAL));
    assert!(!rendered.contains("session=private"));
    let requests = requests.join().expect("join fixture");
    assert_eq!(requests.len(), 1);
    assert!(requests[0].starts_with("GET /app-platform/api/apps HTTP/1.1\r\n"));
}

#[tokio::test(flavor = "multi_thread")]
async fn shiju_app_prepare_launch_validates_owned_ready_runtime() {
    let (base, requests) = fixture(Vec::new());
    let client = ShijuAppClient::with_base_url(&base).expect("client");
    for invalid_id in ["", "lowercase", "ABC-123", &"A".repeat(65)] {
        let error = client
            .prepare_launch(CREDENTIAL, invalid_id)
            .await
            .expect_err("invalid installation ID");
        assert_eq!(error.kind(), ShijuAppErrorKind::InvalidInput);
    }
    assert!(requests.join().expect("join fixture").is_empty());

    assert_prepare_rejected(
        installed_app(
            INSTALLATION_ID,
            "ready",
            "ready",
            &format!("https://app.shiju.cloud/app/{INSTALLATION_ID}/"),
        ),
        "OTHER123",
    )
    .await;
    for (reconcile, runtime, url) in [
        ("pending", "ready", format!("https://app.shiju.cloud/app/{INSTALLATION_ID}/")),
        ("ready", "stopped", format!("https://app.shiju.cloud/app/{INSTALLATION_ID}/")),
        ("ready", "ready", format!("http://app.shiju.cloud/app/{INSTALLATION_ID}/")),
        ("ready", "ready", format!("https://evil.example/app/{INSTALLATION_ID}/")),
        ("ready", "ready", "https://app.shiju.cloud/app/OTHER123/".to_string()),
        (
            "ready",
            "ready",
            format!("https://app.shiju.cloud/app/{INSTALLATION_ID}/?session=private"),
        ),
        ("ready", "ready", format!("https://app.shiju.cloud/app/{INSTALLATION_ID}/#private")),
    ] {
        assert_prepare_rejected(
            installed_app(INSTALLATION_ID, reconcile, runtime, &url),
            INSTALLATION_ID,
        )
        .await;
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn shiju_apps_adapter_redacts_credentials_and_upstream_bodies() {
    let sensitive_body = format!(
        r#"{{"message":"upstream-body-secret","credential":"{CREDENTIAL}","runtimeUrl":"https://app.shiju.cloud/app/{INSTALLATION_ID}/?session=private"}}"#
    );
    let (base, requests) =
        fixture(vec![FixtureResponse { status: 500, headers: Vec::new(), body: sensitive_body }]);
    let client = ShijuAppClient::with_base_url(&base).expect("client");

    let error = client.catalog(CREDENTIAL).await.expect_err("upstream error");

    assert_eq!(error.kind(), ShijuAppErrorKind::Upstream);
    let rendered = format!("{error:?} {error}");
    for secret in [CREDENTIAL, "upstream-body-secret", "session=private"] {
        assert!(!rendered.contains(secret));
    }
    let requests = requests.join().expect("join fixture");
    assert_api_key(&requests[0]);
    assert!(!requests[0].lines().next().unwrap().contains(CREDENTIAL));
}
