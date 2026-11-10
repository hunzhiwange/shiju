use super::{
    RoutingListener, SharedTransport, dispatch::JsonTarget, route_auth::VerifiedRouteBinding,
};
use flate2::{Compression, write::GzEncoder};
use serde_json::json;
use std::{io::Write, net::SocketAddr, time::Duration};
use tokio::{
    io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader},
    net::{TcpListener, TcpStream},
    time::timeout,
};
use vw_api_types::{
    agent_apps::AppId,
    app_providers::validation::{ProviderRequestOptions, ProviderRequestOverrides},
    id::{ProjectScope, TenantScope},
};

const LIMIT: usize = 4 * 1024 * 1024;
const PROMPT: &[u8] = br#"{"model":"fixture"}"#;

struct Fixture {
    listener: RoutingListener,
    address: SocketAddr,
    token: String,
    upstream: TcpListener,
}

impl Fixture {
    async fn start() -> Self {
        Self::start_with_options(ProviderRequestOptions::default()).await
    }

    async fn start_with_options(request_options: ProviderRequestOptions) -> Self {
        let upstream = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let listener = RoutingListener::default();
        let address = listener.start(0).unwrap();
        let token = listener
            .auth
            .issue_test_token(
                VerifiedRouteBinding::new(
                    ProjectScope::new(
                        TenantScope::new("platform", "company").unwrap(),
                        "project".into(),
                    )
                    .unwrap(),
                    "host".into(),
                    AppId::Codex,
                    "account".into(),
                    1,
                ),
                JsonTarget {
                    url: format!("http://{}/fixture", upstream.local_addr().unwrap())
                        .parse()
                        .unwrap(),
                    authorization: "Bearer provider-fixture".parse().unwrap(),
                    transport: SharedTransport::new().unwrap(),
                    request_options,
                },
            )
            .unwrap();
        Self { listener, address, token, upstream }
    }

    async fn request(&self, fields: &str, body: &[u8]) -> BufReader<TcpStream> {
        let mut socket = TcpStream::connect(self.address).await.unwrap();
        socket
            .write_all(
                format!(
                    "POST /v1/chat/completions HTTP/1.1\r\nHost: {}\r\nAuthorization: Bearer {}\r\nContent-Type: application/json\r\n{fields}\r\n",
                    self.address, self.token
                )
                .as_bytes(),
            )
            .await
            .unwrap();
        socket.write_all(body).await.unwrap();
        BufReader::new(socket)
    }

    async fn upstream_request(&self) -> BufReader<TcpStream> {
        let (socket, _) = timeout(Duration::from_secs(3), self.upstream.accept())
            .await
            .expect("upstream did not receive the request")
            .unwrap();
        let mut socket = BufReader::new(socket);
        let mut head = String::new();
        timeout(Duration::from_secs(3), async {
            while !head.ends_with("\r\n\r\n") {
                assert!(head.len() < 4096);
                assert_ne!(socket.read_line(&mut head).await.unwrap(), 0);
            }
            let mut body = [0; PROMPT.len()];
            socket.read_exact(&mut body).await.unwrap();
            assert_eq!(body, PROMPT);
        })
        .await
        .unwrap();
        let head = head.to_ascii_lowercase();
        assert!(head.starts_with("post /fixture http/1.1\r\n"));
        assert!(head.contains("\r\nauthorization: bearer provider-fixture\r\n"));
        socket
    }
}

async fn assert_status(socket: &mut BufReader<TcpStream>, code: u16, reason: &str) {
    let mut line = String::new();
    timeout(Duration::from_secs(3), socket.read_line(&mut line))
        .await
        .unwrap_or_else(|_| panic!("no response for {reason}"))
        .unwrap();
    assert!(line.starts_with(&format!("HTTP/1.1 {code} ")), "{reason}: {line:?}");
}

fn gzip(body: &[u8]) -> Vec<u8> {
    let mut encoder = GzEncoder::new(Vec::new(), Compression::fast());
    encoder.write_all(body).unwrap();
    encoder.finish().unwrap()
}

