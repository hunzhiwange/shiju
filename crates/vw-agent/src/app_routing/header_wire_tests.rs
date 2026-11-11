use super::{JsonTarget, VerifiedRouteBinding};
use crate::app_routing::{RoutingListener, SharedTransport};
use serde_json::json;
use std::time::Duration;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt, BufReader},
    net::{TcpListener, TcpStream},
    time::timeout,
};
use vw_api_types::{
    agent_apps::AppId,
    id::{ProjectScope, TenantScope},
};

async fn read_head(reader: &mut BufReader<TcpStream>) -> String {
    let mut head = Vec::new();
    while !head.ends_with(b"\r\n\r\n") {
        assert!(head.len() < 4096);
        head.push(reader.read_u8().await.unwrap());
    }
    String::from_utf8(head).unwrap()
}

#[tokio::test]
async fn cc_route_017_header_wire_expected_behavior() {
    timeout(Duration::from_secs(10), async {
        let upstream = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("http://{}/fixture", upstream.local_addr().unwrap()).parse().unwrap();
        let captured = tokio::spawn(async move {
            let (socket, _) = upstream.accept().await.unwrap();
            let mut socket = BufReader::new(socket);
            let head = read_head(&mut socket).await;
            let length = head
                .lines()
                .filter_map(|line| line.split_once(": "))
                .find(|(name, _)| name.eq_ignore_ascii_case("content-length"))
                .unwrap()
                .1
                .parse::<usize>()
                .unwrap();
            assert!(length < 1024);
            let mut body = vec![0; length];
            socket.read_exact(&mut body).await.unwrap();
            socket
                .write_all(b"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: 2\r\nConnection: close, X-Response-Hop\r\nX-Response-Hop: hidden\r\nSet-Cookie: hidden\r\n\r\n{}")
                .await
                .unwrap();
            (head, body)
        });
        let listener = RoutingListener::default();
        let address = listener.start(0).unwrap();
        let grant = listener
            .auth
            .issue_with_target(
                VerifiedRouteBinding {
                    project: ProjectScope::new(
                        TenantScope::new("platform", "company").unwrap(),
                        "project".into(),
                    )
                    .unwrap(),
                    host: "host".into(),
                    app: AppId::Codex,
                    account: "account".into(),
                    account_generation: 1,
                },
                Some(JsonTarget {
                    #[cfg(target_os = "macos")]
                    managed: None,
                    url,
                    authorization: "Bearer provider-fixture".parse().unwrap(),
                    transport: SharedTransport::new().unwrap(),
                    request_options: Default::default(),
                }),
            )
            .unwrap();

        let body = br#" { "model": "fixture", "_private": true } "#;
        let mut socket = TcpStream::connect(address).await.unwrap();
        let request = format!(
            "POST /v1/chat/completions HTTP/1.1\r\nHost: {address}\r\nX-Api-Key: {}\r\nX-Goog-Api-Key: client-fixture\r\nCookie: hidden\r\nConnection: x-remove\r\nConnection: X-Other, USEr-agenT, Authorization\r\nX-Remove: hidden\r\nX-Other: hidden\r\nUSEr-agenT: nominated\r\nAcCePt: application/json\r\nACCEPT-Language: zh-CN\r\naccePT: text/plain;q=0.1\r\nContent-Type: application/json\r\nContent-Length: {}\r\nContent-MD5: stale\r\n\r\n",
            grant.token,
            body.len(),
        );
        let split = request.find("ACCEPT-Language").unwrap() + 2;
        socket.write_all(&request.as_bytes()[..split]).await.unwrap();
        tokio::time::sleep(Duration::from_millis(10)).await;
        socket.write_all(&request.as_bytes()[split..]).await.unwrap();
        socket.write_all(body).await.unwrap();

        let mut response = String::new();
        socket.read_to_string(&mut response).await.unwrap();
        let (head, outgoing) = captured.await.unwrap();
        listener.stop().await.unwrap();

        assert!(response.starts_with("HTTP/1.1 200 "));
        assert!(response.ends_with("\r\n{}\r\n0\r\n\r\n") || response.ends_with("\r\n\r\n{}"));
        let response = response.to_ascii_lowercase();
        assert!(!response.contains("x-response-hop:") && !response.contains("set-cookie:"));

        let lower = head.to_ascii_lowercase();
        for stripped in ["x-api-key", "x-goog-api-key", "cookie", "user-agent", "x-remove", "x-other", "content-md5"] {
            assert!(!lower.contains(&format!("\r\n{stripped}:")), "{stripped} leaked");
        }
        let connections: Vec<_> = lower.lines()
            .filter(|line| line.starts_with("connection:"))
            .collect();
        assert_eq!(connections, ["connection: close"]);
        assert!(!head.contains(&grant.token));
        assert_eq!(lower.matches("\r\nauthorization:").count(), 1);
        assert!(head.lines().any(|line| line.split_once(": ").is_some_and(|(name, value)| {
            name.eq_ignore_ascii_case("authorization") && value == "Bearer provider-fixture"
        })));
        assert_eq!(lower.matches("\r\ncontent-length:").count(), 1);
        assert!(lower.contains(&format!("\r\ncontent-length: {}\r\n", outgoing.len())));
        assert_ne!(outgoing.len(), body.len());
        assert_eq!(serde_json::from_slice::<serde_json::Value>(&outgoing).unwrap(), json!({"model": "fixture"}));

        let accepted: Vec<_> = head
            .lines()
            .filter(|line| line.to_ascii_lowercase().starts_with("accept"))
            .filter(|line| !line.to_ascii_lowercase().starts_with("accept-encoding"))
            .collect();
        assert_eq!(accepted, ["AcCePt: application/json", "ACCEPT-Language: zh-CN", "accePT: text/plain;q=0.1"]);
    })
    .await
    .expect("routing wire fixture timed out");
}
