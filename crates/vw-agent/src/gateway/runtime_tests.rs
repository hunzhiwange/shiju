// 覆盖网关运行时的监听地址、关闭信号与启动失败处理。

// 端口测试优先使用临时监听器制造占用场景，不依赖固定外部服务。
use super::*;

use std::time::Duration;

#[test]
fn runtime_entrypoints_are_available() {
    let _ = serve;
    let _ = start;
}

#[tokio::test]
async fn bind_prefer_uses_requested_available_port() {
    let reserved =
        tokio::net::TcpListener::bind(("127.0.0.1", 0)).await.expect("reserve free port");
    let port = reserved.local_addr().expect("read reserved address").port();
    drop(reserved);

    let listener = bind_prefer("127.0.0.1", port).await.expect("bind requested port");

    assert_eq!(listener.local_addr().expect("read bound address").port(), port);
}

#[tokio::test]
async fn bind_prefer_rejects_occupied_requested_port() {
    let occupied = tokio::net::TcpListener::bind(("127.0.0.1", 0)).await.expect("occupy free port");
    let port = occupied.local_addr().expect("read occupied address").port();

    let error = bind_prefer("127.0.0.1", port).await.expect_err("occupied port should fail");

    assert_eq!(error.status, axum::http::StatusCode::INTERNAL_SERVER_ERROR);
    assert!(!error.to_string().is_empty());
}

#[tokio::test]
async fn bind_prefer_rejects_invalid_host() {
    let result = tokio::time::timeout(Duration::from_secs(5), bind_prefer("256.256.256.256", 0))
        .await
        .expect("invalid host lookup should finish quickly");

    assert!(result.is_err());
}

#[tokio::test]
async fn bind_prefer_zero_port_prefers_4099_then_falls_back_when_occupied() {
    let probe = match tokio::net::TcpListener::bind(("127.0.0.1", 4099)).await {
        Ok(listener) => listener,
        Err(_) => return,
    };
    drop(probe);

    let preferred = bind_prefer("127.0.0.1", 0).await.expect("bind preferred port");
    assert_eq!(preferred.local_addr().expect("read preferred address").port(), 4099);

    let fallback = bind_prefer("127.0.0.1", 0).await.expect("bind fallback port");
    let fallback_port = fallback.local_addr().expect("read fallback address").port();
    assert_ne!(fallback_port, 0);
    assert_ne!(fallback_port, 4099);
}

#[tokio::test]
async fn start_binds_service_and_returns_actual_address() {
    let opts = ServeOptions {
        hostname: "127.0.0.1".to_string(),
        port: 0,
        cors: vec!["https://example.test".to_string()],
    };

    let (addr, handle) = start(opts).await.expect("start gateway");

    assert!(addr.ip().is_loopback());
    assert_ne!(addr.port(), 0);

    handle.abort();
}

#[tokio::test]
async fn start_until_uses_custom_shutdown_future() {
    let opts = ServeOptions { hostname: "127.0.0.1".to_string(), port: 0, cors: Vec::new() };

    let (addr, handle) = start_until(opts, async {}).await.expect("start gateway");
    let result = tokio::time::timeout(Duration::from_secs(5), handle)
        .await
        .expect("service task should stop after shutdown")
        .expect("service task should not panic");

    assert!(addr.ip().is_loopback());
    assert_ne!(addr.port(), 0);
    assert!(result.is_ok());
}

#[tokio::test]
async fn start_until_keeps_service_running_until_shutdown_is_requested() {
    let opts = ServeOptions { hostname: "127.0.0.1".to_string(), port: 0, cors: Vec::new() };
    let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel();

    let (_, handle) = start_until(opts, async move {
        let _ = shutdown_rx.await;
    })
    .await
    .expect("start gateway");

    assert!(!handle.is_finished());
    shutdown_tx.send(()).expect("request graceful shutdown");
    let result = tokio::time::timeout(Duration::from_secs(5), handle)
        .await
        .expect("service task should stop after shutdown")
        .expect("service task should not panic");
    assert!(result.is_ok());
}

#[tokio::test]
async fn start_returns_bind_error_for_occupied_requested_port() {
    let occupied = tokio::net::TcpListener::bind(("127.0.0.1", 0)).await.expect("occupy free port");
    let port = occupied.local_addr().expect("read occupied address").port();
    let opts = ServeOptions { hostname: "127.0.0.1".to_string(), port, cors: Vec::new() };

    let error = start(opts).await.expect_err("start should fail before spawning service task");

    assert_eq!(error.status, axum::http::StatusCode::INTERNAL_SERVER_ERROR);
    assert!(!error.to_string().is_empty());
}

#[tokio::test]
async fn start_until_returns_bind_error_without_polling_shutdown() {
    let occupied = tokio::net::TcpListener::bind(("127.0.0.1", 0)).await.expect("occupy free port");
    let port = occupied.local_addr().expect("read occupied address").port();
    let opts = ServeOptions { hostname: "127.0.0.1".to_string(), port, cors: Vec::new() };

    let error = start_until(opts, async {
        panic!("shutdown future must not be polled when binding fails");
    })
    .await
    .expect_err("binding should fail before spawning the service task");

    assert_eq!(error.status, axum::http::StatusCode::INTERNAL_SERVER_ERROR);
}

#[tokio::test]
async fn serve_returns_actual_address_after_graceful_shutdown() {
    let opts = ServeOptions { hostname: "127.0.0.1".to_string(), port: 0, cors: Vec::new() };

    let addr = serve_until(opts, async {}).await.expect("serve should stop cleanly");

    assert!(addr.ip().is_loopback());
    assert_ne!(addr.port(), 0);
}

#[tokio::test]
async fn serve_until_returns_address_when_shutdown_future_panics() {
    let opts = ServeOptions { hostname: "127.0.0.1".to_string(), port: 0, cors: Vec::new() };

    let addr = serve_until(opts, async {
        panic!("shutdown future failed");
    })
    .await
    .expect("shutdown panic should not hide the bound address");

    assert!(addr.ip().is_loopback());
    assert_ne!(addr.port(), 0);
}

#[tokio::test]
async fn serve_returns_bind_error_for_occupied_port() {
    let occupied = tokio::net::TcpListener::bind(("127.0.0.1", 0)).await.expect("occupy free port");
    let port = occupied.local_addr().expect("read occupied address").port();
    let opts = ServeOptions { hostname: "127.0.0.1".to_string(), port, cors: Vec::new() };

    let error = serve(opts).await.expect_err("serve should fail before waiting on task");

    assert_eq!(error.status, axum::http::StatusCode::INTERNAL_SERVER_ERROR);
    assert!(!error.to_string().is_empty());
}
