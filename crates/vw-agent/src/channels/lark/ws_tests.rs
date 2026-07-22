use super::*;
use serde_json::json;
use std::time::Duration;
use tokio::sync::mpsc;

#[path = "ws_test_support.rs"]
mod ws_test_support;
use ws_test_support::*;

#[test]
fn should_refresh_last_recv_refreshes_only_activity_frames() {
    assert!(should_refresh_last_recv(&WsMsg::Binary(vec![1, 2, 3].into())));
    assert!(should_refresh_last_recv(&WsMsg::Ping(vec![1].into())));
    assert!(should_refresh_last_recv(&WsMsg::Pong(vec![2].into())));

    assert!(!should_refresh_last_recv(&WsMsg::Text("event".into())));
    assert!(!should_refresh_last_recv(&WsMsg::Close(None)));
}

#[test]
fn ws_client_config_ping_interval_defaults_are_interpretable() {
    let default_config = WsClientConfig::default();
    assert_eq!(default_config.ping_interval, None);

    let parsed: WsClientConfig =
        serde_json::from_str(r#"{"PingInterval":15}"#).expect("client config");
    assert_eq!(parsed.ping_interval, Some(15));
}

#[test]
fn pb_frame_header_lookup_handles_duplicate_and_missing_keys() {
    let frame = PbFrame {
        seq_id: 1,
        log_id: 2,
        service: 3,
        method: 1,
        headers: vec![
            PbHeader { key: "type".to_string(), value: "event".to_string() },
            PbHeader { key: "type".to_string(), value: "later".to_string() },
        ],
        payload: None,
    };

    assert_eq!(frame.header_value("type"), "event");
    assert_eq!(frame.header_value("missing"), "");
}

#[tokio::test]
async fn get_ws_endpoint_from_base_posts_credentials_and_reads_endpoint() {
    let mut server = TestHttpServer::spawn(
        200,
        json!({
            "code": 0,
            "data": {
                "URL": "ws://127.0.0.1:9/ws?service_id=7",
                "ClientConfig": {
                    "PingInterval": 25
                }
            }
        }),
    )
    .await;
    let channel = test_channel(false);

    let (url, config) = channel.get_ws_endpoint_from_base(&server.base_url).await.unwrap();
    let request = server.next_request().await;
    let body: serde_json::Value = serde_json::from_str(request_body(&request)).unwrap();

    assert_eq!(url, "ws://127.0.0.1:9/ws?service_id=7");
    assert_eq!(config.ping_interval, Some(25));
    assert!(request.starts_with("POST /callback/ws/endpoint "));
    assert_eq!(body["AppID"], "cli_ws_test");
    assert_eq!(body["AppSecret"], "ws_secret");
}

#[tokio::test]
async fn get_ws_endpoint_from_base_rejects_error_code_and_empty_data() {
    let channel = test_channel(false);
    let mut code_server = TestHttpServer::spawn(200, json!({"code": 190, "msg": "denied"})).await;

    let code_error = channel.get_ws_endpoint_from_base(&code_server.base_url).await.unwrap_err();
    let _ = code_server.next_request().await;

    let mut empty_server = TestHttpServer::spawn(200, json!({"code": 0})).await;
    let empty_error = channel.get_ws_endpoint_from_base(&empty_server.base_url).await.unwrap_err();
    let _ = empty_server.next_request().await;

    assert!(code_error.to_string().contains("code=190"));
    assert!(empty_error.to_string().contains("empty data"));
}

#[tokio::test]
async fn get_ws_endpoint_from_base_reports_unreachable_endpoint() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    drop(listener);
    let channel = test_channel(false);

    let error = channel.get_ws_endpoint_from_base(&format!("http://{addr}")).await.unwrap_err();

    assert!(!error.to_string().is_empty());
}

