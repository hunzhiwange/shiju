// 覆盖识炬应用目录、启动和绑定接口的转发与脱敏。

use std::io::{Read, Write};
use std::net::TcpListener;
use std::thread;

use axum::Router;
use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode};
use serde_json::Value;
use tower::ServiceExt;
use vw_gateway_client::SHIJU_CREDENTIAL_HEADER;

const CREDENTIAL: &str = "fixture-handler-credential-secret";
const INSTALLATION_ID: &str = "01K1APP0000000000000000001";

struct FixtureResponse {
    status: u16,
    headers: Vec<(&'static str, String)>,
    body: String,
}

fn fixture(responses: Vec<FixtureResponse>) -> (String, String, thread::JoinHandle<Vec<String>>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind SHIJU handler fixture");
    let address = listener.local_addr().expect("fixture address");
    let handle = thread::spawn(move || {
        responses
            .into_iter()
            .map(|response| {
                let (mut stream, _) = listener.accept().expect("accept SHIJU handler request");
                let mut request = Vec::new();
                let mut buffer = [0_u8; 4096];
                let mut expected_len = None;
                loop {
                    let read = stream.read(&mut buffer).expect("read SHIJU handler request");
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
                stream.write_all(wire.as_bytes()).expect("write SHIJU handler response");
                String::from_utf8(request).expect("request UTF-8")
            })
            .collect()
    });
    (format!("http://{address}/app-platform/api/"), format!("http://{address}/iam/api/v1/"), handle)
}

fn envelope(items: Value) -> String {
    serde_json::json!({"$schema": "fixture", "items": items}).to_string()
}

fn installed() -> Value {
    serde_json::json!([{
        "installationId": INSTALLATION_ID,
        "appId": "knowledge-assistant",
        "title": "知识助手",
        "version": "1.2.3",
        "origin": "catalog",
        "reconcileStatus": "ready",
        "runtimeStatus": "ready",
        "runtimeUrl": format!("https://app.shiju.cloud/app/{INSTALLATION_ID}/"),
        "installedAt": "2026-07-30T09:30:00+08:00",
        "updatedAt": "2026-07-30T09:31:00+08:00"
    }])
}

async fn response(app: Router, request: Request<Body>) -> (StatusCode, Value) {
    let response = app.oneshot(request).await.expect("response");
    let status = response.status();
    let bytes = to_bytes(response.into_body(), usize::MAX).await.expect("body");
    let body = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
    (status, body)
}

fn request(method: &str, path: &str, credential: Option<&str>) -> Request<Body> {
    request_with_body(method, path, credential, Body::empty())
}

fn request_with_body(
    method: &str,
    path: &str,
    credential: Option<&str>,
    body: Body,
) -> Request<Body> {
    let mut builder = Request::builder().method(method).uri(path);
    if let Some(credential) = credential {
        builder = builder.header(SHIJU_CREDENTIAL_HEADER, credential);
    }
    builder.header("content-type", "application/json").body(body).expect("request")
}

// 本地上游夹具记录实际请求并返回预设响应，既验证转发参数也避免依赖外部服务。
#[tokio::test]
async fn shiju_app_routes_require_credentials_and_valid_installation_ids() {
    let client = crate::shiju::apps::ShijuAppClient::with_base_urls(
        "http://127.0.0.1:9/app-platform/api/",
        "http://127.0.0.1:9/iam/api/v1/",
    )
    .expect("client");
    let app = super::router_with_client::<()>(client);

    let (status, body) = response(app.clone(), request("GET", "/shiju/apps/catalog", None)).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["error"], "SHIJU credential header is required");

    let (status, body) = response(app.clone(), request("GET", "/shiju/apps", Some("   "))).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(!body.to_string().contains(CREDENTIAL));

    let (status, body) =
        response(app.clone(), request("POST", "/shiju/apps/lowercase/launch", Some(CREDENTIAL)))
            .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(body["error"].as_str().unwrap_or_default().contains("installation ID"));
    assert!(!body.to_string().contains(CREDENTIAL));

    let (status, body) = response(
        app.clone(),
        request("GET", "/shiju/apps/lowercase/api-key-binding", Some(CREDENTIAL)),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(body["error"].as_str().unwrap_or_default().contains("installation ID"));

    let (status, body) = response(
        app.clone(),
        request_with_body(
            "PUT",
            "/shiju/apps/lowercase/api-key-binding",
            Some(CREDENTIAL),
            Body::from(r#"{"apiKeyId":42}"#),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(body["error"].as_str().unwrap_or_default().contains("installation ID"));

    for (method, path) in [
        ("POST", "/shiju/apps/catalog"),
        ("POST", "/shiju/apps"),
        ("POST", "/shiju/apps/api-keys"),
        ("POST", &format!("/shiju/apps/{INSTALLATION_ID}/api-key-binding")),
        ("GET", &format!("/shiju/apps/{INSTALLATION_ID}/launch")),
    ] {
        let (status, _) = response(app.clone(), request(method, path, Some(CREDENTIAL))).await;
        assert_eq!(status, StatusCode::METHOD_NOT_ALLOWED, "{method} {path}");
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn shiju_app_routes_map_catalog_apps_and_launch() {
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
    let installed = installed();
    let (app_base, iam_base, requests) = fixture(vec![
        FixtureResponse {
            status: 200,
            headers: Vec::new(),
            body: envelope(catalog),
        },
        FixtureResponse {
            status: 200,
            headers: Vec::new(),
            body: envelope(installed.clone()),
        },
        FixtureResponse {
            status: 200,
            headers: Vec::new(),
            body: envelope(installed),
        },
        FixtureResponse {
            status: 200,
            headers: Vec::new(),
            body: serde_json::json!({
                "personal": {
                    "configured": true,
                    "canEdit": true,
                    "apiKeyId": 42,
                    "keyName": "Desktop automation",
                    "apiKeyMask": "sk-****cafe",
                    "status": "enabled",
                    "expiresAt": null,
                    "updatedAt": null
                }
            })
            .to_string(),
        },
        FixtureResponse {
            status: 204,
            headers: vec![(
                "Set-Cookie",
                "app_runtime_session=handler-runtime-session-secret; Path=/; Max-Age=3600; HttpOnly; Secure; SameSite=None"
                    .to_string(),
            )],
            body: String::new(),
        },
    ]);
    let client =
        crate::shiju::apps::ShijuAppClient::with_base_urls(&app_base, &iam_base).expect("client");
    let app = super::router_with_client::<()>(client);

    let (status, catalog) =
        response(app.clone(), request("GET", "/shiju/apps/catalog", Some(CREDENTIAL))).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(catalog["apps"][0]["userId"], "101027");
    assert_eq!(catalog["apps"][0]["permissions"], serde_json::json!([]));

    let (status, apps) =
        response(app.clone(), request("GET", "/shiju/apps", Some(CREDENTIAL))).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(apps["apps"][0]["runtimeStatus"], "ready");

    let (status, launch) = response(
        app,
        request("POST", &format!("/shiju/apps/{INSTALLATION_ID}/launch"), Some(CREDENTIAL)),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(launch["installationId"], INSTALLATION_ID);
    assert_eq!(launch["name"], "知识助手");
    assert!(
        launch["runtimeSetCookie"]
            .as_str()
            .is_some_and(|value| value.starts_with("app_runtime_session="))
    );
    assert_eq!(launch["runtimeUrl"], format!("https://app.shiju.cloud/app/{INSTALLATION_ID}/"));

    let requests = requests.join().expect("join fixture");
    assert_eq!(requests.len(), 5);
    assert!(requests[0].starts_with("GET /app-platform/api/catalog "));
    assert!(requests[1].starts_with("GET /app-platform/api/apps "));
    assert!(requests[2].starts_with("GET /app-platform/api/apps "));
    assert!(requests[3].starts_with(&format!(
        "GET /app-platform/api/tenant/installations/{INSTALLATION_ID}/api-key-binding "
    )));
    assert!(requests[4].starts_with("POST /app-platform/api/runtime/browser-session "));
    for request in requests {
        assert!(!request.lines().next().unwrap_or_default().contains(CREDENTIAL));
    }
}

#[tokio::test(flavor = "multi_thread")]
// 上游错误需保留可操作状态，同时裁剪凭据、响应正文等敏感细节。
async fn shiju_app_route_redacts_upstream_errors() {
    let sensitive = format!(
        r#"{{"credential":"{CREDENTIAL}","runtimeUrl":"https://app.shiju.cloud/app/{INSTALLATION_ID}/?private=1","message":"upstream-body-secret"}}"#
    );
    let (app_base, iam_base, requests) =
        fixture(vec![FixtureResponse { status: 500, headers: Vec::new(), body: sensitive }]);
    let client =
        crate::shiju::apps::ShijuAppClient::with_base_urls(&app_base, &iam_base).expect("client");

    let (status, body) = response(
        super::router_with_client::<()>(client),
        request("GET", "/shiju/apps/catalog", Some(CREDENTIAL)),
    )
    .await;

    assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
    for secret in [CREDENTIAL, "upstream-body-secret", "private=1"] {
        assert!(!body.to_string().contains(secret));
    }
    requests.join().expect("join fixture");
}

#[tokio::test(flavor = "multi_thread")]
async fn shiju_app_binding_routes_map_get_keys_and_put() {
    let binding = serde_json::json!({
        "personal": {
            "configured": false,
            "canEdit": true,
            "apiKeyId": null,
            "keyName": null,
            "apiKeyMask": null,
            "status": null,
            "expiresAt": null,
            "updatedAt": null
        }
    });
    let key = serde_json::json!({
        "api_key_id": "42",
        "key_name": "Desktop automation",
        "api_key_mask": "sk-****cafe",
        "status": "enabled",
        "expires_at": null,
        "updated_at": null
    });
    let (app_base, iam_base, requests) = fixture(vec![
        FixtureResponse { status: 200, headers: Vec::new(), body: binding.to_string() },
        FixtureResponse {
            status: 200,
            headers: Vec::new(),
            body: serde_json::json!({"code": 200, "data": {"list": [key.clone()]}}).to_string(),
        },
        FixtureResponse { status: 200, headers: Vec::new(), body: binding.to_string() },
        FixtureResponse {
            status: 200,
            headers: Vec::new(),
            body: serde_json::json!({"code": 200, "data": {"list": [key]}}).to_string(),
        },
        FixtureResponse { status: 204, headers: Vec::new(), body: String::new() },
    ]);
    let client =
        crate::shiju::apps::ShijuAppClient::with_base_urls(&app_base, &iam_base).expect("client");
    let app = super::router_with_client::<()>(client);

    let (status, binding) = response(
        app.clone(),
        request("GET", &format!("/shiju/apps/{INSTALLATION_ID}/api-key-binding"), Some(CREDENTIAL)),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(binding["personal"]["configured"], false);

    let (status, keys) =
        response(app.clone(), request("GET", "/shiju/apps/api-keys", Some(CREDENTIAL))).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(keys["apiKeys"][0]["apiKeyId"], 42);

    let (status, body) = response(
        app,
        request_with_body(
            "PUT",
            &format!("/shiju/apps/{INSTALLATION_ID}/api-key-binding"),
            Some(CREDENTIAL),
            Body::from(r#"{"apiKeyId":42}"#),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    assert_eq!(body, Value::Null);

    let requests = requests.join().expect("join fixture");
    assert_eq!(requests.len(), 5);
    assert!(requests[0].starts_with(&format!(
        "GET /app-platform/api/tenant/installations/{INSTALLATION_ID}/api-key-binding "
    )));
    assert!(requests[1].starts_with("GET /iam/api/v1/api-keys "));
    assert!(requests[2].starts_with(&format!(
        "GET /app-platform/api/tenant/installations/{INSTALLATION_ID}/api-key-binding "
    )));
    assert!(requests[3].starts_with("GET /iam/api/v1/api-keys "));
    assert!(requests[4].starts_with(&format!(
        "PUT /app-platform/api/tenant/installations/{INSTALLATION_ID}/api-key-binding "
    )));
}