#[tokio::test]
async fn cc_route_020_body_limits_expected_behavior() {
    let fixture = Fixture::start().await;
    for request_headers in ["", "User-Agent: wire-order-fixture\r\n"] {
        let mut client = fixture
            .request(&format!("{request_headers}Content-Length: {}\r\n", PROMPT.len()), PROMPT)
            .await;
        let mut upstream = fixture.upstream_request().await;
        upstream
            .write_all(
                format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close, Content-Length\r\n\r\n",
                    LIMIT + 1
                )
                .as_bytes(),
            )
            .await
            .unwrap();
        assert_status(&mut client, 502, "over-limit response declaration with stalled body").await;
        let mut byte = [0];
        assert_eq!(
            timeout(Duration::from_secs(3), upstream.read(&mut byte)).await.unwrap().unwrap(),
            0
        );
    }
    fixture.listener.stop().await.unwrap();
}

#[tokio::test]
async fn cc_route_020_request_bounds_reject_before_forwarding() {
    let fixture = Fixture::start().await;
    for (fields, body, code) in [
        (format!("Content-Length: {}\r\n", LIMIT + 1), vec![], 413),
        ("Content-Length: +2\r\n".into(), vec![], 400),
        ("Content-Length: 2\r\nContent-Length: 3\r\n".into(), vec![], 400),
        (
            "Transfer-Encoding: chunked\r\nContent-Length: 2\r\n".into(),
            b"2\r\n{}\r\n0\r\n\r\n".to_vec(),
            400,
        ),
        ("Content-Length: 6\r\n".into(), b"{}".to_vec(), 400),
    ] {
        let mut client = fixture.request(&fields, &body).await;
        if fields == "Content-Length: 6\r\n" {
            client.shutdown().await.unwrap();
        }
        assert_status(&mut client, code, &fields).await;
    }

    let mut oversized = vec![b' '; LIMIT + 1];
    oversized[..PROMPT.len()].copy_from_slice(PROMPT);
    let encoded = gzip(&oversized);
    assert!(encoded.len() < LIMIT);
    let mut client = fixture
        .request(
            &format!("Content-Length: {}\r\nContent-Encoding: gzip\r\n", encoded.len()),
            &encoded,
        )
        .await;
    assert_status(&mut client, 413, "expanded request").await;

    let mut client = fixture.request("Transfer-Encoding: chunked\r\n", b"").await;
    client.write_all(format!("{:x}\r\n", oversized.len()).as_bytes()).await.unwrap();
    client.write_all(&oversized).await.unwrap();
    assert_status(&mut client, 413, "chunked request without EOF").await;
    assert!(timeout(Duration::from_millis(50), fixture.upstream.accept()).await.is_err());
    oversized.pop();
    let mut client = fixture.request(&format!("Content-Length: {LIMIT}\r\n"), &oversized).await;
    let mut upstream = fixture.upstream_request().await;
    upstream
        .write_all(
            b"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: 2\r\nConnection: close\r\n\r\n{}",
        )
        .await
        .unwrap();
    assert_status(&mut client, 200, "request at the exact wire limit").await;
    fixture.listener.stop().await.unwrap();

    let fixture = Fixture::start_with_options(ProviderRequestOptions {
        local_proxy_request_overrides: Some(ProviderRequestOverrides {
            body: json!({"added": true}).as_object().cloned(),
            headers: None,
        }),
        ..ProviderRequestOptions::default()
    })
    .await;
    let prefix = br#"{"model":""#;
    let mut exact = vec![b'x'; LIMIT];
    exact[..prefix.len()].copy_from_slice(prefix);
    exact[LIMIT - 2..].copy_from_slice(br#""}"#);
    let mut client = fixture.request(&format!("Content-Length: {LIMIT}\r\n"), &exact).await;
    assert_status(&mut client, 413, "rebuilt request exceeded the wire limit").await;
    assert!(timeout(Duration::from_millis(50), fixture.upstream.accept()).await.is_err());
    fixture.listener.stop().await.unwrap();
}

#[tokio::test]
async fn cc_route_020_response_bounds_wait_for_complete_body() {
    let fixture = Fixture::start().await;
    let fields = format!("Content-Length: {}\r\n", PROMPT.len());

    for (head, body, code) in [
        ("Content-Length: 2\r\n".to_owned(), b"{}".to_vec(), 200),
        ("Content-Length: 3\r\n".into(), b"{}".to_vec(), 502),
        ("Content-Length: x\r\n".into(), vec![], 502),
        (
            "Content-Length: 2\r\nTransfer-Encoding: chunked\r\n".into(),
            b"2\r\n{}\r\n0\r\n\r\n".to_vec(),
            502,
        ),
        ("Transfer-Encoding: chunked\r\n".into(), b"2\r\n{}\r\n".to_vec(), 502),
        ("Transfer-Encoding: chunked\r\n".into(), b"2\r\n{}\r\n0\r\n\r\n".to_vec(), 200),
    ] {
        let mut client = fixture.request(&fields, PROMPT).await;
        let mut upstream = fixture.upstream_request().await;
        upstream
            .write_all(format!("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nConnection: close\r\n{head}\r\n").as_bytes())
            .await
            .unwrap();
        upstream.write_all(&body).await.unwrap();
        upstream.shutdown().await.unwrap();
        assert_status(&mut client, code, &head).await;
    }

    let mut exact = vec![b' '; LIMIT];
    exact[..2].copy_from_slice(b"{}");
    let mut client = fixture.request(&fields, PROMPT).await;
    let mut upstream = fixture.upstream_request().await;
    upstream
        .write_all(
            format!("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {LIMIT}\r\nConnection: close\r\n\r\n").as_bytes(),
        )
        .await
        .unwrap();
    upstream.write_all(&exact).await.unwrap();
    assert_status(&mut client, 200, "response at the exact wire limit").await;
    let mut response = Vec::new();
    timeout(Duration::from_secs(3), client.read_to_end(&mut response)).await.unwrap().unwrap();
    assert!(response.ends_with(&exact));

    exact.push(b' ');
    let encoded = gzip(&exact);
    let mut client = fixture.request(&fields, PROMPT).await;
    let mut upstream = fixture.upstream_request().await;
    upstream
        .write_all(
            format!("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Encoding: gzip\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", encoded.len()).as_bytes(),
        )
        .await
        .unwrap();
    upstream.write_all(&encoded).await.unwrap();
    assert_status(&mut client, 502, "expanded response").await;

    for coding in ["", "Content-Encoding: gzip\r\n"] {
        let mut client = fixture.request(&fields, PROMPT).await;
        let mut upstream = fixture.upstream_request().await;
        upstream
            .write_all(
                format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nTransfer-Encoding: chunked\r\nConnection: close\r\n{coding}\r\n{:x}\r\n",
                    exact.len()
                ).as_bytes(),
            )
            .await
            .unwrap();
        upstream.write_all(&exact).await.unwrap();
        assert_status(&mut client, 502, "chunked upstream over limit without EOF").await;
    }

    let mut client = fixture.request(&fields, PROMPT).await;
    let mut upstream = fixture.upstream_request().await;
    upstream
        .write_all(b"HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nTransfer-Encoding: chunked\r\nConnection: close\r\n\r\n20\r\ndata: {")
        .await
        .unwrap();
    upstream.shutdown().await.unwrap();
    assert_status(&mut client, 502, "SSE is not aggregated on the JSON route").await;
    fixture.listener.stop().await.unwrap();
}

