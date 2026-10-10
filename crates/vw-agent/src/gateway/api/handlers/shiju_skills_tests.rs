// 覆盖识炬技能查询、详情与安装请求的参数转发。

use std::io::{Read, Write};
use std::net::TcpListener;
use std::thread;

use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode};
use serde_json::Value;
use tower::ServiceExt;
use vw_gateway_client::SHIJU_CREDENTIAL_HEADER;

async fn response(request: Request<Body>) -> (StatusCode, Value) {
    let response = super::router_with_client::<()>(
        crate::skills::shiju::ShijuSkillClient::with_base_url("http://127.0.0.1:9")
            .expect("client"),
    )
    .oneshot(request)
    .await
    .expect("response");
    let status = response.status();
    let body = to_bytes(response.into_body(), usize::MAX).await.expect("body");
    let value = serde_json::from_slice(&body).unwrap_or(Value::Null);
    (status, value)
}

fn upstream_fixture_with_response(
    request_count: usize,
    response: Vec<u8>,
) -> (String, thread::JoinHandle<Vec<String>>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind SHIJU route fixture");
    let address = listener.local_addr().expect("fixture address");
    let handle = thread::spawn(move || {
        (0..request_count)
            .map(|_| {
                let (mut stream, _) = listener.accept().expect("accept SHIJU route request");
                let mut request = Vec::new();
                let mut buffer = [0_u8; 2048];
                loop {
                    let read = stream.read(&mut buffer).expect("read SHIJU route request");
                    request.extend_from_slice(&buffer[..read]);
                    if read == 0 {
                        break;
                    }
                    if let Some(header_end) =
                        request.windows(4).position(|window| window == b"\r\n\r\n")
                    {
                        let headers = String::from_utf8_lossy(&request[..header_end]);
                        let content_length = headers
                            .lines()
                            .find_map(|line| {
                                line.to_ascii_lowercase()
                                    .strip_prefix("content-length:")
                                    .and_then(|value| value.trim().parse::<usize>().ok())
                            })
                            .unwrap_or(0);
                        if request.len() >= header_end + 4 + content_length {
                            break;
                        }
                    }
                }
                stream.write_all(&response).expect("write SHIJU route response");
                String::from_utf8(request).expect("request UTF-8")
            })
            .collect()
    });
    (format!("http://{address}"), handle)
}

fn upstream_fixture(request_count: usize) -> (String, thread::JoinHandle<Vec<String>>) {
    upstream_fixture_with_response(
        request_count,
        b"HTTP/1.1 204 No Content\r\nContent-Length: 0\r\nConnection: close\r\n\r\n".to_vec(),
    )
}

#[tokio::test(flavor = "multi_thread")]
async fn shiju_skill_readme_route_forwards_markdown_content() {
    let markdown = "# 使用说明\n支持 UTF-8 正文。";
    let body = serde_json::json!({
        "code": 200,
        "message": "",
        "data": {"skill_id": 31, "version": "0.1.3", "markdown": markdown}
    })
    .to_string();
    let response = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: text/plain; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    let (base, requests) = upstream_fixture_with_response(1, response.into_bytes());
    let router = super::router_with_client::<()>(
        crate::skills::shiju::ShijuSkillClient::with_base_url(&base).expect("client"),
    );

    let response = router
        .oneshot(
            Request::get("/shiju/skills/31/readme")
                .header(SHIJU_CREDENTIAL_HEADER, "fixture-handler-secret")
                .body(Body::empty())
                .expect("README request"),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::OK);
    let response_body = to_bytes(response.into_body(), usize::MAX).await.expect("body");
    assert_eq!(
        serde_json::from_slice::<Value>(&response_body).expect("README JSON"),
        serde_json::json!({"markdown": markdown})
    );

    let requests = requests.join().expect("join fixture");
    assert!(requests[0].starts_with("GET /skills/31/readme "));
    assert!(!requests[0].contains(markdown));
}

