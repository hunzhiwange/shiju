use super::{
    RoutingListener, SharedTransport, dispatch::JsonTarget, route_auth::VerifiedRouteBinding,
};
use std::{io, net::SocketAddr, time::Duration};
use tokio::{
    io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader},
    net::{TcpListener, TcpStream},
    time::timeout,
};
use vw_api_types::{
    agent_apps::AppId,
    id::{ProjectScope, TenantScope},
};
use vw_gateway_client::{GatewayAuth, GatewayClient, GatewayEndpoint};

const GUARD: Duration = Duration::from_secs(3);
const REQUEST: &str = r#"{"model":"fixture","stream":true}"#;
const OUTPUT: &str = "data: {\"choices\":[{\"delta\":{\"content\":\"output\"}}]}\n\n";

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

    fn issue(&self) -> (String, impl Fn() + '_) {
        self.listener
            .auth
            .issue_revocable_test_token(
                VerifiedRouteBinding::new(
                    ProjectScope::new(
                        TenantScope::new("platform", "company").unwrap(),
                        "cancel-project".into(),
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
                    authorization: "Bearer cancellation-fixture".parse().unwrap(),
                    transport: SharedTransport::new().unwrap(),
                    request_options: Default::default(),
                },
            )
            .unwrap()
    }

    async fn request(&self, token: &str, extra: &str, pipeline: bool) -> BufReader<TcpStream> {
        let mut client = TcpStream::connect(self.address).await.unwrap();
        let head = format!(
            "POST /v1/chat/completions HTTP/1.1\r\nHost: {}\r\nAuthorization: Bearer {token}\r\nContent-Type: application/json\r\nContent-Length: {}\r\n{extra}\r\n",
            self.address,
            REQUEST.len()
        );
        let mut request = format!("{head}{REQUEST}");
        if pipeline {
            request.push_str(&head);
            request.push_str(REQUEST);
        }
        client.write_all(request.as_bytes()).await.unwrap();
        BufReader::new(client)
    }

    async fn upstream_request(&self) -> BufReader<TcpStream> {
        timeout(GUARD, async {
            let mut upstream = BufReader::new(self.upstream.accept().await.unwrap().0);
            let mut head = String::new();
            while !head.ends_with("\r\n\r\n") {
                assert!(head.len() < 4096);
                assert_ne!(upstream.read_line(&mut head).await.unwrap(), 0);
            }
            let head = head.to_ascii_lowercase();
            assert!(head.starts_with("post /fixture http/1.1\r\n"));
            assert!(head.contains("\r\nauthorization: bearer cancellation-fixture\r\n"));
            let mut body = vec![0; REQUEST.len()];
            upstream.read_exact(&mut body).await.unwrap();
            assert_eq!(body, REQUEST.as_bytes());
            upstream
        })
        .await
        .expect("the route did not send an upstream request")
    }

    async fn assert_no_replay(&self) {
        assert!(
            timeout(Duration::from_millis(100), self.upstream.accept()).await.is_err(),
            "cancellation replayed the request"
        );
    }
}

async fn start_stream(upstream: &mut BufReader<TcpStream>) {
    upstream
        .write_all(
            format!(
                "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nTransfer-Encoding: chunked\r\n\r\n{:x}\r\n{OUTPUT}\r\n",
                OUTPUT.len()
            )
            .as_bytes(),
        )
        .await
        .unwrap();
}

async fn start_stream_chunk(upstream: &mut BufReader<TcpStream>) {
    upstream.write_all(format!("{:x}\r\n{OUTPUT}\r\n", OUTPUT.len()).as_bytes()).await.unwrap();
}

async fn assert_closed(socket: &mut BufReader<TcpStream>, reason: &str) {
    match timeout(GUARD, socket.read_u8()).await {
        Ok(Err(error))
            if matches!(
                error.kind(),
                io::ErrorKind::UnexpectedEof | io::ErrorKind::ConnectionReset
            ) => {}
        result => panic!("{reason}: socket remained open or received more data: {result:?}"),
    }
}

