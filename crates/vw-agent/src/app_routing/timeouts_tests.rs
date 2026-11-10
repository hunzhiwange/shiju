use super::{
    RoutingListener, SharedTransport,
    config_apply::{AppTimeouts, RoutingConfigStore},
    dispatch::JsonTarget,
    route_auth::VerifiedRouteBinding,
};
use crate::global_database::GlobalDatabase;
use axum::{
    body::Body,
    http::{Request, StatusCode, header},
};
use futures_util::StreamExt;
use hyper::{Response, body::Incoming};
use hyper_util::rt::TokioIo;
use serde_json::{Value, json};
use std::{future::Future, net::SocketAddr, time::Duration};
use tokio::{
    io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader},
    net::{TcpListener, TcpStream},
    task::JoinHandle,
    time::{advance, pause, resume, sleep, timeout},
};
use vw_api_types::{
    agent_apps::AppId,
    id::{ProjectScope, TenantScope},
};

const GUARD: Duration = Duration::from_secs(4);
const STREAM_HEAD: &[u8] =
    b"HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nTransfer-Encoding: chunked\r\n\r\n";
const HEARTBEAT: &[u8] =
    b": keepalive\r\n\r\nevent: ping\r\ndata: {}\r\n\r\ndata: {\"type\":\"heartbeat\"}\n\n";

struct Fixture {
    listener: RoutingListener,
    address: SocketAddr,
    token: String,
    upstream: TcpListener,
    store: RoutingConfigStore,
    project: ProjectScope,
}

impl Fixture {
    async fn start() -> Self {
        let upstream = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let listener = RoutingListener::default();
        let address = listener.start(0).unwrap();
        let tenant = TenantScope::new("platform", "company").unwrap();
        let project = ProjectScope::new(tenant.clone(), "project".into()).unwrap();
        let database = GlobalDatabase::for_home(&crate::global::paths().home).unwrap();
        let store = RoutingConfigStore::new(database, tenant, "host".into()).unwrap();
        let token = listener
            .auth
            .issue_test_token(
                VerifiedRouteBinding::new(
                    project.clone(),
                    "host".into(),
                    AppId::Codex,
                    "account".into(),
                    1,
                ),
                JsonTarget {
                    url: format!("http://{}/fixture", upstream.local_addr().unwrap())
                        .parse()
                        .unwrap(),
                    authorization: "Bearer upstream-fixture".parse().unwrap(),
                    transport: SharedTransport::new().unwrap(),
                    request_options: Default::default(),
                },
            )
            .unwrap();
        Self { listener, address, token, upstream, store, project }
    }

    fn save(&self, timeouts: AppTimeouts) {
        self.store.save_app_timeouts(&self.project, AppId::Codex, timeouts).unwrap();
    }

    fn request(&self, stream: bool, ordered: bool) -> JoinHandle<Response<Incoming>> {
        let address = self.address;
        let mut request = Request::post("/v1/chat/completions")
            .header(header::HOST, address.to_string())
            .header(header::AUTHORIZATION, format!("Bearer {}", self.token))
            .header(header::CONTENT_TYPE, "application/json")
            .header(header::CONNECTION, "close");
        if ordered {
            request = request.header(header::USER_AGENT, "wire-order-fixture");
        }
        let request = request
            .body(Body::from(json!({"model": "fixture", "stream": stream}).to_string()))
            .unwrap();
        tokio::spawn(async move {
            let socket = TcpStream::connect(address).await.unwrap();
            let (mut sender, connection) =
                hyper::client::conn::http1::handshake(TokioIo::new(socket)).await.unwrap();
            tokio::spawn(connection);
            sender.send_request(request).await.unwrap()
        })
    }

