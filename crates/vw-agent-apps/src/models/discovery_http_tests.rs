use super::discovery_http::{DiscoveredModel, HttpDiscoveryRequest, discover};
use reqwest::Url;
use std::{collections::BTreeMap, future::pending, sync::Mutex, time::Duration};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
    sync::Notify,
    task::JoinHandle,
    time::timeout,
};
use vw_api_types::app_providers::{ProviderProtocol, endpoints::AppProviderEndpointKind};

const KEY: &str = "fixture-secret";
const DATA: &str = r#"{"data":[{"id":"model-a"}]}"#;

struct Server {
    origin: String,
    requests: std::sync::Arc<Mutex<Vec<String>>>,
    connected: std::sync::Arc<Notify>,
    closed: std::sync::Arc<Notify>,
    worker: JoinHandle<()>,
}

impl Server {
    async fn serve(responses: Vec<String>) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let origin = format!("http://{}", listener.local_addr().unwrap());
        let requests = std::sync::Arc::new(Mutex::new(Vec::new()));
        let connected = std::sync::Arc::new(Notify::new());
        let closed = std::sync::Arc::new(Notify::new());
        let worker = tokio::spawn({
            let (requests, connected, closed) =
                (requests.clone(), connected.clone(), closed.clone());
            async move {
                let mut responses = responses.into_iter();
                while let Ok((mut stream, _)) = listener.accept().await {
                    let mut buffer = Vec::new();
                    let mut chunk = [0; 1024];
                    loop {
                        let len = stream.read(&mut chunk).await.unwrap();
                        if len == 0 {
                            break;
                        }
                        buffer.extend_from_slice(&chunk[..len]);
                        if buffer.windows(4).any(|window| window == b"\r\n\r\n") {
                            break;
                        }
                    }
                    requests.lock().unwrap().push(String::from_utf8(buffer).unwrap());
                    if let Some(response) = responses.next() {
                        let _ = stream.write_all(response.as_bytes()).await;
                    } else {
                        let _ = stream
                            .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 100\r\n\r\n{")
                            .await;
                        connected.notify_one();
                        if matches!(stream.read(&mut chunk).await, Ok(0) | Err(_)) {
                            closed.notify_one();
                        }
                    }
                }
            }
        });
        Self { origin, requests, connected, closed, worker }
    }

    fn requests(&self) -> Vec<String> {
        self.requests.lock().unwrap().clone()
    }

    fn paths(&self) -> Vec<String> {
        self.requests()
            .iter()
            .map(|request| request.split_ascii_whitespace().nth(1).unwrap().into())
            .collect()
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        self.worker.abort();
    }
}

fn response(status: u16, body: &str) -> String {
    format!(
        "HTTP/1.1 {status} Fixture\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    )
}

fn request<'a>(
    base_url: &'a str,
    headers: &'a BTreeMap<String, String>,
) -> HttpDiscoveryRequest<'a> {
    HttpDiscoveryRequest {
        base_url,
        endpoint_kind: AppProviderEndpointKind::BaseUrl,
        protocol: ProviderProtocol::OpenaiResponses,
        models_url: None,
        authorized_overrides: &[],
        api_key: KEY,
        user_agent: None,
        request_headers: headers,
    }
}

fn header<'a>(request: &'a str, name: &str) -> Option<&'a str> {
    request
        .lines()
        .filter_map(|line| line.split_once(':'))
        .find_map(|(key, value)| key.eq_ignore_ascii_case(name).then_some(value.trim()))
}

#[tokio::test]
async fn catalog_discovery_http_stops_after_auth_denial() {
    for status in [401, 403] {
        let body = "upstream said Authorization: Bearer fixture-secret";
        let server = Server::serve(vec![response(status, body), response(200, DATA)]).await;
        let base = format!("{}/v4", server.origin);
        let error = discover(request(&base, &BTreeMap::new()), pending()).await.unwrap_err();
        assert_eq!(server.paths(), ["/v4/models"]);
        assert!(!error.contains(KEY) && !error.contains("Bearer") && !error.contains(body));
        assert_eq!(error, "unauthorized");
    }
}

#[tokio::test]
async fn catalog_discovery_http_falls_back_only_for_missing_or_unsupported_endpoints() {
    let server =
        Server::serve(vec![response(404, KEY), response(405, KEY), response(200, DATA)]).await;
    let base = format!("{}/anthropic", server.origin);
    let models = discover(request(&base, &BTreeMap::new()), pending()).await.unwrap();
    assert_eq!(models[0].id, "model-a");
    assert_eq!(server.paths(), ["/anthropic/v1/models", "/v1/models", "/models"]);
}