#[tokio::test(flavor = "multi_thread")]
async fn shiju_skill_routes_forward_maintained_multi_value_filters() {
    let body = r#"{"code":200,"message":"","data":{"items":[],"page":1,"page_size":20,"total":0}}"#;
    let response = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    let (base, requests) = upstream_fixture_with_response(1, response.into_bytes());
    let router = super::router_with_client::<()>(
        crate::skills::shiju::ShijuSkillClient::with_base_url(&base).expect("client"),
    );

    let response = router
        .oneshot(
            Request::get(
                "/shiju/skills/maintained?page=1&page_size=20&tag_ids=2%2C1%2C3%2C4&statuses=draft%2Cpending_review%2Cpublished%2Cunpublished",
            )
            .header(SHIJU_CREDENTIAL_HEADER, "fixture-handler-secret")
            .body(Body::empty())
            .expect("maintained request"),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::OK);

    let requests = requests.join().expect("join fixture");
    assert!(requests[0].starts_with(
        "GET /me/maintained-skills?page=1&page_size=20&tag_ids=2%2C1%2C3%2C4&statuses=draft%2Cpending_review%2Cpublished%2Cunpublished "
    ));
}

#[tokio::test(flavor = "multi_thread")]
async fn shiju_skill_routes_forward_empty_and_version_install_bodies() {
    let (base, requests) = upstream_fixture(2);
    let router = super::router_with_client::<()>(
        crate::skills::shiju::ShijuSkillClient::with_base_url(&base).expect("client"),
    );

    let response = router
        .clone()
        .oneshot(
            Request::post("/shiju/skills/31/install")
                .header(SHIJU_CREDENTIAL_HEADER, "fixture-handler-secret")
                .body(Body::empty())
                .expect("install request"),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::OK);

    let response = router
        .oneshot(
            Request::post("/shiju/skills/31/install")
                .header(SHIJU_CREDENTIAL_HEADER, "fixture-handler-secret")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"version_id":88}"#))
                .expect("version install request"),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::OK);

    let requests = requests.join().expect("join fixture");
    assert!(requests[0].starts_with("POST /skills/31/install "));
    assert!(!requests[0].contains("version_id"));
    assert!(requests[1].starts_with("POST /skills/31/install "));
    assert!(requests[1].ends_with(r#"{"version_id":88}"#));
}

// 本地上游夹具记录实际请求并返回预设响应，既验证转发参数也避免依赖外部服务。
// 上游错误需保留可操作状态，同时裁剪凭据、响应正文等敏感细节。
#[tokio::test]
async fn shiju_skill_routes_reject_missing_credential_and_invalid_filters() {
    let (status, body) = response(
        Request::get("/shiju/skills/installed").body(Body::empty()).expect("installed request"),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["error"], "SHIJU credential header is required");

    let (status, body) = response(
        Request::get("/shiju/skills/system?page=0&page_size=20")
            .header(SHIJU_CREDENTIAL_HEADER, "fixture-handler-secret")
            .body(Body::empty())
            .expect("system request"),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(body["error"].as_str().unwrap_or_default().contains("page"));

    let (status, body) = response(
        Request::get("/shiju/skills/company?page=1&page_size=20&status=%20%20")
            .header(SHIJU_CREDENTIAL_HEADER, "fixture-handler-secret")
            .body(Body::empty())
            .expect("company request"),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(body["error"].as_str().unwrap_or_default().contains("status"));
    assert!(!body.to_string().contains("fixture-handler-secret"));

    for (query, expected) in [
        ("tag_ids=2,,1", "tag_ids"),
        ("tag_ids=2,0,1", "tag_ids"),
        ("statuses=draft,archived", "statuses"),
    ] {
        let (status, body) = response(
            Request::get(format!("/shiju/skills/maintained?page=1&page_size=20&{query}"))
                .header(SHIJU_CREDENTIAL_HEADER, "fixture-handler-secret")
                .body(Body::empty())
                .expect("maintained request"),
        )
        .await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert!(body["error"].as_str().unwrap_or_default().contains(expected));
    }

    let (status, body) = response(
        Request::post("/shiju/skills/31/install")
            .header(SHIJU_CREDENTIAL_HEADER, "fixture-handler-secret")
            .header("content-type", "application/json")
            .body(Body::from(r#"{"version_id":0}"#))
            .expect("install request"),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(body["error"].as_str().unwrap_or_default().contains("version"));
    assert!(!body.to_string().contains("fixture-handler-secret"));
}