async fn read_output(client: &mut BufReader<TcpStream>) -> String {
    let mut response = String::new();
    timeout(GUARD, async {
        while !response.ends_with(OUTPUT) {
            assert!(response.len() < 4096);
            assert_ne!(client.read_line(&mut response).await.unwrap(), 0);
        }
    })
    .await
    .unwrap();
    response
}

#[tokio::test]
async fn cc_route_022_cancellation_expected_behavior() {
    let fixture = Fixture::start().await;
    let (token, _) = fixture.issue();
    let capacity = fixture.listener.available_body_permits();
    for (extra, pipeline) in [
        ("Connection: close\r\n", false),
        ("Connection: close\r\nUser-Agent: ordered\r\n", false),
        ("", true),
        ("User-Agent: ordered\r\n", true),
    ] {
        let client = fixture.request(&token, extra, pipeline).await;
        let mut upstream = fixture.upstream_request().await;
        assert_eq!(fixture.listener.available_body_permits(), capacity - 1);
        // No upstream bytes arrive to drive a downstream write after disconnect.
        drop(client);
        assert_closed(&mut upstream, "client disconnected before first output").await;
        assert_eq!(fixture.listener.available_body_permits(), capacity);

        let mut client = fixture.request(&token, extra, pipeline).await;
        let mut upstream = fixture.upstream_request().await;
        start_stream(&mut upstream).await;
        assert!(read_output(&mut client).await.starts_with("HTTP/1.1 200 "));
        assert_eq!(fixture.listener.available_body_permits(), capacity - 1);
        drop(client);
        assert_closed(&mut upstream, "client disconnected during idle SSE").await;
        assert_eq!(fixture.listener.available_body_permits(), capacity);
    }
    fixture.assert_no_replay().await;
    fixture.listener.stop().await.unwrap();
}

#[tokio::test]
async fn cc_route_022_sdk_cancel_and_credential_revocation_release_each_request_once() {
    let fixture = Fixture::start().await;
    let (token, revoke) = fixture.issue();
    let capacity = fixture.listener.available_body_permits();

    // The existing GatewayClient inference call stays pending before the
    // first SSE output. Cancel the caller's task without changing the grant.
    let endpoint = GatewayEndpoint::new(fixture.address.ip().to_string(), fixture.address.port())
        .with_auth(GatewayAuth { skey: Some(token.clone()) });
    let sdk = GatewayClient::new(endpoint).unwrap();
    let request = tokio::spawn(async move {
        let payload = serde_json::from_str(REQUEST).unwrap();
        sdk.inference_openai_chat(&payload).await
    });
    let mut sdk_upstream = fixture.upstream_request().await;
    let mut client = fixture.request(&token, "User-Agent: ordered\r\n", false).await;
    let mut upstream = fixture.upstream_request().await;
    start_stream(&mut upstream).await;
    assert!(read_output(&mut client).await.starts_with("HTTP/1.1 200 "));
    assert_eq!(fixture.listener.available_body_permits(), capacity - 2);

    request.abort();
    assert!(request.await.unwrap_err().is_cancelled());
    assert_closed(&mut sdk_upstream, "SDK call cancelled").await;
    assert_eq!(fixture.listener.available_body_permits(), capacity - 1);

    // The sibling on the same grant is unaffected until explicit revocation.
    start_stream_chunk(&mut upstream).await;
    assert!(read_output(&mut client).await.ends_with(OUTPUT));

    revoke();
    revoke();
    assert_closed(&mut upstream, "credential revoked during idle SSE").await;
    assert_eq!(fixture.listener.available_body_permits(), capacity);
    let mut denied = fixture.request(&token, "", false).await;
    let mut response = String::new();
    timeout(GUARD, denied.read_line(&mut response)).await.unwrap().unwrap();
    assert!(response.starts_with("HTTP/1.1 401 "));
    fixture.assert_no_replay().await;
    fixture.listener.stop().await.unwrap();
}
