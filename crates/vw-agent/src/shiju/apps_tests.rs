//! 验证始居应用列表、安装状态和请求响应转换行为。
//! 目录、安装、绑定和凭据流程共用伪网关，重点检查所有权、就绪状态与密钥脱敏。

use std::io::{Read, Write};
use std::net::TcpListener;
use std::thread;

use super::*;

const CREDENTIAL: &str = "fixture-app-credential-secret";
const INSTALLATION_ID: &str = "01K1APP0000000000000000001";

#[test]
fn shiju_apps_production_gateway_uses_sj_prefix() {
    assert_eq!(PRODUCTION_APP_BASE_URL, "https://gateway.shiju.cloud/sj/app-platform/api/");
    assert_eq!(PRODUCTION_IAM_BASE_URL, "https://gateway.shiju.cloud/sj/iam/api/v1/");
}

struct FixtureResponse {
    status: u16,
    headers: Vec<(&'static str, String)>,
    body: String,
}

// 目录与绑定用例共享固定网关响应，凭据只在断言中检查存在性而不回显原值。
fn fixture(responses: Vec<FixtureResponse>) -> (String, String, thread::JoinHandle<Vec<String>>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind SHIJU apps fixture");
    let address = listener.local_addr().expect("fixture address");
    let handle = thread::spawn(move || {
        responses
            .into_iter()
            .map(|response| {
                let (mut stream, _) = listener.accept().expect("accept SHIJU apps request");
                let mut request = Vec::new();
                let mut buffer = [0_u8; 4096];
                let mut expected_len = None;
                loop {
                    let read = stream.read(&mut buffer).expect("read SHIJU apps request");
                    request.extend_from_slice(&buffer[..read]);
                    if read == 0 {
                        break;
                    }
                    if let Some(header_end) = request.windows(4).position(|window| window == b"\r\n\r\n") {
                        let header_end = header_end + 4;
                        let content_length = expected_len.get_or_insert_with(|| {
                            String::from_utf8_lossy(&request[..header_end])
                                .lines()
                                .find_map(|line| {
                                    let (name, value) = line.split_once(':')?;
                                    name.eq_ignore_ascii_case("content-length")
                                        .then(|| value.trim().parse::<usize>().ok())
                                        .flatten()
                                })
                                .unwrap_or(0)
                        });
                        if request.len() >= header_end + *content_length {
                            break;
                        }
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
    (format!("http://{address}/app-platform/api/"), format!("http://{address}/iam/api/v1/"), handle)
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

fn binding(configured: bool, can_edit: bool) -> serde_json::Value {
    serde_json::json!({
        "personal": {
            "configured": configured,
            "canEdit": can_edit,
            "apiKeyId": configured.then_some(42),
            "keyName": configured.then_some("Desktop automation"),
            "apiKeyMask": configured.then_some("sk-****cafe"),
            "status": configured.then_some("enabled"),
            "expiresAt": null,
            "updatedAt": configured.then_some("2026-08-22T09:30:00+08:00")
        },
        "internalPolicy": "must-not-leak"
    })
}

fn iam_key(id: &str, status: &str) -> serde_json::Value {
    serde_json::json!({
        "api_key_id": id,
        "key_name": format!("Key {id}"),
        "api_key_mask": format!("sk-****{id}"),
        "status": status,
        "expires_at": null,
        "updated_at": "2026-08-21T09:30:00+08:00",
        "raw_api_key": "must-not-leak"
    })
}

fn iam_keys(keys: serde_json::Value) -> FixtureResponse {
    FixtureResponse {
        status: 200,
        headers: Vec::new(),
        body: serde_json::json!({
            "code": 200,
            "message": "success",
            "data": {"list": keys, "total": 99},
            "traceId": "must-not-leak"
        })
        .to_string(),
    }
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
    let (app_base, iam_base, requests) = fixture(vec![
        ok(catalog),
        ok(installed.clone()),
        ok(installed),
        FixtureResponse {
            status: 200,
            headers: Vec::new(),
            body: binding(true, true).to_string(),
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
    let client = ShijuAppClient::with_base_urls(&app_base, &iam_base).expect("client");

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
    let (app_base, iam_base, requests) = fixture(vec![ok(serde_json::json!([app]))]);
    let client = ShijuAppClient::with_base_urls(&app_base, &iam_base).expect("client");

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
    let (app_base, iam_base, requests) = fixture(Vec::new());
    let client = ShijuAppClient::with_base_urls(&app_base, &iam_base).expect("client");
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
    let (app_base, iam_base, requests) =
        fixture(vec![FixtureResponse { status: 500, headers: Vec::new(), body: sensitive_body }]);
    let client = ShijuAppClient::with_base_urls(&app_base, &iam_base).expect("client");

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

#[tokio::test(flavor = "multi_thread")]
async fn shiju_app_binding_flow_parses_configured_and_unconfigured_records() {
    let (app_base, iam_base, requests) = fixture(vec![
        FixtureResponse { status: 200, headers: Vec::new(), body: binding(true, true).to_string() },
        FixtureResponse {
            status: 200,
            headers: Vec::new(),
            body: binding(false, true).to_string(),
        },
        iam_keys(serde_json::json!([iam_key("42", "enabled"), iam_key("43", "disabled")])),
    ]);
    let client = ShijuAppClient::with_base_urls(&app_base, &iam_base).expect("client");

    let configured = client.api_key_binding(CREDENTIAL, INSTALLATION_ID).await.expect("binding");
    let unconfigured = client.api_key_binding(CREDENTIAL, INSTALLATION_ID).await.expect("binding");
    let keys = client.api_keys(CREDENTIAL).await.expect("IAM keys");

    assert!(configured.personal.configured);
    assert_eq!(configured.personal.api_key_id, Some(42));
    assert_eq!(configured.personal.api_key_mask.as_deref(), Some("sk-****cafe"));
    assert!(!unconfigured.personal.configured);
    assert!(unconfigured.personal.api_key_id.is_none());
    assert_eq!(keys.api_keys.len(), 2);
    assert_eq!(keys.api_keys[0].api_key_id, 42);
    assert_eq!(keys.api_keys[0].status, "enabled");
    let safe = serde_json::to_string(&(configured, unconfigured, keys)).expect("safe DTOs");
    for secret in ["internalPolicy", "raw_api_key", "traceId", "must-not-leak"] {
        assert!(!safe.contains(secret));
    }

    let requests = requests.join().expect("join fixture");
    assert_eq!(requests.len(), 3);
    assert!(requests[0].starts_with(&format!(
        "GET /app-platform/api/tenant/installations/{INSTALLATION_ID}/api-key-binding "
    )));
    assert!(requests[1].starts_with(&format!(
        "GET /app-platform/api/tenant/installations/{INSTALLATION_ID}/api-key-binding "
    )));
    assert!(requests[2].starts_with("GET /iam/api/v1/api-keys "));
    for request in requests {
        assert_api_key(&request);
    }
}

async fn save_binding_result(
    binding_response: serde_json::Value,
    keys: serde_json::Value,
    api_key_id: u64,
    include_put: bool,
) -> (Result<(), ShijuAppError>, Vec<String>) {
    let mut responses = vec![FixtureResponse {
        status: 200,
        headers: Vec::new(),
        body: binding_response.to_string(),
    }];
    if keys != serde_json::Value::Null {
        responses.push(iam_keys(keys));
    }
    if include_put {
        responses.push(FixtureResponse { status: 204, headers: Vec::new(), body: String::new() });
    }
    let (app_base, iam_base, requests) = fixture(responses);
    let client = ShijuAppClient::with_base_urls(&app_base, &iam_base).expect("client");
    let result = client.save_api_key_binding(CREDENTIAL, INSTALLATION_ID, api_key_id).await;
    (result, requests.join().expect("join fixture"))
}

#[tokio::test(flavor = "multi_thread")]
// 启动前重新读取绑定状态，避免使用已撤销、未启用或不属于当前账号的旧密钥。
async fn shiju_app_binding_save_reloads_and_accepts_only_enabled_owned_key() {
    let (result, requests) = save_binding_result(
        binding(false, true),
        serde_json::json!([iam_key("42", "enabled"), iam_key("43", "disabled")]),
        42,
        true,
    )
    .await;
    result.expect("enabled owned key is accepted");
    assert_eq!(requests.len(), 3);
    assert!(requests[0].starts_with(&format!(
        "GET /app-platform/api/tenant/installations/{INSTALLATION_ID}/api-key-binding "
    )));
    assert!(requests[1].starts_with("GET /iam/api/v1/api-keys "));
    assert!(requests[2].starts_with(&format!(
        "PUT /app-platform/api/tenant/installations/{INSTALLATION_ID}/api-key-binding "
    )));
    let body = requests[2].split_once("\r\n\r\n").expect("PUT body").1;
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(body).expect("PUT JSON"),
        serde_json::json!({
            "apiKeyId": 42,
            "keyName": "Key 42",
            "apiKeyMask": "sk-****42",
            "status": "enabled",
            "expiresAt": null,
            "target": "personal"
        })
    );
    for request in &requests {
        assert_api_key(request);
    }

    for (binding_response, keys, id, expected_requests) in [
        (binding(false, false), serde_json::Value::Null, 42, 1),
        (binding(false, true), serde_json::json!([iam_key("43", "disabled")]), 43, 2),
        (binding(false, true), serde_json::json!([iam_key("42", "enabled")]), 999, 2),
    ] {
        let (result, requests) = save_binding_result(binding_response, keys, id, false).await;
        let error = result.expect_err("binding save must reject the key");
        assert_eq!(error.kind(), ShijuAppErrorKind::InvalidInput);
        assert_eq!(requests.len(), expected_requests);
        assert!(!format!("{error:?} {error}").contains(CREDENTIAL));
    }

    let sensitive_body = format!(
        r#"{{"credential":"{CREDENTIAL}","rawApiKey":"upstream-secret","message":"private"}}"#
    );
    let (app_base, iam_base, requests) = fixture(vec![
        FixtureResponse {
            status: 200,
            headers: Vec::new(),
            body: binding(false, true).to_string(),
        },
        iam_keys(serde_json::json!([iam_key("42", "enabled")])),
        FixtureResponse { status: 500, headers: Vec::new(), body: sensitive_body },
    ]);
    let client = ShijuAppClient::with_base_urls(&app_base, &iam_base).expect("client");
    let error = client
        .save_api_key_binding(CREDENTIAL, INSTALLATION_ID, 42)
        .await
        .expect_err("upstream PUT failure");
    let rendered = format!("{error:?} {error}");
    for secret in [CREDENTIAL, "upstream-secret", "private"] {
        assert!(!rendered.contains(secret));
    }
    assert_eq!(requests.join().expect("join fixture").len(), 3);
}

#[tokio::test(flavor = "multi_thread")]
async fn shiju_app_prepare_launch_rejects_unconfigured_binding() {
    let (app_base, iam_base, requests) = fixture(vec![
        ok(serde_json::json!([installed_app(
            INSTALLATION_ID,
            "ready",
            "ready",
            &format!("https://app.shiju.cloud/app/{INSTALLATION_ID}/")
        )])),
        FixtureResponse {
            status: 200,
            headers: Vec::new(),
            body: binding(false, true).to_string(),
        },
    ]);
    let client = ShijuAppClient::with_base_urls(&app_base, &iam_base).expect("client");

    let error = client
        .prepare_launch(CREDENTIAL, INSTALLATION_ID)
        .await
        .expect_err("unconfigured launch must be rejected");

    assert_eq!(error.kind(), ShijuAppErrorKind::InvalidInput);
    assert!(error.to_string().contains("API key binding is not configured"));
    let requests = requests.join().expect("join fixture");
    assert_eq!(requests.len(), 2);
    assert!(!requests.iter().any(|request| request.starts_with("POST ")));
}