#[tokio::test]
async fn listen_ws_dispatches_text_and_skips_non_matching_frames() {
    let (ws_url, ws_handle) = spawn_ws_server(|mut ws| async move {
        let initial = next_ws_message(&mut ws).await;
        assert_ping_message(&initial, 42);

        ws.send(WsMsg::Ping(vec![7].into())).await.unwrap();
        let pong = next_ws_message(&mut ws).await;
        assert!(matches!(pong, WsMsg::Pong(_)));

        ws.send(binary_frame(
            2,
            0,
            &[("type", "pong")],
            Some(json!({"PingInterval": 1}).to_string().into_bytes()),
        ))
        .await
        .unwrap();
        ws.send(binary_frame(
            3,
            0,
            &[("type", "pong")],
            Some(json!({"PingInterval": 15}).to_string().into_bytes()),
        ))
        .await
        .unwrap();
        ws.send(WsMsg::Binary(vec![0x0a].into())).await.unwrap();
        ws.send(binary_frame(4, 1, &[("type", "ignored")], None)).await.unwrap();
        let ack = next_ws_message(&mut ws).await;
        assert_ack_message(&ack);

        ws.send(event_frame(
            5,
            "wrong_event",
            json!({"header": {"event_type": "other", "event_id": "evt"}, "event": {}})
                .to_string()
                .into_bytes(),
        ))
        .await
        .unwrap();
        ws.send(event_frame(
            6,
            "bad_payload",
            json!({"header": {"event_type": "im.message.receive_v1", "event_id": "evt"}, "event": {}})
                .to_string()
                .into_bytes(),
        ))
        .await
        .unwrap();
        ws.send(event_frame(
            7,
            "app_sender",
            message_event(
                ("app", "ou_allowed"),
                ("app_sender", "p2p", "text", &json!({"text": "app"}).to_string()),
                json!([]),
            ),
        ))
        .await
        .unwrap();
        ws.send(event_frame(
            8,
            "bot_sender",
            message_event(
                ("bot", "ou_allowed"),
                ("bot_sender", "p2p", "text", &json!({"text": "bot"}).to_string()),
                json!([]),
            ),
        ))
        .await
        .unwrap();
        ws.send(event_frame(9, "denied", text_event("ou_denied", "denied", "denied")))
            .await
            .unwrap();
        ws.send(event_frame(
            10,
            "bad_content",
            message_event(
                ("user", "ou_allowed"),
                ("bad_content", "p2p", "text", "not-json"),
                json!([]),
            ),
        ))
        .await
        .unwrap();
        ws.send(event_frame(11, "empty_text", text_event("ou_allowed", "empty_text", "")))
            .await
            .unwrap();
        ws.send(event_frame(
            12,
            "unsupported",
            message_event(
                ("user", "ou_allowed"),
                ("unsupported", "p2p", "audio", "{}"),
                json!([]),
            ),
        ))
        .await
        .unwrap();
        ws.send(event_frame(13, "", text_event("ou_allowed", "", " hello @_user_1 world ")))
            .await
            .unwrap();
        ws.send(event_frame(14, "", text_event("ou_allowed", "", "duplicate")))
            .await
            .unwrap();
        let _ = ws.close(None).await;
    })
    .await;
    let mut endpoint = TestHttpServer::spawn(
        200,
        json!({
            "code": 0,
            "data": {
                "URL": format!("{ws_url}/?service_id=42"),
                "ClientConfig": {
                    "PingInterval": 1
                }
            }
        }),
    )
    .await;
    let channel = test_channel(false);
    let (tx, mut rx) = mpsc::channel(8);

    channel.listen_ws_with_endpoint_base(tx, &endpoint.base_url).await.unwrap();
    let request = endpoint.next_request().await;
    let message = rx.recv().await.unwrap();

    assert!(request.starts_with("POST /callback/ws/endpoint "));
    assert_eq!(message.sender, "oc_chat");
    assert_eq!(message.reply_target, "oc_chat");
    assert_eq!(message.content, "hello world");
    assert_eq!(message.channel, "lark");
    assert!(message.timestamp > 0);
    assert!(rx.try_recv().is_err());
    ws_handle.await.unwrap();
}