#[tokio::test]
async fn catalog_discovery_http_does_not_fall_back_for_other_statuses() {
    for status in [400, 429, 500] {
        let server = Server::serve(vec![response(status, KEY), response(200, DATA)]).await;
        let base = format!("{}/v4", server.origin);
        let result = discover(request(&base, &BTreeMap::new()), pending()).await;
        assert_eq!(result.unwrap_err(), "http_status");
        assert_eq!(server.paths(), ["/v4/models"]);
    }
}

#[tokio::test]
async fn catalog_discovery_http_prefers_data_preserves_owner_and_filters_blank_ids() {
    let body = r#"{"data":[{"id":" zeta ","owned_by":"fixture"},{"id":" "},{"id":"alpha"},{"id":"alpha"},{}],"models":{"malformed":true}}"#;
    let server = Server::serve(vec![response(200, body)]).await;
    let result = discover(request(&server.origin, &BTreeMap::new()), pending()).await.unwrap();
    assert_eq!(
        result,
        [
            DiscoveredModel { id: "alpha".into(), owned_by: None },
            DiscoveredModel { id: "zeta".into(), owned_by: Some("fixture".into()) },
        ]
    );
}

#[tokio::test]
async fn catalog_discovery_http_accepts_models_slug_and_id() {
    let body = r#"{"models":[{"slug":"catalog-slug","id":"ignore"},{"slug":" ","id":"catalog-id"},{"id":"models/keep"},{"slug":1,"id":""},null,{}]}"#;
    let server = Server::serve(vec![response(200, body)]).await;
    let result = discover(request(&server.origin, &BTreeMap::new()), pending()).await.unwrap();
    let ids: Vec<_> = result.iter().map(|model| model.id.as_str()).collect();
    assert_eq!(ids, ["catalog-id", "catalog-slug", "models/keep"]);
}

#[tokio::test]
async fn catalog_discovery_http_uses_gemini_names_and_protocol_auth() {
    let body =
        r#"{"models":[{"name":"models/gemini-test"},{"name":"models/ "},{"name":"Display name"}]}"#;
    let server = Server::serve(vec![response(200, body)]).await;
    let base = format!("{}/v1", server.origin);
    let headers = BTreeMap::new();
    let mut input = request(&base, &headers);
    input.protocol = ProviderProtocol::Gemini;
    let models = discover(input, pending()).await.unwrap();
    assert_eq!(models, [DiscoveredModel { id: "gemini-test".into(), owned_by: None }]);
    assert_eq!(server.paths(), ["/v1beta/models"]);
    let requests = server.requests();
    assert_eq!(header(&requests[0], "x-goog-api-key"), Some(KEY));
    assert_eq!(header(&requests[0], "authorization"), None);
}

#[tokio::test]
async fn catalog_discovery_http_sends_only_configured_user_agent_and_custom_headers() {
    let server = Server::serve(vec![response(200, DATA), response(200, DATA)]).await;
    let headers = BTreeMap::from([("x-tenant".into(), "fixture-tenant".into())]);
    let mut input = request(&server.origin, &headers);
    input.protocol = ProviderProtocol::Anthropic;
    input.user_agent = Some("fixture-cli/1");
    discover(input, pending()).await.unwrap();
    discover(request(&server.origin, &BTreeMap::new()), pending()).await.unwrap();

    let requests = server.requests();
    assert_eq!(header(&requests[0], "x-api-key"), Some(KEY));
    assert_eq!(header(&requests[0], "authorization"), None);
    assert_eq!(header(&requests[0], "user-agent"), Some("fixture-cli/1"));
    assert_eq!(header(&requests[0], "x-tenant"), Some("fixture-tenant"));
    assert_eq!(header(&requests[1], "authorization"), Some("Bearer fixture-secret"));
    assert_eq!(header(&requests[1], "user-agent"), None);
    assert_eq!(header(&requests[1], "x-tenant"), None);
}

#[tokio::test]
async fn catalog_discovery_http_rejects_protected_or_invalid_headers_without_io() {
    let server = Server::serve(vec![response(200, DATA); 12]).await;
    for name in [
        " Authorization ",
        "X-API-Key",
        "x-goog-api-key",
        "Host",
        "Transfer-Encoding",
        "Cookie",
        "User-Agent",
        "X-Forwarded-Host",
        "bad header",
    ] {
        let headers = BTreeMap::from([(name.into(), KEY.into())]);
        let result = discover(request(&server.origin, &headers), pending()).await;
        assert_eq!(result.unwrap_err(), "invalid_headers");
    }
    let headers = BTreeMap::from([("X-Tenant".into(), "a".repeat(8193))]);
    assert_eq!(
        discover(request(&server.origin, &headers), pending()).await.unwrap_err(),
        "invalid_headers"
    );
    assert!(server.requests().is_empty());
}