#[tokio::test]
async fn cc_route_020_slow_requests_release_buffer_slots() {
    let fixture = Fixture::start().await;
    let mut stalled = Vec::new();
    for _ in 0..8 {
        let mut client =
            fixture.request("Content-Length: 100\r\nExpect: 100-continue\r\n", b"").await;
        assert_status(&mut client, 100, "admitted slow upload").await;
        stalled.push(client);
    }
    let mut client = fixture.request("Content-Length: 2\r\n", b"{}").await;
    assert_status(&mut client, 503, "all body buffers occupied").await;

    let mut response = String::new();
    timeout(Duration::from_secs(12), stalled[0].read_to_string(&mut response))
        .await
        .unwrap()
        .unwrap();
    assert!(response.starts_with("\r\nHTTP/1.1 408 "), "{response:?}");
    assert!(timeout(Duration::from_millis(50), fixture.upstream.accept()).await.is_err());
    let mut client =
        fixture.request(&format!("Content-Length: {}\r\n", PROMPT.len()), PROMPT).await;
    let mut upstream = fixture.upstream_request().await;
    upstream
        .write_all(
            b"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: 2\r\nConnection: close\r\n\r\n{}",
        )
        .await
        .unwrap();
    assert_status(&mut client, 200, "buffer permit reused after timeout").await;
    fixture.listener.stop().await.unwrap();
}
