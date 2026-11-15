use super::*;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

async fn stalled_request_times_out(client: reqwest::Client, stall_body: bool, long_poll: bool) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let mut request = client.get(url);
    if long_poll {
        request = request.timeout(Duration::from_secs(60));
    }
    let (headers_tx, headers_rx) = tokio::sync::oneshot::channel();
    let worker = tokio::spawn(async move {
        let response = request.send().await?;
        let _ = headers_tx.send(());
        response.bytes().await
    });
    let (mut connection, _) =
        tokio::time::timeout(Duration::from_secs(5), listener.accept()).await.unwrap().unwrap();
    let mut byte = [0];
    tokio::time::timeout(Duration::from_secs(5), connection.read_exact(&mut byte))
        .await
        .unwrap()
        .unwrap();
    if stall_body {
        connection.write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 5\r\n\r\n").await.unwrap();
        tokio::time::timeout(Duration::from_secs(5), headers_rx).await.unwrap().unwrap();
    }

    // 连接已建立后推进时钟，分别验证等响应头和等响应体时的总期限。
    tokio::time::pause();
    tokio::time::advance(Duration::from_secs(31)).await;
    tokio::task::yield_now().await;
    if long_poll {
        assert!(!worker.is_finished(), "long polling must outlive the ordinary request budget");
        tokio::time::advance(Duration::from_secs(30)).await;
    }
    let error = tokio::time::timeout(Duration::from_secs(1), worker)
        .await
        .expect("stalled channel request must finish at its deadline")
        .unwrap()
        .unwrap_err();
    assert!(error.is_timeout(), "expected request timeout: {error}");
    tokio::time::resume();
}

#[tokio::test]
async fn channel_http_requests_time_out_before_headers_and_during_body() {
    for client in [client_builder().build().unwrap(), proxied_client("channel.slack")] {
        stalled_request_times_out(client.clone(), false, false).await;
        stalled_request_times_out(client, true, false).await;
    }
}

#[tokio::test]
async fn channel_http_long_poll_override_keeps_a_finite_longer_deadline() {
    stalled_request_times_out(proxied_client("channel.telegram"), true, true).await;
}