#[tokio::test]
async fn listen_ws_skips_text_frame_invalid_event_json_and_invalid_post_content() {
    let (ws_url, ws_handle) = spawn_ws_server(|mut ws| async move {
        let initial = next_ws_message(&mut ws).await;
        assert_ping_message(&initial, 0);

        ws.send(WsMsg::Text("ignored text frame".into())).await.unwrap();

        ws.send(event_frame(2, "bad_json", b"not-json".to_vec())).await.unwrap();
        let bad_json_ack = next_ws_message(&mut ws).await;
        assert_ack_message(&bad_json_ack);

        ws.send(event_frame(
            3,
            "bad_post",
            message_event(
                ("user", "ou_allowed"),
                ("bad_post", "p2p", "post", "not-json"),
                json!([]),
            ),
        ))
        .await
        .unwrap();
        let bad_post_ack = next_ws_message(&mut ws).await;
        assert_ack_message(&bad_post_ack);

        ws.send(event_frame(4, "delivered", text_event("ou_allowed", "delivered", "after skips")))
            .await
            .unwrap();
        let delivered_ack = next_ws_message(&mut ws).await;
        assert_ack_message(&delivered_ack);
        let _ = ws.close(None).await;
    })
    .await;
    let mut endpoint =
        TestHttpServer::spawn(200, json!({"code": 0, "data": {"URL": ws_url}})).await;
    let channel = test_channel(false);
    let (tx, mut rx) = mpsc::channel(1);

    channel.listen_ws_with_endpoint_base(tx, &endpoint.base_url).await.unwrap();
    let _ = endpoint.next_request().await;
    let message = rx.recv().await.unwrap();

    assert_eq!(message.content, "after skips");
    assert!(rx.try_recv().is_err());
    ws_handle.await.unwrap();
}

#[tokio::test]
async fn listen_ws_reassembles_fragmented_event_payload() {
    let payload = text_event("ou_allowed", "", "fragmented hello");
    let split = payload.len() / 2;
    let first = payload[..split].to_vec();
    let second = payload[split..].to_vec();
    let (ws_url, ws_handle) = spawn_ws_server(move |mut ws| async move {
        let initial = next_ws_message(&mut ws).await;
        assert_ping_message(&initial, 0);

        ws.send(fragment_frame(2, "frag", "3", "0", first.clone())).await.unwrap();
        let first_ack = next_ws_message(&mut ws).await;
        assert_ack_message(&first_ack);

        ws.send(fragment_frame(3, "frag", "2", "0", first)).await.unwrap();
        let reset_ack = next_ws_message(&mut ws).await;
        assert_ack_message(&reset_ack);

        ws.send(fragment_frame(4, "frag", "2", "1", second)).await.unwrap();
        let final_ack = next_ws_message(&mut ws).await;
        assert_ack_message(&final_ack);
        let _ = ws.close(None).await;
    })
    .await;
    let mut endpoint = TestHttpServer::spawn(
        200,
        json!({
            "code": 0,
            "data": {
                "URL": ws_url
            }
        }),
    )
    .await;
    let channel = test_channel(false);
    let (tx, mut rx) = mpsc::channel(2);

    channel.listen_ws_with_endpoint_base(tx, &endpoint.base_url).await.unwrap();
    let _ = endpoint.next_request().await;
    let message = rx.recv().await.unwrap();

    assert_eq!(message.content, "fragmented hello");
    assert!(rx.try_recv().is_err());
    ws_handle.await.unwrap();
}

#[tokio::test]
async fn listen_ws_treats_zero_fragment_count_as_single_frame() {
    let payload = text_event("ou_allowed", "zero-sum", "zero fragment count");
    let (ws_url, ws_handle) = spawn_ws_server(move |mut ws| async move {
        let initial = next_ws_message(&mut ws).await;
        assert_ping_message(&initial, 0);

        ws.send(fragment_frame(2, "zero-sum", "0", "4", payload)).await.unwrap();
        let ack = next_ws_message(&mut ws).await;
        assert_ack_message(&ack);
        let _ = ws.close(None).await;
    })
    .await;
    let mut endpoint =
        TestHttpServer::spawn(200, json!({"code": 0, "data": {"URL": ws_url}})).await;
    let channel = test_channel(false);
    let (tx, mut rx) = mpsc::channel(1);

    channel.listen_ws_with_endpoint_base(tx, &endpoint.base_url).await.unwrap();
    let _ = endpoint.next_request().await;
    let message = rx.recv().await.unwrap();

    assert_eq!(message.content, "zero fragment count");
    ws_handle.await.unwrap();
}

