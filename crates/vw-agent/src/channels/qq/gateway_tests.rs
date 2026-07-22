use std::future::Future;
use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use serde_json::Value;
use tokio::net::TcpListener;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tokio_tungstenite::WebSocketStream;
use tokio_tungstenite::tungstenite::Message;

use super::*;

async fn spawn_gateway_server<F, Fut>(handler: F) -> (String, JoinHandle<()>)
where
    F: FnOnce(WebSocketStream<tokio::net::TcpStream>) -> Fut + Send + 'static,
    Fut: Future<Output = ()> + Send + 'static,
{
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind local websocket server");
    let addr = listener.local_addr().expect("read local websocket addr");
    let handle = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.expect("accept websocket client");
        let websocket = tokio_tungstenite::accept_async(stream).await.expect("accept websocket");
        handler(websocket).await;
    });

    (format!("ws://{addr}"), handle)
}

async fn maybe_read_heartbeat(
    websocket: &mut WebSocketStream<tokio::net::TcpStream>,
) -> Option<Value> {
    let frame = match tokio::time::timeout(Duration::from_millis(100), websocket.next()).await {
        Ok(Some(Ok(frame))) => frame,
        _ => return None,
    };

    Some(
        serde_json::from_str(frame.to_text().expect("heartbeat text"))
            .expect("parse heartbeat payload"),
    )
}

#[test]
fn heartbeat_payload_uses_null_for_unknown_sequence() {
    assert_eq!(heartbeat_payload(-1), serde_json::json!({"op": 1, "d": null}));
    assert_eq!(heartbeat_payload(42), serde_json::json!({"op": 1, "d": 42}));
}

#[test]
fn heartbeat_payload_preserves_zero_sequence() {
    assert_eq!(heartbeat_payload(0), serde_json::json!({"op": 1, "d": 0}));
}

#[test]
fn heartbeat_interval_from_hello_reads_configured_value() {
    let interval = heartbeat_interval_from_hello(&Message::Text(
        r#"{"d":{"heartbeat_interval":1234}}"#.into(),
    ))
    .expect("parse hello interval");

    assert_eq!(interval, 1234);
}

