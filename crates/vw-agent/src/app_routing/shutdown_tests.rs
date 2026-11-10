use super::{ListenerError, RoutingListener, SharedTransport};
use axum::{body::Body, response::Response};
use hyper::{server::conn::http1, service::service_fn};
use hyper_util::rt::TokioIo;
use std::{convert::Infallible, io::ErrorKind, net::SocketAddr, sync::Arc, time::Duration};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    sync::{mpsc, oneshot},
    time::timeout,
};
use tokio_util::sync::CancellationToken;
use vw_agent_apps::routing::lifecycle_state::LifecycleState;

const DEADLINE: Duration = Duration::from_secs(10);
const LOOPBACK: SocketAddr =
    SocketAddr::new(std::net::IpAddr::V4(std::net::Ipv4Addr::LOCALHOST), 0);

#[tokio::test]
async fn cc_route_006_shutdown_expected_behavior() {
    timeout(DEADLINE, async {
        let upstream = TcpListener::bind(LOOPBACK).await.unwrap();
        let upstream_address = upstream.local_addr().unwrap();
        let (received, mut requests) = mpsc::channel(2);
        let upstream_task = tokio::spawn(async move {
            let (sse, _) = upstream.accept().await.unwrap();
            let sse = tokio::spawn(slow_upstream(sse, true, received.clone()));
            let (http, _) = upstream.accept().await.unwrap();
            slow_upstream(http, false, received).await;
            sse.await.unwrap();
        });

        let transport = Arc::new(SharedTransport::new().unwrap());
        let connection_transport = Arc::clone(&transport);
        let (accepted, mut connections) = mpsc::channel(3);
        let listener = RoutingListener::default();
        let address = listener
            .start_at(LOOPBACK, move |connection| {
                accepted.try_send(()).unwrap();
                let transport = Arc::clone(&connection_transport);
                Some(async move {
                    let service =
                        service_fn(move |request: hyper::Request<hyper::body::Incoming>| {
                            let transport = Arc::clone(&transport);
                            let url: reqwest::Url =
                                format!("http://{upstream_address}{}", request.uri().path())
                                    .parse()
                                    .unwrap();
                            async move {
                                let request =
                                    reqwest::Request::new(reqwest::Method::GET, url.clone());
                                let response = transport
                                    .forward(&url, request, CancellationToken::new())
                                    .await
                                    .unwrap_or_else(|status| {
                                        Response::builder()
                                            .status(status)
                                            .body(Body::empty())
                                            .unwrap()
                                    });
                                Ok::<_, Infallible>(response)
                            }
                        });
                    let _ = http1::Builder::new()
                        .serve_connection(TokioIo::new(connection), service)
                        .await;
                })
            })
            .unwrap();

        let client = reqwest::Client::builder().no_proxy().build().unwrap();
        let mut sse = client.get(format!("http://{address}/sse")).send().await.unwrap();
        assert_eq!(sse.headers()["content-type"], "text/event-stream");
        assert_eq!(sse.chunk().await.unwrap().unwrap(), "data: ready\n\n");
        assert_eq!(requests.recv().await.as_deref(), Some("GET /sse HTTP/1.1"));

        let pending = tokio::spawn(client.get(format!("http://{address}/slow")).send());
        assert_eq!(requests.recv().await.as_deref(), Some("GET /slow HTTP/1.1"));

        // Also leave a connection idle before it sends any request headers.
        let mut idle = TcpStream::connect(address).await.unwrap();
        for _ in 0..3 {
            connections.recv().await.unwrap();
        }
        listener.stop().await.unwrap();
        assert_eq!(listener.state(), LifecycleState::Stopped);
        assert_eq!(Arc::strong_count(&transport), 1, "stop must join every connection task");
        let next = timeout(Duration::from_secs(1), sse.chunk())
            .await
            .expect("stop left the SSE response open");
        assert!(matches!(next, Ok(None) | Err(_)));
        assert!(pending.await.unwrap().is_err());
        assert_eq!(idle.read(&mut [0]).await.unwrap(), 0);
        upstream_task.await.unwrap();
        assert_eq!(
            TcpStream::connect(address).await.unwrap_err().kind(),
            ErrorKind::ConnectionRefused
        );
        assert_eq!(listener.start(address.port()).unwrap(), address);
        listener.stop().await.unwrap();
    })
    .await
    .expect("shutdown fixture did not complete");
}

async fn slow_upstream(mut socket: TcpStream, sse: bool, received: mpsc::Sender<String>) {
    let mut request = Vec::new();
    while !request.ends_with(b"\r\n\r\n") {
        request.push(socket.read_u8().await.unwrap());
        assert!(request.len() <= 4096);
    }
    received
        .send(String::from_utf8(request).unwrap().lines().next().unwrap().to_owned())
        .await
        .unwrap();
    if sse {
        socket
            .write_all(b"HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nContent-Length: 1024\r\n\r\ndata: ready\n\n")
            .await
            .unwrap();
    }
    assert_eq!(socket.read(&mut [0]).await.unwrap(), 0, "stop must close the upstream");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn cc_route_006_shutdown_times_out_while_a_connection_is_still_running() {
    let listener = RoutingListener::default();
    let (entered, wait_until_entered) = oneshot::channel();
    let (release, blocked) = std::sync::mpsc::channel::<()>();
    let fixture = std::sync::Mutex::new(Some((entered, blocked)));
    let address = listener
        .start_at(LOOPBACK, move |connection| {
            let (entered, blocked) = fixture.lock().unwrap().take().unwrap();
            Some(async move {
                entered.send(()).unwrap();
                // Model a connection task that temporarily cannot yield to cancellation.
                let _ = blocked.recv();
                drop(connection);
            })
        })
        .unwrap();

    let _client = TcpStream::connect(address).await.unwrap();
    timeout(DEADLINE, wait_until_entered).await.unwrap().unwrap();
    let result = timeout(DEADLINE, listener.stop()).await.unwrap();
    assert!(matches!(result, Err(ListenerError::StopTimeout)));
    assert_eq!(listener.state(), LifecycleState::Stopping);
    assert!(matches!(listener.start(0), Err(ListenerError::TransitionInProgress)));
    assert_eq!(TcpStream::connect(address).await.unwrap_err().kind(), ErrorKind::ConnectionRefused);
    drop(release);

    timeout(DEADLINE, async {
        while listener.state() == LifecycleState::Stopping {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    assert_eq!(listener.state(), LifecycleState::Stopped);
    assert_eq!(listener.start(address.port()).unwrap(), address);
    listener.stop().await.unwrap();
}
