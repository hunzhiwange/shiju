use super::SharedTransport;
use axum::{
    Router,
    body::{Body, Bytes, to_bytes},
    extract::Request,
    http::{Method, StatusCode, header},
    response::Response,
};
use futures_util::StreamExt;
use std::{
    io::Write,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};
use tokio_util::sync::CancellationToken;

const DEADLINE: Duration = Duration::from_secs(3);
const FIXTURE: &str = "opaque request bytes";

async fn listen(router: Router) -> (reqwest::Url, tokio::task::JoinHandle<()>) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap()).parse().unwrap();
    let server = tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
    (url, server)
}

#[tokio::test]
async fn cc_route_002_shared_transport_expected_behavior() {
    let hits = Arc::new(AtomicUsize::new(0));
    let router = Router::new().fallback({
        let hits = hits.clone();
        move |request: Request| {
            let hits = hits.clone();
            async move {
                hits.fetch_add(1, Ordering::Relaxed);
                match request.uri().path() {
                    "/vendor" => {
                        assert_eq!(request.method(), Method::PUT);
                        assert_eq!(request.uri().query(), Some("alt=fixture"));
                        assert_eq!(request.headers()[header::AUTHORIZATION], "Bearer fixture-key");
                        assert!(!request.headers().contains_key("x-agent-apps-token"));
                        assert_eq!(to_bytes(request.into_body(), 1024).await.unwrap(), FIXTURE);
                        let mut gzip =
                            flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::fast());
                        gzip.write_all(b"opaque response bytes").unwrap();
                        Response::builder()
                            .status(StatusCode::CREATED)
                            .header(header::CONTENT_ENCODING, "gzip")
                            .body(Body::from(gzip.finish().unwrap()))
                            .unwrap()
                    }
                    "/redirect" => Response::builder()
                        .status(StatusCode::TEMPORARY_REDIRECT)
                        .header(header::LOCATION, "/other")
                        .body(Body::empty())
                        .unwrap(),
                    _ => Response::builder()
                        .status(StatusCode::IM_A_TEAPOT)
                        .body(Body::from("opaque error"))
                        .unwrap(),
                }
            }
        }
    });
    let (base, server) = listen(router).await;
    let transport = SharedTransport::new().unwrap();
    let target = base.join("/vendor?alt=fixture").unwrap();
    let mut request = reqwest::Request::new(Method::PUT, target.clone());
    request.headers_mut().insert(header::AUTHORIZATION, "Bearer fixture-key".parse().unwrap());
    *request.body_mut() = Some(FIXTURE.into());
    let response = tokio::time::timeout(
        DEADLINE,
        transport.forward(&target, request, CancellationToken::new()),
    )
    .await
    .unwrap()
    .unwrap();
    assert_eq!(response.status(), StatusCode::CREATED);
    assert!(!response.headers().contains_key(header::CONTENT_ENCODING));
    assert_eq!(to_bytes(response.into_body(), 1024).await.unwrap(), "opaque response bytes");

    for (path, expected) in
        [("/redirect", StatusCode::TEMPORARY_REDIRECT), ("/other", StatusCode::IM_A_TEAPOT)]
    {
        let url = base.join(path).unwrap();
        let response = tokio::time::timeout(
            DEADLINE,
            transport.forward(
                &url,
                reqwest::Request::new(Method::GET, url.clone()),
                CancellationToken::new(),
            ),
        )
        .await
        .unwrap()
        .unwrap();
        assert_eq!(response.status(), expected);
    }
    assert_eq!(hits.load(Ordering::Relaxed), 3);

    let other = base.join("/other").unwrap();
    let rejected = tokio::time::timeout(
        DEADLINE,
        transport.forward(
            &target,
            reqwest::Request::new(Method::GET, other),
            CancellationToken::new(),
        ),
    )
    .await
    .unwrap();
    assert_eq!(rejected.unwrap_err(), StatusCode::FORBIDDEN);
    assert_eq!(hits.load(Ordering::Relaxed), 3);

    let mut spoofed_host = reqwest::Request::new(Method::GET, target.clone());
    spoofed_host.headers_mut().insert(header::HOST, "other.example".parse().unwrap());
    assert_eq!(
        transport.forward(&target, spoofed_host, CancellationToken::new()).await.unwrap_err(),
        StatusCode::FORBIDDEN,
    );
    let cancelled = CancellationToken::new();
    cancelled.cancel();
    assert_eq!(
        transport
            .forward(&target, reqwest::Request::new(Method::GET, target.clone()), cancelled)
            .await
            .unwrap_err(),
        StatusCode::REQUEST_TIMEOUT,
    );
    assert_eq!(hits.load(Ordering::Relaxed), 3);

    server.abort();
    assert!(server.await.unwrap_err().is_cancelled());
}