#[test]
fn heartbeat_interval_from_hello_defaults_when_missing() {
    let interval = heartbeat_interval_from_hello(&Message::Text(r#"{"op":10,"d":{}}"#.into()))
        .expect("parse hello interval");

    assert_eq!(interval, 41250);
}

#[test]
fn heartbeat_interval_from_hello_rejects_invalid_json() {
    let error = heartbeat_interval_from_hello(&Message::Text("not-json".into())).unwrap_err();

    assert!(!error.to_string().is_empty());
}

#[test]
fn identify_payload_formats_expected_qq_identify_body() {
    let payload = identify_payload("bot-token");

    assert_eq!(payload["op"], 2);
    assert_eq!(payload["d"]["token"], "QQBot bot-token");
    assert_eq!(payload["d"]["intents"], (1 << 25) | (1 << 30));
    assert_eq!(payload["d"]["properties"]["os"], "linux");
    assert_eq!(payload["d"]["properties"]["browser"], "vibewindow");
    assert_eq!(payload["d"]["properties"]["device"], "vibewindow");
}

#[test]
fn parse_gateway_text_updates_sequence_and_requests_heartbeat() {
    let mut sequence = -1;

    let event = parse_gateway_text(r#"{"op":1,"s":42}"#, &mut sequence);

    assert_eq!(event, GatewayEvent::SendHeartbeat);
    assert_eq!(sequence, 42);
}

#[test]
fn parse_gateway_text_returns_dispatch_event_for_op_zero_payloads() {
    let mut sequence = -1;

    let event = parse_gateway_text(
        r#"{"op":0,"s":3,"t":"C2C_MESSAGE_CREATE","d":{"id":"msg-1"}}"#,
        &mut sequence,
    );

    assert_eq!(
        event,
        GatewayEvent::Dispatch {
            event_type: "C2C_MESSAGE_CREATE".to_string(),
            payload: serde_json::json!({"id": "msg-1"}),
        }
    );
    assert_eq!(sequence, 3);
}

#[test]
fn parse_gateway_text_breaks_on_reconnect_and_invalid_session() {
    let mut reconnect_sequence = -1;
    let mut invalid_session_sequence = -1;

    assert_eq!(parse_gateway_text(r#"{"op":7}"#, &mut reconnect_sequence), GatewayEvent::Break);
    assert_eq!(
        parse_gateway_text(r#"{"op":9,"s":11}"#, &mut invalid_session_sequence),
        GatewayEvent::Break
    );
    assert_eq!(invalid_session_sequence, 11);
}

#[test]
fn parse_gateway_text_ignores_invalid_or_incomplete_payloads() {
    let mut sequence = -1;

    assert_eq!(parse_gateway_text("not-json", &mut sequence), GatewayEvent::Ignore);
    assert_eq!(parse_gateway_text(r#"{"op":2}"#, &mut sequence), GatewayEvent::Ignore);
    assert_eq!(
        parse_gateway_text(r#"{"op":0,"t":"C2C_MESSAGE_CREATE"}"#, &mut sequence),
        GatewayEvent::Ignore
    );
}

#[tokio::test]
async fn run_gateway_session_returns_error_when_no_hello_frame_arrives() {
    let (url, handle) = spawn_gateway_server(|mut websocket| async move {
        websocket.close(None).await.expect("close websocket");
    })
    .await;
    let (ws_stream, _) = tokio_tungstenite::connect_async(&url).await.expect("connect websocket");
    let (mut write, mut read) = ws_stream.split();
    let (tx, _rx) = mpsc::channel(1);
    let channel =
        QQChannel::new("app".to_string(), "secret".to_string(), vec!["user-openid".to_string()]);

    let error =
        channel.run_gateway_session("bot-token", tx, &mut write, &mut read).await.unwrap_err();

    assert_eq!(error.to_string(), "QQ: no hello frame");
    handle.await.expect("server task finished");
}

#[tokio::test]
async fn listen_gateway_propagates_token_errors_before_gateway_lookup() {
    let (tx, _rx) = mpsc::channel(1);
    let channel = QQChannel::new("app".to_string(), "secret".to_string(), vec![])
        .with_auth_url("http://127.0.0.1:1/token".to_string());

    let error = channel.listen_gateway(tx).await.unwrap_err();

    assert!(!error.to_string().is_empty());
}

#[tokio::test]
async fn listen_gateway_propagates_gateway_url_errors_before_websocket_connect() {
    let (tx, _rx) = mpsc::channel(1);
    let channel = QQChannel::new("app".to_string(), "secret".to_string(), vec![])
        .with_api_base("http://127.0.0.1:1".to_string());
    {
        let mut cache = channel.token_cache.write().await;
        *cache = Some(("cached-token".to_string(), u64::MAX));
    }

    let error = channel.listen_gateway(tx).await.unwrap_err();

    assert!(!error.to_string().is_empty());
}

#[tokio::test]
async fn run_gateway_session_sends_identify_and_forwards_dispatch_message() {
    let (url, handle) = spawn_gateway_server(|mut websocket| async move {
        websocket
            .send(Message::Text(r#"{"op":10,"d":{"heartbeat_interval":5000}}"#.into()))
            .await
            .expect("send hello");

        let identify = websocket.next().await.expect("read identify frame").expect("identify frame");
        let identify_payload: Value = serde_json::from_str(identify.to_text().expect("identify text"))
            .expect("parse identify payload");
        assert_eq!(identify_payload["d"]["token"], "QQBot bot-token");
        if let Some(heartbeat) = maybe_read_heartbeat(&mut websocket).await {
            assert_eq!(heartbeat, serde_json::json!({"op": 1, "d": null}));
        }

        websocket
            .send(Message::Text(
                r#"{"op":0,"s":7,"t":"C2C_MESSAGE_CREATE","d":{"id":"msg-1","content":"hello","author":{"id":"author-1","user_openid":"user-openid"}}}"#
                    .into(),
            ))
            .await
            .expect("send dispatch");
        websocket.close(None).await.expect("close websocket");
    })
    .await;
    let (ws_stream, _) = tokio_tungstenite::connect_async(&url).await.expect("connect websocket");
    let (mut write, mut read) = ws_stream.split();
    let (tx, mut rx) = mpsc::channel(1);
    let channel =
        QQChannel::new("app".to_string(), "secret".to_string(), vec!["user-openid".to_string()]);

    let client = tokio::spawn(async move {
        channel.run_gateway_session("bot-token", tx, &mut write, &mut read).await
    });

    let forwarded = tokio::time::timeout(Duration::from_secs(1), rx.recv())
        .await
        .expect("wait for forwarded message")
        .expect("forwarded channel message");
    let error = client.await.expect("gateway task joined").unwrap_err();

    assert!(!forwarded.id.is_empty());
    assert_eq!(forwarded.thread_ts.as_deref(), Some("msg-1"));
    assert_eq!(forwarded.sender, "user-openid");
    assert_eq!(forwarded.reply_target, "user:user-openid");
    assert_eq!(forwarded.content, "hello");
    assert_eq!(forwarded.channel, "qq");
    assert_eq!(error.to_string(), "QQ WebSocket connection closed");
    handle.await.expect("server task finished");
}

#[tokio::test]
async fn run_gateway_session_replies_to_op1_heartbeat_request_with_latest_sequence() {
    let (url, handle) = spawn_gateway_server(|mut websocket| async move {
        websocket
            .send(Message::Text(r#"{"op":10,"d":{"heartbeat_interval":5000}}"#.into()))
            .await
            .expect("send hello");
        let _identify =
            websocket.next().await.expect("read identify frame").expect("identify frame");
        if let Some(heartbeat) = maybe_read_heartbeat(&mut websocket).await {
            assert_eq!(heartbeat, serde_json::json!({"op": 1, "d": null}));
        }

        websocket
            .send(Message::Text(r#"{"op":1,"s":9}"#.into()))
            .await
            .expect("send heartbeat request");

        let heartbeat =
            websocket.next().await.expect("read heartbeat frame").expect("heartbeat frame");
        let payload: Value = serde_json::from_str(heartbeat.to_text().expect("heartbeat text"))
            .expect("parse heartbeat payload");
        assert_eq!(payload, serde_json::json!({"op": 1, "d": 9}));

        websocket.close(None).await.expect("close websocket");
    })
    .await;
    let (ws_stream, _) = tokio_tungstenite::connect_async(&url).await.expect("connect websocket");
    let (mut write, mut read) = ws_stream.split();
    let (tx, _rx) = mpsc::channel(1);
    let channel =
        QQChannel::new("app".to_string(), "secret".to_string(), vec!["user-openid".to_string()]);

    let error =
        channel.run_gateway_session("bot-token", tx, &mut write, &mut read).await.unwrap_err();

    assert_eq!(error.to_string(), "QQ WebSocket connection closed");
    handle.await.expect("server task finished");
}

#[tokio::test]
async fn run_gateway_session_breaks_when_gateway_requests_reconnect() {
    let (url, handle) = spawn_gateway_server(|mut websocket| async move {
        websocket
            .send(Message::Text(r#"{"op":10,"d":{"heartbeat_interval":5000}}"#.into()))
            .await
            .expect("send hello");
        let _identify =
            websocket.next().await.expect("read identify frame").expect("identify frame");
        let _heartbeat = maybe_read_heartbeat(&mut websocket).await;

        websocket.send(Message::Text(r#"{"op":7}"#.into())).await.expect("send reconnect request");
    })
    .await;
    let (ws_stream, _) = tokio_tungstenite::connect_async(&url).await.expect("connect websocket");
    let (mut write, mut read) = ws_stream.split();
    let (tx, _rx) = mpsc::channel(1);
    let channel =
        QQChannel::new("app".to_string(), "secret".to_string(), vec!["user-openid".to_string()]);

    let error =
        channel.run_gateway_session("bot-token", tx, &mut write, &mut read).await.unwrap_err();

    assert_eq!(error.to_string(), "QQ WebSocket connection closed");
    handle.await.expect("server task finished");
}

#[tokio::test]
async fn run_gateway_session_breaks_when_gateway_reports_invalid_session() {
    let (url, handle) = spawn_gateway_server(|mut websocket| async move {
        websocket
            .send(Message::Text(r#"{"op":10,"d":{"heartbeat_interval":5000}}"#.into()))
            .await
            .expect("send hello");
        let _identify =
            websocket.next().await.expect("read identify frame").expect("identify frame");
        let _heartbeat = maybe_read_heartbeat(&mut websocket).await;

        websocket
            .send(Message::Text(r#"{"op":9}"#.into()))
            .await
            .expect("send invalid session request");
    })
    .await;
    let (ws_stream, _) = tokio_tungstenite::connect_async(&url).await.expect("connect websocket");
    let (mut write, mut read) = ws_stream.split();
    let (tx, _rx) = mpsc::channel(1);
    let channel =
        QQChannel::new("app".to_string(), "secret".to_string(), vec!["user-openid".to_string()]);

    let error =
        channel.run_gateway_session("bot-token", tx, &mut write, &mut read).await.unwrap_err();

    assert_eq!(error.to_string(), "QQ WebSocket connection closed");
    handle.await.expect("server task finished");
}

#[tokio::test]
async fn run_gateway_session_ignores_non_text_and_unhandled_gateway_events() {
    let (url, handle) = spawn_gateway_server(|mut websocket| async move {
        websocket
            .send(Message::Text(r#"{"op":10,"d":{"heartbeat_interval":5000}}"#.into()))
            .await
            .expect("send hello");
        let _identify =
            websocket.next().await.expect("read identify frame").expect("identify frame");
        let _heartbeat = maybe_read_heartbeat(&mut websocket).await;

        websocket.send(Message::Binary(vec![1, 2, 3].into())).await.expect("send binary");
        websocket.send(Message::Text(r#"{"op":42}"#.into())).await.expect("send ignored event");
        websocket.close(None).await.expect("close websocket");
    })
    .await;
    let (ws_stream, _) = tokio_tungstenite::connect_async(&url).await.expect("connect websocket");
    let (mut write, mut read) = ws_stream.split();
    let (tx, _rx) = mpsc::channel(1);
    let channel =
        QQChannel::new("app".to_string(), "secret".to_string(), vec!["user-openid".to_string()]);

    let error =
        channel.run_gateway_session("bot-token", tx, &mut write, &mut read).await.unwrap_err();

    assert_eq!(error.to_string(), "QQ WebSocket connection closed");
    handle.await.expect("server task finished");
}

#[tokio::test]
async fn run_gateway_session_breaks_when_forward_channel_is_closed() {
    let (url, handle) = spawn_gateway_server(|mut websocket| async move {
        websocket
            .send(Message::Text(r#"{"op":10,"d":{"heartbeat_interval":5000}}"#.into()))
            .await
            .expect("send hello");
        let _identify =
            websocket.next().await.expect("read identify frame").expect("identify frame");
        let _heartbeat = maybe_read_heartbeat(&mut websocket).await;

        websocket
            .send(Message::Text(
                r#"{"op":0,"s":7,"t":"C2C_MESSAGE_CREATE","d":{"id":"msg-closed","content":"hello","author":{"id":"author-1","user_openid":"user-openid"}}}"#
                    .into(),
            ))
            .await
            .expect("send dispatch");
    })
    .await;
    let (ws_stream, _) = tokio_tungstenite::connect_async(&url).await.expect("connect websocket");
    let (mut write, mut read) = ws_stream.split();
    let (tx, rx) = mpsc::channel(1);
    drop(rx);
    let channel =
        QQChannel::new("app".to_string(), "secret".to_string(), vec!["user-openid".to_string()]);

    let error =
        channel.run_gateway_session("bot-token", tx, &mut write, &mut read).await.unwrap_err();

    assert_eq!(error.to_string(), "QQ WebSocket connection closed");
    handle.await.expect("server task finished");
}

#[tokio::test]
async fn run_gateway_session_sends_timer_heartbeat_with_unknown_sequence() {
    let (url, handle) = spawn_gateway_server(|mut websocket| async move {
        websocket
            .send(Message::Text(r#"{"op":10,"d":{"heartbeat_interval":1}}"#.into()))
            .await
            .expect("send hello");
        let _identify =
            websocket.next().await.expect("read identify frame").expect("identify frame");

        let heartbeat =
            websocket.next().await.expect("read timer heartbeat").expect("heartbeat frame");
        let payload: Value = serde_json::from_str(heartbeat.to_text().expect("heartbeat text"))
            .expect("parse heartbeat payload");
        assert_eq!(payload, serde_json::json!({"op": 1, "d": null}));

        websocket.close(None).await.expect("close websocket");
    })
    .await;
    let (ws_stream, _) = tokio_tungstenite::connect_async(&url).await.expect("connect websocket");
    let (mut write, mut read) = ws_stream.split();
    let (tx, _rx) = mpsc::channel(1);
    let channel =
        QQChannel::new("app".to_string(), "secret".to_string(), vec!["user-openid".to_string()]);

    let error =
        channel.run_gateway_session("bot-token", tx, &mut write, &mut read).await.unwrap_err();

    assert_eq!(error.to_string(), "QQ WebSocket connection closed");
    handle.await.expect("server task finished");
}
