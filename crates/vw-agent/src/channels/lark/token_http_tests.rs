// 本文件验证 Lark 令牌 HTTP 流程、媒体获取、富文本发送与反应重试。

use super::super::*;
use super::token_test_support::*;
use crate::app::agent::channels::traits::{Channel, SendMessage};
use std::time::{Duration, Instant};

#[tokio::test]
async fn get_tenant_access_token_times_out_when_server_never_responds() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let ch = test_channel(&format!("http://{}", listener.local_addr().unwrap()));
    let request = ch.get_tenant_access_token();
    tokio::pin!(request);
    // 保持连接打开但不返回响应，验证客户端自身的超时而非连接失败。
    let (_connection, _) = tokio::select! {
        connection = listener.accept() => connection.unwrap(),
        result = &mut request => panic!("request ended before connection: {result:?}"),
    };
    let error = tokio::time::timeout(Duration::from_secs(35), request)
        .await
        .expect("token request must not hold the chat indefinitely")
        .unwrap_err();
    assert!(error.downcast_ref::<reqwest::Error>().unwrap().is_timeout());
}

#[tokio::test]
async fn lark_channel_send_posts_markdown_as_rich_text() {
    let mut server = TestServer::spawn(vec![MockResponse::json(200, r#"{"code":0}"#)]).await;
    let ch = test_channel(&server.base_url);
    cache_token(&ch, "cached-token", Instant::now() + Duration::from_secs(60)).await;
    let markdown = "## 状态\n\n第一行\n第二行\n\n- **完成**";

    ch.send(&SendMessage::new(markdown, "oc_chat")).await.unwrap();

    let request = server.next_request().await;
    assert_eq!(request_path(&request), "/im/v1/messages?receive_id_type=chat_id");
    assert!(request.starts_with("POST "));
    assert_eq!(header_value(&request, "authorization").as_deref(), Some("Bearer cached-token"));
    let body = request_json_body(&request);
    assert_eq!(body["receive_id"], "oc_chat");
    assert_eq!(body["msg_type"], "post");
    let content: serde_json::Value =
        serde_json::from_str(body["content"].as_str().unwrap()).unwrap();
    assert_eq!(
        content["zh_cn"]["content"][0][0],
        serde_json::json!({"tag": "md", "text": "## 状态\n\n第一行  \n第二行\n\n- **完成**"})
    );
}

#[tokio::test]
async fn lark_channel_send_uses_interactive_card_for_complex_markdown() {
    for markdown in
        ["```rust\nlet ready = true;\n```", "| 项目 | 状态 |\n| --- | --- |\n| 飞书 | ✅ |"]
    {
        let mut server = TestServer::spawn(vec![MockResponse::json(200, r#"{"code":0}"#)]).await;
        let ch = test_channel(&server.base_url);
        cache_token(&ch, "cached-token", Instant::now() + Duration::from_secs(60)).await;

        ch.send(&SendMessage::new(markdown, "oc_chat")).await.unwrap();

        let request = server.next_request().await;
        assert!(request.starts_with("POST "));
        assert_eq!(request_path(&request), "/im/v1/messages?receive_id_type=chat_id");
        assert_eq!(header_value(&request, "authorization").as_deref(), Some("Bearer cached-token"));
        let body = request_json_body(&request);
        assert_eq!(body["receive_id"], "oc_chat");
        assert_eq!(body["msg_type"], "interactive");
        let content: serde_json::Value =
            serde_json::from_str(body["content"].as_str().unwrap()).unwrap();
        assert_eq!(
            content,
            serde_json::json!({
                "schema": "2.0",
                "body": {"elements": [{"tag": "markdown", "content": markdown}]},
            })
        );
    }
}

#[tokio::test]
async fn get_tenant_access_token_fetches_and_caches_success_response() {
    let mut server = TestServer::spawn(vec![MockResponse::json(
        200,
        r#"{"code":0,"tenant_access_token":"fresh-token","expire":180}"#,
    )])
    .await;
    let ch = test_channel(&server.base_url);

    let token = ch.get_tenant_access_token().await.unwrap();

    assert_eq!(token, "fresh-token");
    let request = server.next_request().await;
    assert_eq!(request_path(&request), "/auth/v3/tenant_access_token/internal");
    let body = request_json_body(&request);
    assert_eq!(body["app_id"], "app-id");
    assert_eq!(body["app_secret"], "app-secret");
    assert_eq!(ch.tenant_token.read().await.as_ref().unwrap().value, "fresh-token");
}

#[tokio::test]
async fn get_tenant_access_token_returns_fresh_cache_without_http() {
    let ch = test_channel("http://127.0.0.1:1");
    cache_token(&ch, "cached-token", Instant::now() + Duration::from_secs(60)).await;

    let token = ch.get_tenant_access_token().await.unwrap();

    assert_eq!(token, "cached-token");
}

#[tokio::test]
async fn get_tenant_access_token_refreshes_expired_cache() {
    let mut server = TestServer::spawn(vec![MockResponse::json(
        200,
        r#"{"code":0,"tenant_access_token":"new-token","expire":180}"#,
    )])
    .await;
    let ch = test_channel(&server.base_url);
    cache_token(&ch, "old-token", Instant::now() - Duration::from_secs(1)).await;

    let token = ch.get_tenant_access_token().await.unwrap();

    assert_eq!(token, "new-token");
    assert_eq!(request_path(&server.next_request().await), "/auth/v3/tenant_access_token/internal");
}

#[tokio::test]
async fn get_tenant_access_token_reports_status_business_and_shape_errors() {
    let mut server = TestServer::spawn(vec![MockResponse::json(
        500,
        r#"{"code":999,"tenant_access_token":"sk-testSECRET1234567890"}"#,
    )])
    .await;
    let ch = test_channel(&server.base_url);
    let err = ch.get_tenant_access_token().await.unwrap_err().to_string();
    assert!(err.contains("tenant_access_token request failed"));
    assert!(!err.contains("sk-testSECRET1234567890"));
    assert_eq!(request_path(&server.next_request().await), "/auth/v3/tenant_access_token/internal");

    let mut server =
        TestServer::spawn(vec![MockResponse::json(200, r#"{"code":123,"msg":"bad app"}"#)]).await;
    let ch = test_channel(&server.base_url);
    let err = ch.get_tenant_access_token().await.unwrap_err().to_string();
    assert!(err.contains("bad app"));
    assert_eq!(request_path(&server.next_request().await), "/auth/v3/tenant_access_token/internal");

    let server = TestServer::spawn(vec![MockResponse::json(200, r#"{"code":0}"#)]).await;
    let ch = test_channel(&server.base_url);
    let err = ch.get_tenant_access_token().await.unwrap_err().to_string();
    assert!(err.contains("missing tenant_access_token"));
}

#[tokio::test]
async fn fetch_image_marker_encodes_image_payload() {
    let mut server = TestServer::spawn(vec![
        MockResponse::json(200, r#"{"code":0,"tenant_access_token":"image-token","expire":180}"#),
        MockResponse::bytes(200, "image/jpeg; charset=binary", b"abc"),
    ])
    .await;
    let ch = test_channel(&server.base_url);

    let marker = ch.fetch_image_marker("img_v2").await.unwrap();

    assert_eq!(marker, "[IMAGE:data:image/jpeg;base64,YWJj]");
    assert_eq!(request_path(&server.next_request().await), "/auth/v3/tenant_access_token/internal");
    let image_request = server.next_request().await;
    assert_eq!(request_path(&image_request), "/im/v1/images/img_v2");
    assert_eq!(
        header_value(&image_request, "authorization").as_deref(),
        Some("Bearer image-token")
    );
}

#[tokio::test]
async fn fetch_image_marker_uses_png_fallback_for_non_image_content_type() {
    let server = TestServer::spawn(vec![
        MockResponse::json(200, r#"{"code":0,"tenant_access_token":"image-token","expire":180}"#),
        MockResponse::text(200, "abc"),
    ])
    .await;
    let ch = test_channel(&server.base_url);

    let marker = ch.fetch_image_marker("img_v2").await.unwrap();

    assert_eq!(marker, "[IMAGE:data:image/png;base64,YWJj]");
}

#[tokio::test]
// 媒体请求仅在令牌失效后刷新一次，避免无限重试。
async fn fetch_image_marker_retries_once_after_invalid_token_response() {
    let mut server = TestServer::spawn(vec![
        MockResponse::json(200, r#"{"code":0,"tenant_access_token":"old-token","expire":180}"#),
        MockResponse::json(
            401,
            r#"{"code":99991663,"msg":"Invalid access token for authorization."}"#,
        ),
        MockResponse::json(200, r#"{"code":0,"tenant_access_token":"new-token","expire":180}"#),
        MockResponse::bytes(200, "image/png", b"ok"),
    ])
    .await;
    let ch = test_channel(&server.base_url);

    let marker = ch.fetch_image_marker("img_v2").await.unwrap();

    assert_eq!(marker, "[IMAGE:data:image/png;base64,b2s=]");
    assert_eq!(request_path(&server.next_request().await), "/auth/v3/tenant_access_token/internal");
    let first_image = server.next_request().await;
    assert_eq!(header_value(&first_image, "authorization").as_deref(), Some("Bearer old-token"));
    assert_eq!(request_path(&server.next_request().await), "/auth/v3/tenant_access_token/internal");
    let second_image = server.next_request().await;
    assert_eq!(header_value(&second_image, "authorization").as_deref(), Some("Bearer new-token"));
}

#[tokio::test]
async fn fetch_image_marker_reports_input_payload_and_final_errors() {
    let ch = test_channel("http://127.0.0.1:1");
    assert!(ch.fetch_image_marker("  ").await.unwrap_err().to_string().contains("empty image_key"));

    let server = TestServer::spawn(vec![
        MockResponse::json(200, r#"{"code":0,"tenant_access_token":"image-token","expire":180}"#),
        MockResponse::bytes(200, "image/png", b""),
    ])
    .await;
    let ch = test_channel(&server.base_url);
    let err = ch.fetch_image_marker("img_empty").await.unwrap_err().to_string();
    assert!(err.contains("image payload is empty"));

    let server = TestServer::spawn(vec![
        MockResponse::json(200, r#"{"code":0,"tenant_access_token":"image-token","expire":180}"#),
        MockResponse::json(500, r#"{"msg":"sk-testSECRET1234567890"}"#),
    ])
    .await;
    let ch = test_channel(&server.base_url);
    let err = ch.fetch_image_marker("img_fail").await.unwrap_err().to_string();
    assert!(err.contains("Lark image download failed"));
    assert!(!err.contains("sk-testSECRET1234567890"));
}

#[tokio::test]
async fn try_add_ack_reaction_posts_reaction_body() {
    let mut server = TestServer::spawn(vec![
        MockResponse::json(
            200,
            r#"{"code":0,"tenant_access_token":"reaction-token","expire":180}"#,
        ),
        MockResponse::json(200, r#"{"code":0}"#),
    ])
    .await;
    let ch = test_channel(&server.base_url);

    ch.try_add_ack_reaction("om_1", "THUMBSUP").await;

    assert_eq!(request_path(&server.next_request().await), "/auth/v3/tenant_access_token/internal");
    let request = server.next_request().await;
    assert_eq!(request_path(&request), "/im/v1/messages/om_1/reactions");
    assert_eq!(header_value(&request, "authorization").as_deref(), Some("Bearer reaction-token"));
    assert_eq!(
        request_json_body(&request),
        serde_json::json!({"reaction_type":{"emoji_type":"THUMBSUP"}})
    );
}

#[tokio::test]
async fn try_add_ack_reaction_refreshes_token_after_unauthorized_response() {
    let mut server = TestServer::spawn(vec![
        MockResponse::json(200, r#"{"code":0,"tenant_access_token":"old-token","expire":180}"#),
        MockResponse::json(401, r#"{"msg":"expired"}"#),
        MockResponse::json(200, r#"{"code":0,"tenant_access_token":"new-token","expire":180}"#),
        MockResponse::json(200, r#"{"code":0}"#),
    ])
    .await;
    let ch = test_channel(&server.base_url);

    ch.try_add_ack_reaction("om_1", "OK").await;

    assert_eq!(request_path(&server.next_request().await), "/auth/v3/tenant_access_token/internal");
    let first_reaction = server.next_request().await;
    assert_eq!(header_value(&first_reaction, "authorization").as_deref(), Some("Bearer old-token"));
    assert_eq!(request_path(&server.next_request().await), "/auth/v3/tenant_access_token/internal");
    let second_reaction = server.next_request().await;
    assert_eq!(
        header_value(&second_reaction, "authorization").as_deref(),
        Some("Bearer new-token")
    );
}

#[tokio::test]
async fn try_add_ack_reaction_keeps_failure_paths_best_effort() {
    test_channel("http://127.0.0.1:1").try_add_ack_reaction("", "OK").await;

    let ch = test_channel("http://127.0.0.1:1");
    ch.try_add_ack_reaction("om_1", "OK").await;

    let ch = test_channel("http://127.0.0.1:1");
    cache_token(&ch, "cached-token", Instant::now() + Duration::from_secs(60)).await;
    ch.try_add_ack_reaction("om_1", "OK").await;

    let server = TestServer::spawn(vec![MockResponse::json(500, r#"{"msg":"bad"}"#)]).await;
    let ch = test_channel(&server.base_url);
    cache_token(&ch, "cached-token", Instant::now() + Duration::from_secs(60)).await;
    ch.try_add_ack_reaction("om_1", "OK").await;

    let server = TestServer::spawn(vec![MockResponse::text(200, "not-json")]).await;
    let ch = test_channel(&server.base_url);
    cache_token(&ch, "cached-token", Instant::now() + Duration::from_secs(60)).await;
    ch.try_add_ack_reaction("om_1", "OK").await;

    let server =
        TestServer::spawn(vec![MockResponse::json(200, r#"{"code":123,"msg":"bad"}"#)]).await;
    let ch = test_channel(&server.base_url);
    cache_token(&ch, "cached-token", Instant::now() + Duration::from_secs(60)).await;
    ch.try_add_ack_reaction("om_1", "OK").await;

    let server = TestServer::spawn(vec![
        MockResponse::json(401, r#"{"msg":"expired"}"#),
        MockResponse::json(500, r#"{"msg":"token refresh failed"}"#),
    ])
    .await;
    let ch = test_channel(&server.base_url);
    cache_token(&ch, "cached-token", Instant::now() + Duration::from_secs(60)).await;
    ch.try_add_ack_reaction("om_1", "OK").await;
}

fn delivery_markdown() -> String {
    ["A", "B", "C"]
        .iter()
        .map(|label| format!("```rust\n{label}{}\n```\n\n", "x".repeat(1800)))
        .collect()
}

fn delivery_text(request: &str) -> String {
    let body = request_json_body(request);
    let content: serde_json::Value =
        serde_json::from_str(body["content"].as_str().unwrap()).unwrap();
    if body["msg_type"] == "interactive" {
        content["body"]["elements"][0]["content"].as_str().unwrap().to_string()
    } else {
        content["zh_cn"]["content"][0][0]["text"].as_str().unwrap().to_string()
    }
}

#[tokio::test]
async fn markdown_delivery_lark_chunks_preserve_rendering_and_order() {
    let mut server =
        TestServer::spawn((0..3).map(|_| MockResponse::json(200, r#"{"code":0}"#)).collect()).await;
    let ch = test_channel(&server.base_url);
    cache_token(&ch, "cached-token", Instant::now() + Duration::from_secs(60)).await;
    ch.send(&SendMessage::new(delivery_markdown(), "oc_chat")).await.unwrap();
    for label in ["A", "B", "C"] {
        let request = server.next_request().await;
        let text = delivery_text(&request);
        assert!(text.len() <= 3000, "each rendered chunk must respect its byte budget");
        assert!(text.starts_with(&format!("```rust\n{label}")));
        assert!(text.trim_end().ends_with("```"));
        assert_eq!(request_json_body(&request)["msg_type"], "interactive");
    }
}

#[tokio::test]
async fn markdown_delivery_lark_refresh_retries_only_current_chunk() {
    let mut server = TestServer::spawn(vec![
        MockResponse::json(200, r#"{"code":0}"#),
        MockResponse::json(200, r#"{"code":99991663}"#),
        MockResponse::json(200, r#"{"code":0,"tenant_access_token":"fresh-token","expire":180}"#),
        MockResponse::json(200, r#"{"code":0}"#),
        MockResponse::json(200, r#"{"code":0}"#),
    ])
    .await;
    let ch = test_channel(&server.base_url);
    cache_token(&ch, "cached-token", Instant::now() + Duration::from_secs(60)).await;
    ch.send(&SendMessage::new(delivery_markdown(), "oc_chat")).await.unwrap();
    let a = server.next_request().await;
    let b = server.next_request().await;
    assert_eq!(request_path(&server.next_request().await), "/auth/v3/tenant_access_token/internal");
    let retry_b = server.next_request().await;
    let c = server.next_request().await;
    assert!(delivery_text(&a).starts_with("```rust\nA"));
    assert!(delivery_text(&b).starts_with("```rust\nB"));
    assert_eq!(request_json_body(&b), request_json_body(&retry_b));
    assert!(delivery_text(&c).starts_with("```rust\nC"));
    assert_eq!(header_value(&retry_b, "authorization").as_deref(), Some("Bearer fresh-token"));
    assert_eq!(header_value(&c, "authorization").as_deref(), Some("Bearer fresh-token"));
}

#[tokio::test]
async fn markdown_delivery_lark_unknown_result_does_not_replay() {
    let mut server = TestServer::spawn(vec![
        MockResponse::json(200, r#"{"code":0}"#),
        MockResponse::text(200, "not-json"),
    ])
    .await;
    let ch = test_channel(&server.base_url);
    cache_token(&ch, "cached-token", Instant::now() + Duration::from_secs(60)).await;
    let error = ch.send(&SendMessage::new(delivery_markdown(), "oc_chat")).await.unwrap_err();
    let progress = error
        .downcast_ref::<crate::app::agent::channels::outbound_delivery::ChannelDeliveryError>()
        .expect("structured progress");
    assert_eq!(progress.confirmed_parts, 1);
    assert_eq!(progress.total_parts, 3);
    assert_eq!(progress.failed_part_index, 1);
    assert_eq!(
        progress.kind,
        crate::app::agent::channels::outbound_delivery::DeliveryFailureKind::Unknown
    );
    assert!(delivery_text(&server.next_request().await).starts_with("```rust\nA"));
    assert!(delivery_text(&server.next_request().await).starts_with("```rust\nB"));
}

#[tokio::test]
async fn markdown_delivery_lark_second_part_failure_stops_remaining() {
    use crate::app::agent::channels::outbound_delivery::{
        ChannelDeliveryError, DeliveryFailureKind,
    };
    for (status, body, kind) in [
        (429, r#"{"code":99991400}"#, DeliveryFailureKind::Rejected),
        (500, r#"{"code":99991663}"#, DeliveryFailureKind::Unknown),
        (200, r#"{"code":12345}"#, DeliveryFailureKind::Rejected),
        (200, r#"{}"#, DeliveryFailureKind::Unknown),
        (200, r#"{"code":"0"}"#, DeliveryFailureKind::Unknown),
    ] {
        let mut server = TestServer::spawn(vec![
            MockResponse::json(200, r#"{"code":0}"#),
            MockResponse::json(status, body),
            MockResponse::json(200, r#"{"code":0}"#),
        ])
        .await;
        let ch = test_channel(&server.base_url);
        cache_token(&ch, "cached-token", Instant::now() + Duration::from_secs(60)).await;
        let error = ch.send(&SendMessage::new(delivery_markdown(), "oc_chat")).await.unwrap_err();
        let progress = error.downcast_ref::<ChannelDeliveryError>().unwrap();
        assert_eq!(
            (progress.confirmed_parts, progress.total_parts, progress.failed_part_index),
            (1, 3, 1)
        );
        assert_eq!(progress.kind, kind);
        assert!(delivery_text(&server.next_request().await).starts_with("```rust\nA"));
        assert!(delivery_text(&server.next_request().await).starts_with("```rust\nB"));
        assert!(
            tokio::time::timeout(Duration::from_millis(50), server.next_request()).await.is_err()
        );
    }
}

#[tokio::test]
async fn markdown_delivery_lark_preflight_checks_every_final_payload() {
    use crate::app::agent::channels::outbound_delivery::{
        ChannelDeliveryError, DeliveryFailureKind,
    };
    for message in [
        SendMessage::new(format!("intro\n\n```rust\n{}\n```", "x".repeat(4000)), "oc_chat"),
        SendMessage::new(delivery_markdown(), "o".repeat(17000)),
    ] {
        let mut server = TestServer::spawn(vec![MockResponse::json(200, r#"{"code":0}"#)]).await;
        let ch = test_channel(&server.base_url);
        cache_token(&ch, "cached-token", Instant::now() + Duration::from_secs(60)).await;
        let error = ch.send(&message).await.unwrap_err();
        let progress = error.downcast_ref::<ChannelDeliveryError>().unwrap();
        assert_eq!(progress.confirmed_parts, 0);
        assert_eq!(progress.failed_part_index, 0);
        assert_eq!(progress.kind, DeliveryFailureKind::NotSent);
        assert!(
            tokio::time::timeout(Duration::from_millis(50), server.next_request()).await.is_err()
        );
    }
}

#[tokio::test]
async fn markdown_delivery_lark_transport_failure_and_timeout_stop_current_part() {
    use crate::app::agent::channels::outbound_delivery::{
        ChannelDeliveryError, DeliveryFailureKind,
    };
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    for mode in ["disconnect", "truncated", "timeout"] {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let ch = test_channel(&format!("http://{}", listener.local_addr().unwrap()));
        cache_token(&ch, "cached-token", Instant::now() + Duration::from_secs(60)).await;
        let (captured_tx, mut captured_rx) = tokio::sync::mpsc::channel(2);
        let (release_tx, release_rx) = tokio::sync::oneshot::channel();
        let server = tokio::spawn(async move {
            for index in 0..2 {
                let (mut socket, _) =
                    tokio::time::timeout(Duration::from_secs(2), listener.accept())
                        .await
                        .unwrap()
                        .unwrap();
                let request = tokio::time::timeout(Duration::from_secs(2), async {
                    let mut bytes = Vec::new();
                    loop {
                        let mut buffer = [0u8; 4096];
                        let count = socket.read(&mut buffer).await.unwrap();
                        assert_ne!(count, 0);
                        bytes.extend_from_slice(&buffer[..count]);
                        if let Some(end) = bytes.windows(4).position(|b| b == b"\r\n\r\n") {
                            let headers = String::from_utf8_lossy(&bytes[..end]);
                            let length: usize =
                                header_value(&headers, "content-length").unwrap().parse().unwrap();
                            if bytes.len() >= end + 4 + length {
                                break;
                            }
                        }
                    }
                    String::from_utf8(bytes).unwrap()
                })
                .await
                .unwrap();
                captured_tx.send(request).await.unwrap();
                if index == 0 {
                    socket.write_all(b"HTTP/1.1 200 OK\r\ncontent-length: 10\r\nconnection: close\r\n\r\n{\"code\":0}").await.unwrap();
                } else {
                    if mode == "truncated" {
                        socket.write_all(b"HTTP/1.1 200 OK\r\ncontent-length: 100\r\nconnection: close\r\n\r\n{\"code\":0}").await.unwrap();
                    }
                    if mode == "timeout" {
                        let _ = release_rx.await;
                    }
                    drop(socket);
                    break;
                }
            }
            assert!(
                tokio::time::timeout(Duration::from_millis(50), listener.accept()).await.is_err()
            );
        });
        let send = tokio::spawn(async move {
            ch.send(&SendMessage::new(delivery_markdown(), "oc_chat")).await
        });
        for label in ["A", "B"] {
            let request = tokio::time::timeout(Duration::from_secs(2), captured_rx.recv())
                .await
                .unwrap()
                .unwrap();
            assert!(delivery_text(&request).starts_with(&format!("```rust\n{label}")));
        }
        if mode == "timeout" {
            // Advance the real request timer only after the server has received B.
            tokio::time::pause();
            tokio::time::advance(Duration::from_secs(31)).await;
            tokio::time::resume();
        }
        let error =
            tokio::time::timeout(Duration::from_secs(2), send).await.unwrap().unwrap().unwrap_err();
        let progress = error.downcast_ref::<ChannelDeliveryError>().unwrap();
        assert_eq!(
            (progress.confirmed_parts, progress.total_parts, progress.failed_part_index),
            (1, 3, 1)
        );
        assert_eq!(progress.kind, DeliveryFailureKind::Unknown);
        let _ = release_tx.send(());
        tokio::time::timeout(Duration::from_secs(2), server).await.unwrap().unwrap();
    }
}

#[tokio::test]
async fn markdown_delivery_lark_refresh_failure_preserves_confirmed_prefix() {
    use crate::app::agent::channels::outbound_delivery::{
        ChannelDeliveryError, DeliveryFailureKind,
    };
    for refresh_fails in [false, true] {
        let mut responses = vec![
            MockResponse::json(200, r#"{"code":0}"#),
            MockResponse::json(200, r#"{"code":99991663}"#),
        ];
        if refresh_fails {
            responses.push(MockResponse::json(400, r#"{"code":12345,"msg":"fixture-secret"}"#));
        } else {
            responses.push(MockResponse::json(
                200,
                r#"{"code":0,"tenant_access_token":"fresh-token","expire":180}"#,
            ));
            responses.push(MockResponse::json(200, r#"{"code":99991663,"msg":"fixture-secret"}"#));
        }
        responses.push(MockResponse::json(200, r#"{"code":0}"#));
        let mut server = TestServer::spawn(responses).await;
        let ch = test_channel(&server.base_url);
        cache_token(&ch, "cached-token", Instant::now() + Duration::from_secs(60)).await;
        let error = ch.send(&SendMessage::new(delivery_markdown(), "oc_chat")).await.unwrap_err();
        let progress = error.downcast_ref::<ChannelDeliveryError>().unwrap();
        assert_eq!(
            (progress.confirmed_parts, progress.total_parts, progress.failed_part_index),
            (1, 3, 1)
        );
        assert_eq!(progress.kind, DeliveryFailureKind::Rejected);
        assert!(!format!("{error:?}").contains("fixture-secret"));
        assert!(delivery_text(&server.next_request().await).starts_with("```rust\nA"));
        let b = server.next_request().await;
        assert!(delivery_text(&b).starts_with("```rust\nB"));
        assert_eq!(
            request_path(&server.next_request().await),
            "/auth/v3/tenant_access_token/internal"
        );
        if !refresh_fails {
            assert_eq!(request_json_body(&b), request_json_body(&server.next_request().await));
        }
        assert!(
            tokio::time::timeout(Duration::from_millis(50), server.next_request()).await.is_err()
        );
    }
}
