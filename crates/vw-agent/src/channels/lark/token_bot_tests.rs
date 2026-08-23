use super::super::*;
use super::token_test_support::*;

#[tokio::test]
async fn refresh_bot_open_id_reads_bot_and_data_paths() {
    let server = TestServer::spawn(vec![
        MockResponse::json(200, r#"{"code":0,"tenant_access_token":"bot-token","expire":180}"#),
        MockResponse::json(200, r#"{"code":0,"bot":{"open_id":" ou_bot "}}"#),
    ])
    .await;
    let ch = mention_channel(&server.base_url);

    assert_eq!(ch.refresh_bot_open_id().await.unwrap().as_deref(), Some("ou_bot"));
    assert_eq!(ch.resolved_bot_open_id().as_deref(), Some("ou_bot"));

    let server = TestServer::spawn(vec![
        MockResponse::json(200, r#"{"code":0,"tenant_access_token":"bot-token","expire":180}"#),
        MockResponse::json(200, r#"{"code":0,"data":{"bot":{"open_id":"ou_bot_data"}}}"#),
    ])
    .await;
    let ch = mention_channel(&server.base_url);

    assert_eq!(ch.refresh_bot_open_id().await.unwrap().as_deref(), Some("ou_bot_data"));
}

#[tokio::test]
async fn refresh_bot_open_id_retries_token_and_reports_errors() {
    let server = TestServer::spawn(vec![
        MockResponse::json(200, r#"{"code":0,"tenant_access_token":"old-token","expire":180}"#),
        MockResponse::json(200, r#"{"code":99991663}"#),
        MockResponse::json(200, r#"{"code":0,"tenant_access_token":"new-token","expire":180}"#),
        MockResponse::json(200, r#"{"code":0,"bot":{"open_id":"ou_retry"}}"#),
    ])
    .await;
    let ch = mention_channel(&server.base_url);
    assert_eq!(ch.refresh_bot_open_id().await.unwrap().as_deref(), Some("ou_retry"));

    let server = TestServer::spawn(vec![
        MockResponse::json(200, r#"{"code":0,"tenant_access_token":"old-token","expire":180}"#),
        MockResponse::json(401, r#"{"code":99991663}"#),
        MockResponse::json(200, r#"{"code":0,"tenant_access_token":"new-token","expire":180}"#),
        MockResponse::json(500, r#"{"msg":"still bad"}"#),
    ])
    .await;
    let ch = mention_channel(&server.base_url);
    let err = ch.refresh_bot_open_id().await.unwrap_err().to_string();
    assert!(err.contains("after token refresh"));

    let server = TestServer::spawn(vec![
        MockResponse::json(200, r#"{"code":0,"tenant_access_token":"bot-token","expire":180}"#),
        MockResponse::json(500, r#"{"msg":"bad"}"#),
    ])
    .await;
    let ch = mention_channel(&server.base_url);
    let err = ch.refresh_bot_open_id().await.unwrap_err().to_string();
    assert!(err.contains("bot info request failed"));

    let server = TestServer::spawn(vec![
        MockResponse::json(200, r#"{"code":0,"tenant_access_token":"bot-token","expire":180}"#),
        MockResponse::json(200, r#"{"code":456,"msg":"bad"}"#),
    ])
    .await;
    let ch = mention_channel(&server.base_url);
    let err = ch.refresh_bot_open_id().await.unwrap_err().to_string();
    assert!(err.contains("bot info failed"));

    let server = TestServer::spawn(vec![
        MockResponse::json(200, r#"{"code":0,"tenant_access_token":"bot-token","expire":180}"#),
        MockResponse::json(200, r#"{"code":0,"bot":{"open_id":"  "}}"#),
    ])
    .await;
    let ch = mention_channel(&server.base_url);
    assert_eq!(ch.refresh_bot_open_id().await.unwrap(), None);
}

#[tokio::test]
async fn fetch_bot_open_id_with_token_defaults_invalid_json_to_empty_object() {
    let mut server = TestServer::spawn(vec![MockResponse::text(200, "not-json")]).await;
    let ch = mention_channel(&server.base_url);

    let (status, body) = ch.fetch_bot_open_id_with_token("bot-token").await.unwrap();

    assert_eq!(status, reqwest::StatusCode::OK);
    assert_eq!(body, serde_json::json!({}));
    let request = server.next_request().await;
    assert_eq!(request_path(&request), "/bot/v3/info");
    assert_eq!(header_value(&request, "authorization").as_deref(), Some("Bearer bot-token"));
}

#[tokio::test]
async fn ensure_bot_open_id_is_best_effort() {
    test_channel("http://127.0.0.1:1").ensure_bot_open_id().await;

    let ch = mention_channel("http://127.0.0.1:1");
    ch.set_resolved_bot_open_id(Some("ou_existing".to_string()));
    ch.ensure_bot_open_id().await;
    assert_eq!(ch.resolved_bot_open_id().as_deref(), Some("ou_existing"));

    let server = TestServer::spawn(vec![
        MockResponse::json(200, r#"{"code":0,"tenant_access_token":"bot-token","expire":180}"#),
        MockResponse::json(200, r#"{"code":0,"bot":{"open_id":"ou_bot"}}"#),
    ])
    .await;
    let ch = mention_channel(&server.base_url);
    ch.ensure_bot_open_id().await;
    assert_eq!(ch.resolved_bot_open_id().as_deref(), Some("ou_bot"));

    let server = TestServer::spawn(vec![
        MockResponse::json(200, r#"{"code":0,"tenant_access_token":"bot-token","expire":180}"#),
        MockResponse::json(200, r#"{"code":0}"#),
    ])
    .await;
    let ch = mention_channel(&server.base_url);
    ch.ensure_bot_open_id().await;
    assert_eq!(ch.resolved_bot_open_id(), None);

    let ch = mention_channel("http://127.0.0.1:1");
    ch.ensure_bot_open_id().await;
    assert_eq!(ch.resolved_bot_open_id(), None);
}

#[tokio::test]
async fn send_text_once_posts_json_and_wraps_raw_response() {
    let mut server = TestServer::spawn(vec![
        MockResponse::json(201, r#"{"code":0}"#),
        MockResponse::text(200, "raw-ok"),
    ])
    .await;
    let ch = test_channel(&server.base_url);
    let body = serde_json::json!({"receive_id":"chat","msg_type":"text"});

    let (status, parsed) =
        ch.send_text_once(&format!("{}/send", server.base_url), "send-token", &body).await.unwrap();

    assert_eq!(status, reqwest::StatusCode::CREATED);
    assert_eq!(parsed, serde_json::json!({"code":0}));
    let request = server.next_request().await;
    assert_eq!(request_path(&request), "/send");
    assert_eq!(header_value(&request, "authorization").as_deref(), Some("Bearer send-token"));
    assert_eq!(request_json_body(&request), body);

    let (status, parsed) = ch
        .send_text_once(&format!("{}/send-raw", server.base_url), "send-token", &body)
        .await
        .unwrap();

    assert_eq!(status, reqwest::StatusCode::OK);
    assert_eq!(parsed, serde_json::json!({"raw":"raw-ok"}));
    assert_eq!(request_path(&server.next_request().await), "/send-raw");
}
// 本文件验证 Lark 机器人标识刷新、令牌重试与文本发送封装。
