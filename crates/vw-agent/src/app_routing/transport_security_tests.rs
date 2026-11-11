use super::{
    RoutingListener, SharedTransport,
    outbound_custom_tests::{
        PROXY_CA, complete_upstream, fixture, issue, project, read_head, request, tls_fixture,
    },
    outbound_system::SystemProxy,
};
use axum::http::{StatusCode, header};
use std::{net::Ipv6Addr, time::Duration};
use tokio::{io::AsyncWriteExt, net::TcpListener, time::timeout};
use tokio_util::sync::CancellationToken;

const DEADLINE: Duration = Duration::from_secs(5);

fn direct() -> SharedTransport {
    SharedTransport::with_system_proxy(SystemProxy::from_settings(None, |_| None)).unwrap()
}

#[tokio::test]
async fn cc_route_027_transport_security_expected_behavior() {
    timeout(DEADLINE, async {
        let listener = RoutingListener::default();
        let address = listener.start(0).unwrap();
        let upstream = TcpListener::bind((Ipv6Addr::LOCALHOST, address.port())).await.unwrap();
        let target = format!("http://{}/fixture", upstream.local_addr().unwrap());
        let token = issue(&listener, &project(), "mac", &target, direct());

        let response = request(address, &token);
        tokio::pin!(response);
        let (mut socket, _) = tokio::select! {
            accepted = upstream.accept() => accepted.unwrap(),
            response = &mut response => panic!("separate IPv6 upstream was rejected: {}", response.0),
        };
        let head = read_head(&mut socket).await;
        complete_upstream(&mut socket, &head, "ipv6").await;
        assert_eq!(response.await.0, 200);
        listener.stop().await.unwrap();
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn cc_route_027_rejects_unverifiable_proxy_destination_before_dialing() {
    timeout(DEADLINE, async {
        let listener = RoutingListener::default();
        let address = listener.start(0).unwrap();
        let proxy = fixture().await;
        let transport = SharedTransport::with_custom_proxy(&format!(
            "http://fixture:test@{}",
            proxy.local_addr().unwrap()
        ))
        .unwrap();
        let target = format!("http://provider.invalid:{}/fixture", address.port());
        let token = issue(&listener, &project(), "mac", &target, transport);
        tokio::select! {
            biased;
            _ = proxy.accept() => panic!("an unverifiable same-port destination reached the proxy"),
            response = request(address, &token) => assert_eq!(response.0, 502),
        }
        listener.stop().await.unwrap();
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn cc_route_027_redirect_never_reaches_another_target() {
    timeout(DEADLINE, async {
        let listener = RoutingListener::default();
        let address = listener.start(0).unwrap();
        let upstream = fixture().await;
        let destination = fixture().await;
        let target = format!("http://{}/fixture", upstream.local_addr().unwrap()).parse().unwrap();
        let location = format!("http://{}/other", destination.local_addr().unwrap());
        let server = async {
            for _ in 0..2 {
                let (mut socket, _) = upstream.accept().await.unwrap();
                let head = read_head(&mut socket).await;
                assert!(head.to_ascii_lowercase().contains("authorization: bearer provider-fixture\r\n"));
                socket.write_all(format!(
                    "HTTP/1.1 307 Temporary Redirect\r\nLocation: {location}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
                ).as_bytes()).await.unwrap();
            }
        };
        let client = async {
            let transport = direct();
            let mut outbound = reqwest::Request::new(reqwest::Method::GET, target);
            outbound.headers_mut().insert(header::AUTHORIZATION, "Bearer provider-fixture".parse().unwrap());
            let response = transport.forward(&outbound.url().clone(), outbound, CancellationToken::new()).await.unwrap();
            assert_eq!(response.status(), StatusCode::TEMPORARY_REDIRECT);

            let token = issue(&listener, &project(), "mac", &format!("http://{}/fixture", upstream.local_addr().unwrap()), transport);
            assert_eq!(request(address, &token).await.0, 502);
        };
        tokio::select! {
            biased;
            _ = destination.accept() => panic!("redirect destination received a connection"),
            () = async { tokio::join!(server, client); } => {}
        }
        listener.stop().await.unwrap();
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn cc_route_027_rejects_untrusted_upstream_tls() {
    timeout(DEADLINE, async {
        let listener = RoutingListener::default();
        let address = listener.start(0).unwrap();
        let upstream = fixture().await;
        let target: reqwest::Url =
            format!("https://{}/fixture", upstream.local_addr().unwrap()).parse().unwrap();
        let token = issue(&listener, &project(), "mac", target.as_str(), direct());
        let (acceptor, _) = tls_fixture();
        let server = async {
            for _ in 0..2 {
                let (tcp, _) = upstream.accept().await.unwrap();
                assert!(acceptor.accept(tcp).await.is_err(), "untrusted TLS handshake succeeded");
            }
        };
        let clients = async {
            assert_eq!(request(address, &token).await.0, 502);
            let response = direct()
                .forward(
                    &target,
                    reqwest::Request::new(reqwest::Method::GET, target.clone()),
                    CancellationToken::new(),
                )
                .await;
            assert_eq!(response.unwrap_err(), StatusCode::BAD_GATEWAY);
        };
        tokio::join!(server, clients);
        listener.stop().await.unwrap();
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn cc_route_027_valid_chain_requires_matching_tls_name() {
    timeout(DEADLINE, async {
        let upstream = fixture().await;
        let address = upstream.local_addr().unwrap();
        let (acceptor, _) = tls_fixture();
        let server = async {
            let (tcp, _) = upstream.accept().await.unwrap();
            let mut tls = acceptor.accept(tcp).await.unwrap();
            let _ = read_head(&mut tls).await;
            tls.write_all(b"HTTP/1.1 204 No Content\r\nConnection: close\r\n\r\n").await.unwrap();
            let (tcp, _) = upstream.accept().await.unwrap();
            assert!(acceptor.accept(tcp).await.is_err(), "wrong TLS name was accepted");
        };
        let client = async {
            let certificate = reqwest::Certificate::from_pem(PROXY_CA).unwrap();
            let client = crate::model_relay::upstream::upstream_http_client_builder()
                .no_proxy()
                .tls_certs_only([certificate])
                .resolve("wrong-name.invalid", address)
                .build()
                .unwrap();
            assert_eq!(
                client.get(format!("https://{address}")).send().await.unwrap().status(),
                204
            );
            assert!(
                client
                    .get(format!("https://wrong-name.invalid:{}", address.port()))
                    .send()
                    .await
                    .is_err()
            );
        };
        tokio::join!(server, client);
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn cc_route_027_rejects_self_after_name_resolution() {
    timeout(DEADLINE, async {
        let bound = fixture().await;
        let address = bound.local_addr().unwrap();
        let target: reqwest::Url =
            format!("http://localhost:{}/fixture", address.port()).parse().unwrap();
        let mut request = reqwest::Request::new(reqwest::Method::POST, target.clone());
        request
            .headers_mut()
            .insert(header::AUTHORIZATION, "Bearer provider-fixture".parse().unwrap());
        let transport = direct();
        tokio::select! {
            biased;
            _ = bound.accept() => panic!("the resolved destination reached the bound listener"),
            result = transport.forward_ordered(
                &target,
                request,
                super::header_wire::HeaderWire::default(),
                CancellationToken::new(),
                address,
            ) => assert_eq!(result.unwrap_err(), StatusCode::BAD_GATEWAY),
        }
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn cc_route_027_ordered_tls_checks_the_original_target_name() {
    timeout(DEADLINE, async {
        let upstream = fixture().await;
        let address = upstream.local_addr().unwrap();
        let (acceptor, connector) = tls_fixture();
        let server = async {
            let (tcp, _) = upstream.accept().await.unwrap();
            let _tls = acceptor.accept(tcp).await.unwrap();
            let (tcp, _) = upstream.accept().await.unwrap();
            assert!(acceptor.accept(tcp).await.is_err());
        };
        let client = async {
            use super::transport_security::connect_tls;
            use tokio::net::TcpStream;

            let good = TcpStream::connect(address).await.unwrap();
            assert!(connect_tls(&connector, "localhost", good).await.is_ok());
            let bad = TcpStream::connect(address).await.unwrap();
            assert_eq!(
                connect_tls(&connector, "wrong-name.invalid", bad).await.unwrap_err(),
                StatusCode::BAD_GATEWAY,
            );
        };
        tokio::join!(server, client);
    })
    .await
    .unwrap();
}
