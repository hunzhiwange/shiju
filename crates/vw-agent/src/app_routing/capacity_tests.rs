use super::{
    RoutingListener, SharedTransport, dispatch::JsonTarget, route_auth::VerifiedRouteBinding,
};
use std::{net::SocketAddr, time::Duration};
use tokio::{
    io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader},
    net::{TcpListener, TcpSocket, TcpStream},
    time::timeout,
};
use vw_agent_apps::routing::lifecycle_state::LifecycleState;
use vw_api_types::{
    agent_apps::AppId,
    id::{ProjectScope, TenantScope},
};

const GUARD: Duration = Duration::from_secs(4);
const QUIET: Duration = Duration::from_millis(100);
const JSON: &str = r#"{"model":"fixture"}"#;
const STREAM: &str = r#"{"model":"fixture","stream":true}"#;
const OUTPUT: &str = "data: {\"choices\":[{\"delta\":{\"content\":\"first\"}}]}\n\n";

struct Fixture {
    listener: RoutingListener,
    address: SocketAddr,
    upstream: TcpListener,
}

impl Fixture {
    async fn start() -> Self {
        let upstream = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let listener = RoutingListener::default();
        let address = listener.start(0).unwrap();
        Self { listener, address, upstream }
    }

    fn issue(&self, project: &str) -> String {
        self.listener
            .auth
            .issue_test_token(
                VerifiedRouteBinding::new(
                    ProjectScope::new(
                        TenantScope::new("platform", "company").unwrap(),
                        project.into(),
                    )
                    .unwrap(),
                    "host".into(),
                    AppId::Codex,
                    "account".into(),
                    1,
                ),
                JsonTarget {
                    #[cfg(target_os = "macos")]
                    managed: None,
                    url: format!("http://{}/fixture", self.upstream.local_addr().unwrap())
                        .parse()
                        .unwrap(),
                    authorization: "Bearer capacity-fixture".parse().unwrap(),
                    transport: SharedTransport::new().unwrap(),
                    request_options: Default::default(),
                },
            )
            .unwrap()
    }

    async fn request(&self, token: &str, body: &str, extra: &str) -> BufReader<TcpStream> {
        let socket = TcpSocket::new_v4().unwrap();
        socket.set_recv_buffer_size(64 * 1024).unwrap();
        let mut client = socket.connect(self.address).await.unwrap();
        client
            .write_all(
                format!(
                    "POST /v1/chat/completions HTTP/1.1\r\nHost: {}\r\nAuthorization: Bearer {token}\r\nContent-Type: application/json\r\nContent-Length: {}\r\n{extra}\r\n{body}",
                    self.address,
                    body.len(),
                )
                .as_bytes(),
            )
            .await
            .unwrap();
        BufReader::new(client)
    }

    async fn upstream_request(&self, expected: &str) -> BufReader<TcpStream> {
        timeout(GUARD, async {
            let mut upstream = BufReader::new(self.upstream.accept().await.unwrap().0);
            let mut head = String::new();
            while !head.ends_with("\r\n\r\n") {
                assert!(head.len() < 4096);
                assert_ne!(upstream.read_line(&mut head).await.unwrap(), 0);
            }
            let head = head.to_ascii_lowercase();
            assert!(head.starts_with("post /fixture http/1.1\r\n"));
            assert!(head.contains("\r\nauthorization: bearer capacity-fixture\r\n"));
            let mut body = vec![0; expected.len()];
            upstream.read_exact(&mut body).await.unwrap();
            assert_eq!(body, expected.as_bytes());
            upstream
        })
        .await
        .expect("the admitted request did not reach upstream")
    }

    async fn assert_no_upstream(&self) {
        assert!(
            timeout(QUIET, self.upstream.accept()).await.is_err(),
            "a saturated route reached upstream before an active permit was released"
        );
    }

    async fn wait_for_body_permits(&self, expected: usize) {
        timeout(GUARD, async {
            while self.listener.available_body_permits() != expected {
                tokio::time::sleep(Duration::from_millis(5)).await;
            }
        })
        .await
        .expect("body permits did not return to the expected count");
    }
}

async fn status(socket: &mut BufReader<TcpStream>, code: u16) {
    let mut line = String::new();
    timeout(GUARD, socket.read_line(&mut line)).await.unwrap().unwrap();
    assert!(line.starts_with(&format!("HTTP/1.1 {code} ")), "{line:?}");
}

async fn json_success(upstream: &mut BufReader<TcpStream>) {
    upstream
        .write_all(b"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: 2\r\nConnection: close\r\n\r\n{}")
        .await
        .unwrap();
}

async fn start_stream(upstream: &mut BufReader<TcpStream>) {
    upstream
        .write_all(
            format!(
                "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nTransfer-Encoding: chunked\r\n\r\n{:x}\r\n{OUTPUT}\r\n",
                OUTPUT.len(),
            )
            .as_bytes(),
        )
        .await
        .unwrap();
}