#[tokio::test]
async fn listen_ws_dispatches_group_post_when_bot_is_mentioned_in_body() {
    let post_content = json!({
        "zh_cn": {
            "title": "",
            "content": [[
                {"tag": "at", "user_id": "ou_bot", "user_name": "Bot"},
                {"tag": "text", "text": " post"}
            ]]
        }
    })
    .to_string();
    let (ws_url, ws_handle) = spawn_ws_server(move |mut ws| async move {
        let initial = next_ws_message(&mut ws).await;
        assert_ping_message(&initial, 0);
        ws.send(event_frame(
            2,
            "",
            message_event(("user", "ou_allowed"), ("", "group", "post", &post_content), json!([])),
        ))
        .await
        .unwrap();
        let ack = next_ws_message(&mut ws).await;
        assert_ack_message(&ack);
        let _ = ws.close(None).await;
    })
    .await;
    let mut endpoint =
        TestHttpServer::spawn(200, json!({"code": 0, "data": {"URL": ws_url}})).await;
    let channel = test_channel(true);
    let (tx, mut rx) = mpsc::channel(2);

    channel.listen_ws_with_endpoint_base(tx, &endpoint.base_url).await.unwrap();
    let _ = endpoint.next_request().await;
    let message = rx.recv().await.unwrap();

    assert_eq!(message.content, "@Bot post");
    ws_handle.await.unwrap();
}

#[tokio::test]
async fn listen_ws_dispatches_image_fallback_when_image_key_is_missing() {
    let (ws_url, ws_handle) = spawn_ws_server(|mut ws| async move {
        let initial = next_ws_message(&mut ws).await;
        assert_ping_message(&initial, 0);
        ws.send(event_frame(
            2,
            "",
            message_event(("user", "ou_allowed"), ("", "p2p", "image", "{}"), json!([])),
        ))
        .await
        .unwrap();
        let ack = next_ws_message(&mut ws).await;
        assert_ack_message(&ack);
        let _ = ws.close(None).await;
    })
    .await;
    let mut endpoint =
        TestHttpServer::spawn(200, json!({"code": 0, "data": {"URL": ws_url}})).await;
    let channel = test_channel(false);
    let (tx, mut rx) = mpsc::channel(2);

    channel.listen_ws_with_endpoint_base(tx, &endpoint.base_url).await.unwrap();
    let _ = endpoint.next_request().await;
    let message = rx.recv().await.unwrap();

    assert_eq!(message.content, LARK_IMAGE_DOWNLOAD_FALLBACK_TEXT);
    ws_handle.await.unwrap();
}

#[tokio::test]
async fn listen_ws_uses_fallback_text_when_image_download_fails() {
    let (ws_url, ws_handle) = spawn_ws_server(|mut ws| async move {
        let initial = next_ws_message(&mut ws).await;
        assert_ping_message(&initial, 0);
        ws.send(event_frame(
            2,
            "",
            message_event(
                ("user", "ou_allowed"),
                ("", "p2p", "image", &json!({"image_key": "img_v2_present"}).to_string()),
                json!([]),
            ),
        ))
        .await
        .unwrap();
        let ack = next_ws_message(&mut ws).await;
        assert_ack_message(&ack);
        let _ = ws.close(None).await;
    })
    .await;
    let mut endpoint =
        TestHttpServer::spawn(200, json!({"code": 0, "data": {"URL": ws_url}})).await;
    // 指向已关闭端口，使图片下载所需的 token 请求立即失败，触发回退文本分支。
    let channel = test_channel(false).with_api_base("http://127.0.0.1:1".to_string());
    let (tx, mut rx) = mpsc::channel(2);

    channel.listen_ws_with_endpoint_base(tx, &endpoint.base_url).await.unwrap();
    let _ = endpoint.next_request().await;
    let message = rx.recv().await.unwrap();

    assert_eq!(message.content, LARK_IMAGE_DOWNLOAD_FALLBACK_TEXT);
    ws_handle.await.unwrap();
}