    async fn upstream_request(&self, stream: bool) -> BufReader<TcpStream> {
        timeout(Duration::from_secs(3), async {
            let (socket, _) = self.upstream.accept().await.unwrap();
            let mut socket = BufReader::new(socket);
            let mut head = String::new();
            while !head.ends_with("\r\n\r\n") {
                assert!(head.len() < 4096);
                assert_ne!(socket.read_line(&mut head).await.unwrap(), 0);
            }
            let head = head.to_ascii_lowercase();
            assert!(head.starts_with("post /fixture http/1.1\r\n"));
            assert!(head.contains("\r\nauthorization: bearer upstream-fixture\r\n"));
            if stream {
                assert!(head.contains("\r\naccept-encoding: identity\r\n"));
            }
            let len = head
                .lines()
                .find_map(|line| line.strip_prefix("content-length: "))
                .unwrap()
                .parse()
                .unwrap();
            let mut body = vec![0; len];
            socket.read_exact(&mut body).await.unwrap();
            assert_eq!(
                serde_json::from_slice::<Value>(&body).unwrap(),
                json!({"model": "fixture", "stream": stream})
            );
            socket
        })
        .await
        .expect("the authorized request did not reach the upstream")
    }
}

async fn assert_upstream_closed(upstream: &mut BufReader<TcpStream>) {
    let mut byte = [0];
    match timeout(GUARD, upstream.read(&mut byte)).await {
        Ok(Ok(0)) => {}
        Ok(Err(error)) if error.kind() == std::io::ErrorKind::ConnectionReset => {}
        result => panic!("upstream socket remained open: {result:?}"),
    }
}

#[tokio::test]
async fn cc_route_021_timeouts_expected_behavior() {
    let fixture = Fixture::start().await;
    fixture.save(AppTimeouts { non_streaming_timeout_secs: 180, ..AppTimeouts::default() });
    for ordered in [true, false] {
        let client = fixture.request(false, ordered);
        let mut upstream = fixture.upstream_request(false).await;
        pause();
        advance(Duration::from_secs(100)).await;
        upstream
            .write_all(
                b"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: 64\r\n\r\n{",
            )
            .await
            .unwrap();
        advance(Duration::from_secs(70)).await;
        assert!(!client.is_finished(), "the configured total deadline has not elapsed");
        upstream.write_all(b" ").await.unwrap();
        advance(Duration::from_secs(11)).await;
        resume();
        let response = timeout(GUARD, client).await.unwrap().unwrap();
        assert_eq!(
            response.status(),
            StatusCode::GATEWAY_TIMEOUT,
            "a stalled JSON body should report the application total timeout"
        );
        assert_upstream_closed(&mut upstream).await;
    }
    fixture.listener.stop().await.unwrap();
}

async fn chunk(upstream: &mut BufReader<TcpStream>, bytes: &[u8]) -> std::io::Result<()> {
    upstream.write_all(format!("{:x}\r\n", bytes.len()).as_bytes()).await?;
    upstream.write_all(bytes).await?;
    upstream.write_all(b"\r\n").await
}

async fn with_heartbeats<T>(
    upstream: &mut BufReader<TcpStream>,
    until: impl Future<Output = T>,
) -> T {
    timeout(GUARD, async {
        tokio::pin!(until);
        loop {
            tokio::select! {
                biased;
                result = &mut until => break result,
                () = sleep(Duration::from_millis(50)) => {
                    if chunk(upstream, HEARTBEAT).await.is_err() {
                        break until.await;
                    }
                }
            }
        }
    })
    .await
    .expect("continuous heartbeats extended the deadline")
}

#[tokio::test]
async fn cc_route_021_stream_first_output_deadline_includes_headers_and_heartbeats() {
    let fixture = Fixture::start().await;
    fixture.save(AppTimeouts { first_byte_timeout_secs: 1, ..AppTimeouts::default() });

    let client = fixture.request(true, true);
    let mut upstream = fixture.upstream_request(true).await;
    let response = timeout(GUARD, client).await.unwrap().unwrap();
    assert_eq!(response.status(), StatusCode::GATEWAY_TIMEOUT);
    assert_upstream_closed(&mut upstream).await;

    let client = fixture.request(true, false);
    let mut upstream = fixture.upstream_request(true).await;
    upstream.write_all(STREAM_HEAD).await.unwrap();
    let response = with_heartbeats(&mut upstream, client).await.unwrap();
    assert_eq!(response.status(), StatusCode::GATEWAY_TIMEOUT);
    assert_upstream_closed(&mut upstream).await;
    fixture.listener.stop().await.unwrap();
}