#[tokio::test]
async fn cc_route_002_shared_transport_cancels_waiting_for_headers_and_sse() {
    let arrived = Arc::new(tokio::sync::Notify::new());
    let router = Router::new().fallback({
        let arrived = arrived.clone();
        move |request: Request| {
            let arrived = arrived.clone();
            async move {
                if request.uri().path() == "/slow" {
                    arrived.notify_one();
                    return std::future::pending::<Response>().await;
                }
                let stream = futures_util::stream::iter([Ok::<_, std::io::Error>(
                    Bytes::from_static(b"data: fixture\n\n"),
                )]);
                Response::builder()
                    .header(header::CONTENT_TYPE, "text/event-stream")
                    .body(Body::from_stream(stream.chain(futures_util::stream::pending())))
                    .unwrap()
            }
        }
    });
    let (base, server) = listen(router).await;
    let transport = SharedTransport::new().unwrap();
    let slow = base.join("/slow").unwrap();
    let cancel = CancellationToken::new();
    let (response, ()) = tokio::time::timeout(DEADLINE, async {
        tokio::join!(
            transport.forward(
                &slow,
                reqwest::Request::new(Method::GET, slow.clone()),
                cancel.clone(),
            ),
            async {
                arrived.notified().await;
                cancel.cancel();
            },
        )
    })
    .await
    .expect("cancellation must end a request waiting for headers");
    assert_eq!(response.unwrap_err(), StatusCode::REQUEST_TIMEOUT);

    let sse = base.join("/sse").unwrap();
    let cancel = CancellationToken::new();
    let response = tokio::time::timeout(
        DEADLINE,
        transport.forward(&sse, reqwest::Request::new(Method::GET, sse.clone()), cancel.clone()),
    )
    .await
    .unwrap()
    .unwrap();
    assert_eq!(response.headers()[header::CONTENT_TYPE], "text/event-stream");
    let mut body = response.into_body().into_data_stream();
    let chunk = tokio::time::timeout(DEADLINE, body.next()).await.unwrap().unwrap().unwrap();
    assert_eq!(chunk, "data: fixture\n\n");
    cancel.cancel();
    assert!(tokio::time::timeout(DEADLINE, body.next()).await.unwrap().is_none());

    server.abort();
    assert!(server.await.unwrap_err().is_cancelled());
}

#[tokio::test]
async fn cc_route_002_shared_transport_surfaces_truncated_sse() {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url =
        format!("http://{}/sse", listener.local_addr().unwrap()).parse::<reqwest::Url>().unwrap();
    let server = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut request = Vec::new();
        while !request.ends_with(b"\r\n\r\n") {
            assert!(request.len() < 2048);
            request.push(socket.read_u8().await.unwrap());
        }
        socket
            .write_all(
                b"HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nContent-Length: 99\r\n\r\ndata: fixture\n\n",
            )
            .await
            .unwrap();
        socket.shutdown().await.unwrap();
    });

    let transport = SharedTransport::new().unwrap();
    let response = tokio::time::timeout(
        DEADLINE,
        transport.forward(
            &url,
            reqwest::Request::new(Method::GET, url.clone()),
            CancellationToken::new(),
        ),
    )
    .await
    .unwrap()
    .unwrap();
    let mut body = response.into_body().into_data_stream();
    let chunk = tokio::time::timeout(DEADLINE, body.next()).await.unwrap().unwrap().unwrap();
    assert_eq!(chunk, "data: fixture\n\n");
    assert!(tokio::time::timeout(DEADLINE, body.next()).await.unwrap().unwrap().is_err());
    tokio::time::timeout(DEADLINE, server).await.unwrap().unwrap();
}