#[tokio::test]
async fn catalog_discovery_http_rejects_invalid_credentials_without_io() {
    let server = Server::serve(vec![response(200, DATA); 3]).await;
    let headers = BTreeMap::new();
    for key in [" ", "header\r\nfixture-secret", "fixture-secret\r"] {
        let mut input = request(&server.origin, &headers);
        input.api_key = key;
        assert_eq!(discover(input, pending()).await.unwrap_err(), "invalid_credential");
    }
    assert!(server.requests().is_empty());
}

#[tokio::test]
async fn catalog_discovery_http_rejects_unauthorized_hosts_and_exact_path_mismatches() {
    let base = Server::serve(vec![response(200, DATA); 3]).await;
    let foreign = Server::serve(vec![response(200, DATA); 3]).await;
    let exact = format!("{}/exact/models", foreign.origin);
    let near = format!("{}/other/models", foreign.origin);
    let authorized = [Url::parse(&exact).unwrap()];
    let headers = BTreeMap::new();

    let mut input = request(&base.origin, &headers);
    input.models_url = Some(&near);
    input.authorized_overrides = &authorized;
    assert_eq!(discover(input, pending()).await.unwrap_err(), "invalid_endpoint");

    let query = format!("{exact}?key={KEY}");
    let mut input = request(&base.origin, &headers);
    input.models_url = Some(&query);
    input.authorized_overrides = &authorized;
    assert_eq!(discover(input, pending()).await.unwrap_err(), "invalid_endpoint");
    assert!(base.requests().is_empty());
    assert!(foreign.requests().is_empty());

    let mut input = request(&base.origin, &headers);
    input.models_url = Some(&exact);
    input.authorized_overrides = &authorized;
    discover(input, pending()).await.unwrap();
    assert!(base.requests().is_empty());
    assert_eq!(foreign.paths(), ["/exact/models"]);
}

#[tokio::test]
async fn catalog_discovery_http_never_follows_redirects() {
    let foreign = Server::serve(vec![response(200, DATA)]).await;
    for location in ["/other".into(), format!("{}/other", foreign.origin)] {
        let redirect =
            format!("HTTP/1.1 302 Found\r\nLocation: {location}\r\nContent-Length: 0\r\n\r\n");
        let server = Server::serve(vec![redirect, response(200, DATA)]).await;
        let base = format!("{}/v4", server.origin);
        let result = discover(request(&base, &BTreeMap::new()), pending()).await;
        assert_eq!(result.unwrap_err(), "redirect");
        assert_eq!(server.paths(), ["/v4/models"]);
        assert!(foreign.requests().is_empty());
    }
}

#[tokio::test]
async fn catalog_discovery_http_bounds_declared_and_streamed_bodies() {
    let size = 1024 * 1024 + 1;
    let declared = format!("HTTP/1.1 200 OK\r\nContent-Length: {size}\r\n\r\n");
    let streamed = format!(
        "HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n{size:x}\r\n{}\r\n0\r\n\r\n",
        "a".repeat(size)
    );
    for wire in [declared, streamed] {
        let server = Server::serve(vec![wire]).await;
        let result = discover(request(&server.origin, &BTreeMap::new()), pending()).await;
        assert_eq!(result.unwrap_err(), "response_too_large");
    }
}

#[tokio::test]
async fn catalog_discovery_http_malformed_response_does_not_echo_payload_or_retry() {
    let server = Server::serve(vec![response(200, KEY), response(200, DATA)]).await;
    let base = format!("{}/v4", server.origin);
    let result = discover(request(&base, &BTreeMap::new()), pending()).await;
    assert_eq!(result.unwrap_err(), "invalid_response");
    assert_eq!(server.paths(), ["/v4/models"]);
}

#[tokio::test]
async fn catalog_discovery_http_cancellation_skips_io_and_drops_in_flight_body() {
    let server = Server::serve(Vec::new()).await;
    let base = format!("{}/v4", server.origin);
    let headers = BTreeMap::new();
    let result = timeout(Duration::from_secs(1), discover(request(&base, &headers), async {}))
        .await
        .unwrap();
    assert_eq!(result.unwrap_err(), "cancelled");
    assert!(server.requests().is_empty());

    let result = timeout(
        Duration::from_secs(2),
        discover(request(&base, &headers), server.connected.notified()),
    )
    .await
    .unwrap();
    assert_eq!(result.unwrap_err(), "cancelled");
    timeout(Duration::from_secs(2), server.closed.notified()).await.unwrap();
    assert_eq!(server.paths(), ["/v4/models"]);
}