#[tokio::test]
async fn cc_route_021_stream_heartbeats_do_not_extend_idle_after_output() {
    let fixture = Fixture::start().await;
    fixture.save(AppTimeouts {
        first_byte_timeout_secs: 2,
        idle_timeout_secs: 1,
        ..AppTimeouts::default()
    });
    let client = fixture.request(true, true);
    let mut upstream = fixture.upstream_request(true).await;
    upstream.write_all(STREAM_HEAD).await.unwrap();
    chunk(&mut upstream, HEARTBEAT).await.unwrap();
    chunk(&mut upstream, b"data: {\"choices\":[{\"delta\":{\"content\":\"model").await.unwrap();
    chunk(&mut upstream, b" output\"}}]}\r\n\r\n").await.unwrap();

    let response = timeout(GUARD, client).await.unwrap().unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.headers()[header::CONTENT_TYPE], "text/event-stream");
    let mut stream = Body::new(response.into_body()).into_data_stream();
    let prelude = timeout(GUARD, stream.next()).await.unwrap().unwrap().unwrap();
    assert!(prelude.starts_with(HEARTBEAT));
    assert!(prelude.ends_with(b"model output\"}}]}\r\n\r\n"));
    pause();
    advance(Duration::from_millis(650)).await;
    resume();
    let next_output = b"data: {\"choices\":[{\"delta\":{\"content\":\"more\"}}]}\n\n";
    chunk(&mut upstream, next_output).await.unwrap();
    assert_eq!(timeout(GUARD, stream.next()).await.unwrap().unwrap().unwrap(), &next_output[..]);
    pause();
    advance(Duration::from_millis(650)).await;
    resume();
    chunk(&mut upstream, HEARTBEAT).await.unwrap();
    assert!(timeout(GUARD, stream.next()).await.unwrap().unwrap().is_ok());
    with_heartbeats(&mut upstream, async {
        loop {
            match stream.next().await {
                Some(Ok(_)) => {}
                Some(Err(_)) => break,
                None => panic!("idle timeout was reported as a clean EOF"),
            }
        }
    })
    .await;
    assert_upstream_closed(&mut upstream).await;
    fixture.listener.stop().await.unwrap();
}

#[tokio::test]
async fn cc_route_021_stream_idle_zero_disables_idle_without_a_total_timeout() {
    let fixture = Fixture::start().await;
    fixture.save(AppTimeouts {
        first_byte_timeout_secs: 2,
        idle_timeout_secs: 0,
        ..AppTimeouts::default()
    });
    let client = fixture.request(true, false);
    let mut upstream = fixture.upstream_request(true).await;
    upstream.write_all(STREAM_HEAD).await.unwrap();
    let output = b"data: {\"choices\":[{\"delta\":{\"content\":\"model output\"}}]}\n\n";
    chunk(&mut upstream, output).await.unwrap();
    let response = timeout(GUARD, client).await.unwrap().unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let mut stream = Body::new(response.into_body()).into_data_stream();
    assert_eq!(timeout(GUARD, stream.next()).await.unwrap().unwrap().unwrap(), &output[..]);

    pause();
    advance(Duration::from_secs(601)).await;
    resume();
    chunk(&mut upstream, b"data: [DONE]\n\n").await.unwrap();
    assert_eq!(timeout(GUARD, stream.next()).await.unwrap().unwrap().unwrap(), "data: [DONE]\n\n");
    assert!(timeout(GUARD, stream.next()).await.unwrap().is_none());
    assert_upstream_closed(&mut upstream).await;
    fixture.listener.stop().await.unwrap();
}
