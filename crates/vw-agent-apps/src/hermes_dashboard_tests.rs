use super::*;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

#[test]
fn hermes_probe_distinguishes_offline_port_and_page_boundaries() {
    for value in [None, Some(""), Some("bad"), Some("65536"), Some("-1")] {
        assert_eq!(port_from_env(value), 9119);
    }
    assert_eq!(port_from_env(Some(" 12345 ")), 12345);
    assert_eq!(port_from_env(Some("0")), 0);
    assert_eq!(page_url(9119, None).unwrap(), "http://127.0.0.1:9119/");
    assert_eq!(page_url(9119, Some("/settings/models")).unwrap(),
        "http://127.0.0.1:9119/settings/models");
    for page in ["https://evil.test", "//evil.test", "\\evil.test", "/%2f%2fevil",
        "javascript:alert(1)", "/../outside", "/x?token=secret", "/x#fragment", "/\n"] {
        assert_eq!(page_url(9119, Some(page)), Err(DashboardError::InvalidPage));
    }
}

#[tokio::test]
async fn hermes_probe_distinguishes_offline_any_http_response() {
    for status in ["200 OK", "401 Unauthorized", "500 Internal Server Error", "302 Found"] {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let server = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut request = Vec::new();
            let mut buffer = [0; 1024];
            while !request.windows(4).any(|part| part == b"\r\n\r\n") {
                let n = stream.read(&mut buffer).await.unwrap();
                assert!(n > 0);
                request.extend_from_slice(&buffer[..n]);
            }
            let request = String::from_utf8(request).unwrap();
            assert!(request.starts_with("GET /api/status HTTP/1.1\r\n"));
            assert!(!request.to_ascii_lowercase().contains("authorization:"));
            stream.write_all(format!("HTTP/1.1 {status}\r\nLocation: http://example.invalid/\r\nContent-Length: 0\r\nConnection: close\r\n\r\n").as_bytes()).await.unwrap();
        });
        assert_eq!(probe_port(port, Some("settings")).await.unwrap(),
            DashboardStatus::Online { url: format!("http://127.0.0.1:{port}/settings") });
        server.await.unwrap();
    }
}

#[tokio::test]
async fn hermes_probe_distinguishes_offline_connection_and_timeout() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    // Accept but never send HTTP headers, exercising the total request timeout.
    let server = tokio::spawn(async move {
        let (_stream, _) = listener.accept().await.unwrap();
        std::future::pending::<()>().await;
    });
    assert_eq!(probe_port(port, None).await.unwrap(), DashboardStatus::Offline);
    server.abort();
    let _ = server.await;
    assert_eq!(probe_port(port, None).await.unwrap(), DashboardStatus::Offline);
    assert_eq!(probe_dashboard("other", "local", None).await,
        Err(DashboardError::UnauthorizedHost));
    assert_eq!(probe_dashboard("", "", None).await,
        Err(DashboardError::UnauthorizedHost));
}