#[tokio::test]
async fn cc_route_023_capacity_expected_behavior() {
    let fixture = Fixture::start().await;
    let token = fixture.issue("capacity-a");
    let other = fixture.issue("capacity-b");
    let capacity = fixture.listener.available_body_permits();

    let mut first = fixture.request(&token, STREAM, "").await;
    let mut first_upstream = fixture.upstream_request(STREAM).await;
    start_stream(&mut first_upstream).await;
    status(&mut first, 200).await;

    let mut second = fixture.request(&token, JSON, "").await;
    let mut second_upstream = fixture.upstream_request(JSON).await;
    let mut waiting = fixture.request(&token, JSON, "").await;
    fixture.assert_no_upstream().await;
    assert!(
        timeout(QUIET, waiting.fill_buf()).await.is_err(),
        "the bounded queue rejected immediately"
    );

    let mut overflow = fixture
        .request(&token, JSON, "X-Project-Id: capacity-b\r\nUser-Agent: alternate-client\r\n")
        .await;
    status(&mut overflow, 429).await;

    // The same listener snapshot used by the management status handler remains available.
    let control = fixture.listener.try_control().unwrap();
    let (state, binding) = fixture.listener.snapshot();
    assert_eq!(state, LifecycleState::Running);
    assert_eq!(binding.unwrap().0, fixture.address);
    drop(control);

    let mut independent = fixture.request(&other, JSON, "").await;
    json_success(&mut fixture.upstream_request(JSON).await).await;
    status(&mut independent, 200).await;
    status(&mut waiting, 429).await;
    fixture.wait_for_body_permits(capacity - 2).await;
    fixture.assert_no_upstream().await;

    let cancelled = fixture.request(&token, JSON, "").await;
    fixture.wait_for_body_permits(capacity - 3).await;
    fixture.assert_no_upstream().await;
    drop(cancelled);
    fixture.wait_for_body_permits(capacity - 2).await;

    let mut waiting = fixture.request(&token, JSON, "").await;
    fixture.assert_no_upstream().await;
    second_upstream
        .write_all(b"HTTP/1.1 500 Server Error\r\nContent-Length: 0\r\nConnection: close\r\n\r\n")
        .await
        .unwrap();
    status(&mut second, 502).await;
    let mut admitted = fixture.upstream_request(JSON).await;

    let mut waiting_after_cancel = fixture.request(&token, JSON, "").await;
    fixture.assert_no_upstream().await;
    drop(first);
    let mut admitted_after_cancel = fixture.upstream_request(JSON).await;
    json_success(&mut admitted).await;
    json_success(&mut admitted_after_cancel).await;
    status(&mut waiting, 200).await;
    status(&mut waiting_after_cancel, 200).await;
    fixture.wait_for_body_permits(capacity).await;

    // Both successful responses returned their upstream permits as well.
    let mut one = fixture.request(&token, JSON, "").await;
    let mut one_upstream = fixture.upstream_request(JSON).await;
    let mut two = fixture.request(&token, JSON, "").await;
    let mut two_upstream = fixture.upstream_request(JSON).await;
    json_success(&mut one_upstream).await;
    json_success(&mut two_upstream).await;
    status(&mut one, 200).await;
    status(&mut two, 200).await;
    fixture.wait_for_body_permits(capacity).await;
    fixture.listener.stop().await.unwrap();
}

#[tokio::test]
async fn cc_route_023_slow_stream_client_backpressures_both_transports() {
    let fixture = Fixture::start().await;
    let token = fixture.issue("slow-client");
    let capacity = fixture.listener.available_body_permits();
    let payload = "x".repeat(16 * 1024);
    let frame = format!("data: {{\"choices\":[{{\"delta\":{{\"content\":\"{payload}\"}}}}]}}\n\n");
    let chunk = format!("{:x}\r\n{frame}\r\n", frame.len());
    const FRAMES: usize = 1024;

    for header in ["", "User-Agent: ordered-client\r\n"] {
        let mut client = fixture.request(&token, STREAM, header).await;
        let mut upstream = fixture.upstream_request(STREAM).await;
        start_stream(&mut upstream).await;
        status(&mut client, 200).await;

        let send = async {
            for _ in 0..FRAMES {
                upstream.write_all(chunk.as_bytes()).await?;
            }
            upstream.write_all(b"e\r\ndata: [DONE]\n\n\r\n0\r\n\r\n").await
        };
        tokio::pin!(send);
        assert!(
            timeout(Duration::from_millis(200), &mut send).await.is_err(),
            "upstream wrote the entire stream while the downstream socket was idle"
        );
        let mut sink = tokio::io::sink();
        let (_, received) = timeout(Duration::from_secs(12), async {
            tokio::try_join!(&mut send, tokio::io::copy(&mut client, &mut sink))
        })
        .await
        .expect("upstream did not resume when downstream started reading")
        .unwrap();
        assert!(received >= (FRAMES * frame.len()) as u64);
        fixture.wait_for_body_permits(capacity).await;
    }
    fixture.listener.stop().await.unwrap();
}