#[tokio::test]
async fn listen_ws_skips_group_post_when_bot_is_not_mentioned() {
    let unmentioned_post = json!({
        "zh_cn": {
            "title": "",
            "content": [[{"tag": "text", "text": "no mention here"}]]
        }
    })
    .to_string();
    let (ws_url, ws_handle) = spawn_ws_server(move |mut ws| async move {
        let initial = next_ws_message(&mut ws).await;
        assert_ping_message(&initial, 0);

        // 群聊 post 未 @ 机器人，应被跳过。
        ws.send(event_frame(
            2,
            "",
            message_event(
                ("user", "ou_allowed"),
                ("skipped", "group", "post", &unmentioned_post),
                json!([]),
            ),
        ))
        .await
        .unwrap();
        let skipped_ack = next_ws_message(&mut ws).await;
        assert_ack_message(&skipped_ack);

        // 后续 p2p 文本应正常分发，证明上一条被跳过而非阻塞。
        ws.send(event_frame(3, "", text_event("ou_allowed", "delivered", "private hello")))
            .await
            .unwrap();
        let delivered_ack = next_ws_message(&mut ws).await;
        assert_ack_message(&delivered_ack);
        let _ = ws.close(None).await;
    })
    .await;
    let mut endpoint =
        TestHttpServer::spawn(200, json!({"code": 0, "data": {"URL": ws_url}})).await;
    let channel = test_channel(true);
    let (tx, mut rx) = mpsc::channel(2);

    channel.listen_ws_with_endpoint_base(tx, &endpoint.base_url).await.unwrap();
    let _ = endpoint.next_request().await;
    let message = rx.recv().await.unwrap();

    assert_eq!(message.content, "private hello");
    assert!(rx.try_recv().is_err());
    ws_handle.await.unwrap();
}

#[tokio::test]
async fn listen_ws_breaks_when_downstream_channel_is_closed() {
    let (ws_url, ws_handle) = spawn_ws_server(|mut ws| async move {
        let initial = next_ws_message(&mut ws).await;
        assert_ping_message(&initial, 0);
        ws.send(event_frame(2, "", text_event("ou_allowed", "closed", "dropped"))).await.unwrap();
        let ack = next_ws_message(&mut ws).await;
        assert_ack_message(&ack);
        // 不主动关闭，等待客户端因发送失败而断开。
        let _ = next_ws_message(&mut ws).await;
    })
    .await;
    let mut endpoint =
        TestHttpServer::spawn(200, json!({"code": 0, "data": {"URL": ws_url}})).await;
    // 指向已关闭端口，避免后台 ack 反应任务触达真实网络。
    let channel = test_channel(false).with_api_base("http://127.0.0.1:1".to_string());
    let (tx, rx) = mpsc::channel(1);
    drop(rx); // 关闭接收端，使 tx.send 失败并触发 break。

    channel.listen_ws_with_endpoint_base(tx, &endpoint.base_url).await.unwrap();
    let _ = endpoint.next_request().await;
    ws_handle.abort();
}

#[tokio::test(start_paused = true)]
async fn listen_ws_sends_interval_ping_after_server_configured_floor() {
    let (ws_url, ws_handle) = spawn_ws_server(|mut ws| async move {
        let initial = ws.next().await.unwrap().unwrap();
        assert_ping_message(&initial, 0);

        tokio::time::advance(Duration::from_secs(10)).await;
        let interval = ws.next().await.unwrap().unwrap();
        assert_ping_message(&interval, 0);
        let _ = ws.close(None).await;
    })
    .await;
    let mut endpoint = TestHttpServer::spawn(
        200,
        json!({
            "code": 0,
            "data": {
                "URL": ws_url,
                "ClientConfig": {
                    "PingInterval": 1
                }
            }
        }),
    )
    .await;
    let channel = test_channel(false);
    let (tx, _rx) = mpsc::channel(1);

    channel.listen_ws_with_endpoint_base(tx, &endpoint.base_url).await.unwrap();
    let _ = endpoint.next_request().await;
    ws_handle.await.unwrap();
}
