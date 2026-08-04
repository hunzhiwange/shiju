use super::super::*;
use super::token_test_support::*;
use crate::app::agent::channels::traits::{Channel, SendMessage};
use std::time::{Duration, Instant};

#[tokio::test]
async fn lark_channel_send_posts_markdown_as_rich_text() {
    let mut server = TestServer::spawn(vec![MockResponse::json(200, r#"{"code":0}"#)]).await;
    let ch = test_channel(&server.base_url);
    cache_token(&ch, "cached-token", Instant::now() + Duration::from_secs(60)).await;
    let markdown = "## 状态\n\n- **完成**\n\n```rust\nlet ready = true;\n```\n\n| 项目 | 状态 |\n| --- | --- |\n| 飞书 | ✅ |";

    ch.send(&SendMessage::new(markdown, "oc_chat")).await.unwrap();

    let request = server.next_request().await;
    assert_eq!(request_path(&request), "/im/v1/messages?receive_id_type=chat_id");
    let body = request_json_body(&request);
    assert_eq!(body["receive_id"], "oc_chat");
    assert_eq!(body["msg_type"], "post");
    let content: serde_json::Value = serde_json::from_str(body["content"].as_str().unwrap()).unwrap();
    assert_eq!(
        content["zh_cn"]["content"][0][0],
        serde_json::json!({"tag": "md", "text": markdown})
    );
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
