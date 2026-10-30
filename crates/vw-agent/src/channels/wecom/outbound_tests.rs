use super::*;
use crate::app::agent::channels::outbound_delivery::{ChannelDeliveryError, DeliveryFailureKind};
use tokio::net::{TcpListener, TcpStream};
use tokio::task::JoinHandle;
use tokio::time::timeout;
use tokio_tungstenite::{WebSocketStream, accept_async};

type Socket = WebSocketStream<TcpStream>;

async fn fixture_with_callbacks()
-> (WeComChannel, Socket, JoinHandle<anyhow::Result<()>>, mpsc::Receiver<ChannelMessage>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let channel = WeComChannel::new_for_test(
        "fixture-bot".into(),
        "fixture-secret".into(),
        format!("ws://{}", listener.local_addr().unwrap()),
    );
    let cloned = channel.clone();
    let (tx, mut rx) = mpsc::channel(8);
    let listen = tokio::spawn(async move { cloned.listen(tx).await });
    let (stream, _) = timeout(Duration::from_secs(2), listener.accept()).await.unwrap().unwrap();
    let mut socket = accept_async(stream).await.unwrap();
    let subscribe = next_frame(&mut socket).await;
    ack(&mut socket, subscribe["headers"]["req_id"].as_str().unwrap(), json!(0)).await;
    for (recipient, request_id) in [("chat-a", "callback-a"), ("chat-b", "callback-b")] {
        socket
            .send(Message::Text(
                json!({
                    "cmd": "aibot_msg_callback", "headers": {"req_id": request_id},
                    "body": {"msgtype": "text", "chatid": recipient,
                             "from": {"userid": recipient}, "text": {"content": "fixture"}}
                })
                .to_string()
                .into(),
            ))
            .await
            .unwrap();
        timeout(Duration::from_secs(2), rx.recv()).await.unwrap().unwrap();
    }
    (channel, socket, listen, rx)
}

async fn fixture() -> (WeComChannel, Socket, JoinHandle<anyhow::Result<()>>) {
    let (channel, socket, listen, _) = fixture_with_callbacks().await;
    (channel, socket, listen)
}

async fn next_frame(socket: &mut Socket) -> Value {
    let frame = timeout(Duration::from_secs(2), socket.next()).await.unwrap().unwrap().unwrap();
    serde_json::from_str(frame.into_text().unwrap().as_ref()).unwrap()
}

async fn ack(socket: &mut Socket, request_id: &str, code: Value) {
    socket
        .send(Message::Text(
            json!({"headers": {"req_id": request_id}, "errcode": code}).to_string().into(),
        ))
        .await
        .unwrap();
}

fn send(channel: &WeComChannel, recipient: &str) -> JoinHandle<anyhow::Result<()>> {
    let channel = channel.clone();
    let recipient = recipient.to_string();
    tokio::spawn(async move {
        channel
            .send(&SendMessage {
                recipient,
                content: "fixture response".into(),
                subject: None,
                thread_ts: None,
            })
            .await
    })
}

async fn pending(task: &mut JoinHandle<anyhow::Result<()>>) {
    assert!(
        timeout(Duration::from_millis(40), task).await.is_err(),
        "send must wait for matching business acknowledgement"
    );
}

async fn result(task: JoinHandle<anyhow::Result<()>>, kind: DeliveryFailureKind) {
    let error = timeout(Duration::from_secs(2), task).await.unwrap().unwrap().unwrap_err();
    let delivery = error.downcast_ref::<ChannelDeliveryError>().expect("structured error");
    assert_eq!(delivery.kind, kind);
    assert_eq!(
        (delivery.confirmed_parts, delivery.total_parts, delivery.failed_part_index),
        (0, 1, 0)
    );
    assert!(!error.to_string().contains("fixture-secret"));
    assert!(!error.to_string().contains("fixture response"));
}

#[tokio::test]
async fn markdown_delivery_wecom_waits_for_matching_ack() {
    let (channel, mut socket, listen) = fixture().await;
    let mut task = send(&channel, "chat-a");
    let frame = next_frame(&mut socket).await;
    assert_eq!(frame["headers"]["req_id"], "callback-a");
    pending(&mut task).await;
    ack(&mut socket, "wrong-request", json!(0)).await;
    ack(&mut socket, "ping_fixture", json!(0)).await;
    pending(&mut task).await;
    ack(&mut socket, "callback-a", json!(0)).await;
    timeout(Duration::from_secs(2), task).await.unwrap().unwrap().unwrap();
    listen.abort();
    let _ = listen.await;
}

#[tokio::test]
async fn markdown_delivery_wecom_rejection_propagates_to_sender() {
    let (channel, mut socket, listen) = fixture().await;
    let task = send(&channel, "chat-a");
    next_frame(&mut socket).await;
    ack(&mut socket, "callback-a", json!(45009)).await;
    result(task, DeliveryFailureKind::Rejected).await;
    listen.abort();
    let _ = listen.await;
}

#[tokio::test]
async fn markdown_delivery_wecom_timeout_clears_pending_without_replay() {
    let (channel, mut socket, listen) = fixture().await;
    tokio::time::pause();
    let task = send(&channel, "chat-a");
    next_frame(&mut socket).await;
    tokio::time::advance(Duration::from_secs(6)).await;
    result(task, DeliveryFailureKind::Unknown).await;
    ack(&mut socket, "callback-a", json!(0)).await;
    let task = send(&channel, "chat-a");
    result(task, DeliveryFailureKind::NotSent).await;
    assert!(timeout(Duration::from_millis(40), socket.next()).await.is_err());
    listen.abort();
    let _ = listen.await;
}

#[tokio::test]
async fn markdown_delivery_wecom_same_request_serializes_and_other_requests_are_isolated() {
    let (channel, mut socket, listen) = fixture().await;
    let first = send(&channel, "chat-a");
    next_frame(&mut socket).await;
    let mut second = send(&channel, "chat-a");
    pending(&mut second).await;
    let other = send(&channel, "chat-b");
    assert_eq!(next_frame(&mut socket).await["headers"]["req_id"], "callback-b");
    ack(&mut socket, "callback-b", json!(0)).await;
    timeout(Duration::from_secs(2), other).await.unwrap().unwrap().unwrap();
    pending(&mut second).await;
    ack(&mut socket, "callback-a", json!(0)).await;
    timeout(Duration::from_secs(2), first).await.unwrap().unwrap().unwrap();
    assert_eq!(next_frame(&mut socket).await["headers"]["req_id"], "callback-a");
    ack(&mut socket, "callback-a", json!(0)).await;
    timeout(Duration::from_secs(2), second).await.unwrap().unwrap().unwrap();
    listen.abort();
    let _ = listen.await;
}

#[tokio::test]
async fn markdown_delivery_wecom_malformed_ack_blocks_only_its_request() {
    for code in [Value::Null, json!("0"), json!(false)] {
        let (channel, mut socket, listen) = fixture().await;
        let first = send(&channel, "chat-a");
        next_frame(&mut socket).await;
        let mut queued = send(&channel, "chat-a");
        pending(&mut queued).await;
        ack(&mut socket, "callback-a", code).await;
        result(first, DeliveryFailureKind::Unknown).await;
        result(queued, DeliveryFailureKind::NotSent).await;
        ack(&mut socket, "callback-a", json!(0)).await;
        result(send(&channel, "chat-a"), DeliveryFailureKind::NotSent).await;
        let other = send(&channel, "chat-b");
        assert_eq!(next_frame(&mut socket).await["headers"]["req_id"], "callback-b");
        ack(&mut socket, "callback-b", json!(0)).await;
        timeout(Duration::from_secs(2), other).await.unwrap().unwrap().unwrap();
        listen.abort();
        let _ = listen.await;
    }
}

#[tokio::test]
async fn markdown_delivery_wecom_callback_and_heartbeat_are_not_reply_acknowledgements() {
    let (channel, mut socket, listen) = fixture().await;
    let mut task = send(&channel, "chat-a");
    next_frame(&mut socket).await;
    socket
        .send(Message::Text(
            json!({
                "cmd": "aibot_event_callback", "headers": {"req_id": "callback-a"}, "errcode": 0,
            })
            .to_string()
            .into(),
        ))
        .await
        .unwrap();
    pending(&mut task).await;
    ack(&mut socket, "ping_fixture", json!(0)).await;
    pending(&mut task).await;
    socket
        .send(Message::Text(json!({"headers": {"req_id": "callback-a"}}).to_string().into()))
        .await
        .unwrap();
    result(task, DeliveryFailureKind::Unknown).await;
    listen.abort();
    let _ = listen.await;
}

#[tokio::test]
async fn markdown_delivery_wecom_disconnect_and_listener_abort_resolve_all_waiters() {
    for abort in [false, true] {
        let (channel, mut socket, listen) = fixture().await;
        let first = send(&channel, "chat-a");
        next_frame(&mut socket).await;
        let mut queued = send(&channel, "chat-a");
        pending(&mut queued).await;
        if abort {
            listen.abort();
        } else {
            socket.close(None).await.unwrap();
        }
        result(first, DeliveryFailureKind::Unknown).await;
        result(queued, DeliveryFailureKind::NotSent).await;
        let _ = timeout(Duration::from_secs(2), listen).await.unwrap();
        result(send(&channel, "chat-a"), DeliveryFailureKind::NotSent).await;
    }
}

#[tokio::test]
async fn markdown_delivery_wecom_cancelled_queued_item_is_never_sent() {
    let (channel, mut socket, listen) = fixture().await;
    let first = send(&channel, "chat-a");
    next_frame(&mut socket).await;
    let mut cancelled = send(&channel, "chat-a");
    pending(&mut cancelled).await;
    cancelled.abort();
    let _ = cancelled.await;
    ack(&mut socket, "callback-a", json!(0)).await;
    timeout(Duration::from_secs(2), first).await.unwrap().unwrap().unwrap();
    let other = send(&channel, "chat-b");
    assert_eq!(next_frame(&mut socket).await["headers"]["req_id"], "callback-b");
    ack(&mut socket, "callback-b", json!(0)).await;
    timeout(Duration::from_secs(2), other).await.unwrap().unwrap().unwrap();
    assert!(timeout(Duration::from_millis(40), socket.next()).await.is_err());
    listen.abort();
    let _ = listen.await;
}

#[tokio::test]
async fn markdown_delivery_wecom_cancelled_written_item_blocks_late_ack_and_queue() {
    let (channel, mut socket, listen) = fixture().await;
    let first = send(&channel, "chat-a");
    next_frame(&mut socket).await;
    let mut queued = send(&channel, "chat-a");
    pending(&mut queued).await;
    first.abort();
    let _ = first.await;
    result(queued, DeliveryFailureKind::NotSent).await;
    ack(&mut socket, "callback-a", json!(0)).await;
    result(send(&channel, "chat-a"), DeliveryFailureKind::NotSent).await;
    let other = send(&channel, "chat-b");
    assert_eq!(next_frame(&mut socket).await["headers"]["req_id"], "callback-b");
    ack(&mut socket, "callback-b", json!(0)).await;
    timeout(Duration::from_secs(2), other).await.unwrap().unwrap().unwrap();
    listen.abort();
    let _ = listen.await;
}

#[tokio::test]
async fn markdown_delivery_wecom_invalid_json_disconnects_without_replay() {
    let (channel, mut socket, listen) = fixture().await;
    let task = send(&channel, "chat-a");
    next_frame(&mut socket).await;
    socket.send(Message::Text("{incomplete".into())).await.unwrap();
    result(task, DeliveryFailureKind::Unknown).await;
    assert!(timeout(Duration::from_secs(2), listen).await.unwrap().unwrap().is_err());
    result(send(&channel, "chat-a"), DeliveryFailureKind::NotSent).await;
}

#[tokio::test]
async fn markdown_delivery_wecom_reconnect_drops_old_queue_and_ambiguous_requests() {
    let (mut channel, mut old_socket, listen) = fixture().await;
    let first = send(&channel, "chat-a");
    next_frame(&mut old_socket).await;
    let mut queued = send(&channel, "chat-a");
    pending(&mut queued).await;
    listen.abort();
    let _ = listen.await;
    result(first, DeliveryFailureKind::Unknown).await;
    result(queued, DeliveryFailureKind::NotSent).await;
    drop(old_socket);

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    channel.endpoint = format!("ws://{}", listener.local_addr().unwrap());
    let cloned = channel.clone();
    let (tx, mut rx) = mpsc::channel(1);
    let listen = tokio::spawn(async move { cloned.listen(tx).await });
    let (stream, _) = timeout(Duration::from_secs(2), listener.accept()).await.unwrap().unwrap();
    let mut socket = accept_async(stream).await.unwrap();
    let subscribe = next_frame(&mut socket).await;
    ack(&mut socket, subscribe["headers"]["req_id"].as_str().unwrap(), json!(0)).await;
    // The server observes no queued frame replayed on the replacement connection.
    assert!(timeout(Duration::from_millis(40), socket.next()).await.is_err());
    result(send(&channel, "chat-a"), DeliveryFailureKind::NotSent).await;
    socket
        .send(Message::Text(
            json!({
                "cmd": "aibot_msg_callback", "headers": {"req_id": "callback-a"},
                "body": {"msgtype": "text", "chatid": "chat-a", "from": {"userid": "chat-a"},
                         "text": {"content": "fresh callback"}}
            })
            .to_string()
            .into(),
        ))
        .await
        .unwrap();
    timeout(Duration::from_secs(2), rx.recv()).await.unwrap().unwrap();
    let task = send(&channel, "chat-a");
    assert_eq!(next_frame(&mut socket).await["headers"]["req_id"], "callback-a");
    ack(&mut socket, "callback-a", json!(0)).await;
    timeout(Duration::from_secs(2), task).await.unwrap().unwrap().unwrap();
    listen.abort();
    let _ = listen.await;
}

#[tokio::test]
async fn markdown_delivery_wecom_state_is_bounded_and_drop_resolves_unsent_items() {
    let mut replies = OutboundReplies::default();
    let mut receivers = Vec::new();
    for index in 0..64 {
        let (item, receiver) = OutboundItem::new(format!("fixture-{index}"), json!({}));
        replies.enqueue(item);
        receivers.push(receiver);
    }
    let (excess, receiver) = OutboundItem::new("excess".into(), json!({}));
    replies.enqueue(excess);
    assert_eq!(receiver.await.unwrap(), Err(DeliveryFailureKind::NotSent));
    drop(replies);
    for receiver in receivers {
        assert_eq!(receiver.await.unwrap(), Err(DeliveryFailureKind::NotSent));
    }
}

fn chunked_send(channel: &WeComChannel, content: String) -> JoinHandle<anyhow::Result<()>> {
    let channel = channel.clone();
    tokio::spawn(async move {
        channel
            .send(&SendMessage {
                recipient: "chat-a".into(),
                content,
                subject: None,
                thread_ts: None,
            })
            .await
    })
}

fn three_chunks() -> String {
    ["甲".repeat(650), "乙".repeat(650), "丙".repeat(650)].join("\n\n")
}

#[tokio::test]
async fn markdown_delivery_wecom_chunks_use_distinct_stream_ids() {
    let (channel, mut socket, listen) = fixture().await;
    let content = three_chunks();
    let mut task = chunked_send(&channel, content.clone());
    let mut ids = std::collections::HashSet::new();
    let mut reconstructed = String::new();
    for _ in 0..3 {
        let frame = next_frame(&mut socket).await;
        assert_eq!(frame["headers"]["req_id"], "callback-a");
        assert_eq!(frame["body"]["stream"]["finish"], true);
        let part = frame["body"]["stream"]["content"].as_str().unwrap();
        assert!(part.len() <= 3000, "each final stream body must fit the byte budget");
        reconstructed.push_str(part);
        assert!(serde_json::to_vec(&frame).unwrap().len() <= 16 * 1024);
        assert!(ids.insert(frame["body"]["stream"]["id"].as_str().unwrap().to_owned()));
        pending(&mut task).await;
        assert!(timeout(Duration::from_millis(40), socket.next()).await.is_err());
        ack(&mut socket, "callback-a", json!(0)).await;
    }
    timeout(Duration::from_secs(2), task).await.unwrap().unwrap().unwrap();
    assert_eq!(reconstructed, content);
    listen.abort();
    let _ = listen.await;
}

#[tokio::test]
async fn markdown_delivery_wecom_partial_failure_stops_after_ack() {
    let (channel, mut socket, listen) = fixture().await;
    let task = chunked_send(&channel, three_chunks());
    next_frame(&mut socket).await;
    ack(&mut socket, "callback-a", json!(0)).await;
    let second = next_frame(&mut socket).await;
    assert!(second["body"]["stream"]["content"].as_str().unwrap().contains('乙'));
    ack(&mut socket, "callback-a", json!(45009)).await;
    let error = timeout(Duration::from_secs(2), task).await.unwrap().unwrap().unwrap_err();
    let error = error.downcast_ref::<ChannelDeliveryError>().unwrap();
    assert_eq!((error.confirmed_parts, error.total_parts, error.failed_part_index), (1, 3, 1));
    assert_eq!(error.kind, DeliveryFailureKind::Rejected);
    assert!(timeout(Duration::from_millis(40), socket.next()).await.is_err());
    listen.abort();
    let _ = listen.await;
}

#[tokio::test]
async fn markdown_delivery_wecom_preflight_prevents_partial_table_send() {
    let (channel, mut socket, listen) = fixture().await;
    let content = format!("prefix\n\n| 名称 |\n| --- |\n| {} |", "界".repeat(1100));
    let task = chunked_send(&channel, content);
    assert!(
        timeout(Duration::from_millis(40), socket.next()).await.is_err(),
        "invalid late structure must prevent every frame"
    );
    let error = timeout(Duration::from_secs(2), task).await.unwrap().unwrap().unwrap_err();
    let error = error.downcast_ref::<ChannelDeliveryError>().unwrap();
    assert_eq!(error.kind, DeliveryFailureKind::NotSent);
    assert_eq!(error.confirmed_parts, 0);
    listen.abort();
    let _ = listen.await;
}

#[tokio::test]
async fn markdown_delivery_wecom_callback_update_keeps_original_reply_association() {
    let (channel, mut socket, listen, mut callbacks) = fixture_with_callbacks().await;
    let task = chunked_send(&channel, three_chunks());
    next_frame(&mut socket).await;
    socket
        .send(Message::Text(
            json!({
                "cmd": "aibot_msg_callback", "headers": {"req_id": "callback-new"},
                "body": {"msgtype": "text", "chatid": "chat-a", "from": {"userid": "chat-a"},
                         "text": {"content": "new callback while replying"}}
            })
            .to_string()
            .into(),
        ))
        .await
        .unwrap();
    timeout(Duration::from_secs(2), callbacks.recv()).await.unwrap().unwrap();
    assert_eq!(channel.reply_requests.read().await["chat-a"], "callback-new");
    ack(&mut socket, "callback-a", json!(0)).await;
    for _ in 0..2 {
        assert_eq!(next_frame(&mut socket).await["headers"]["req_id"], "callback-a");
        ack(&mut socket, "callback-a", json!(0)).await;
    }
    timeout(Duration::from_secs(2), task).await.unwrap().unwrap().unwrap();
    listen.abort();
    let _ = listen.await;
}

#[tokio::test]
async fn markdown_delivery_wecom_long_table_and_code_preserve_final_content() {
    let (channel, mut socket, listen) = fixture().await;
    let rows = (0..150)
        .map(|index| format!("| 项目{index:03} | 中文状态与 emoji 😀 |\n"))
        .collect::<Vec<_>>();
    let code = "~~~rust\nlet value = \"| code | pipes |\";\n~~~";
    let content = format!("| 名称 | 状态 |\n| --- | --- |\n{}\n{code}", rows.concat());
    let normalized =
        crate::app::agent::channels::outbound_format::format_wecom_markdown_tables(&content);
    let task = chunked_send(&channel, content);
    let mut parts = Vec::new();
    loop {
        let frame = next_frame(&mut socket).await;
        let part = frame["body"]["stream"]["content"].as_str().unwrap().to_owned();
        assert!(part.len() <= 3000);
        let has_code = part.contains(code);
        assert!(serde_json::to_vec(&frame).unwrap().len() <= 16 * 1024);
        if part.contains("| 名称") {
            assert!(part.contains(normalized.lines().nth(1).unwrap()));
            assert!(
                pulldown_cmark::Parser::new_ext(&part, pulldown_cmark::Options::ENABLE_TABLES).any(
                    |event| matches!(
                        event,
                        pulldown_cmark::Event::Start(pulldown_cmark::Tag::Table(_))
                    )
                )
            );
        }
        parts.push(part);
        ack(&mut socket, "callback-a", json!(0)).await;
        if has_code {
            break;
        }
        assert!(parts.len() < 10, "fixture reply must finish within ten frames");
    }
    timeout(Duration::from_secs(2), task).await.unwrap().unwrap().unwrap();
    assert!(parts.len() >= 3);
    let joined = parts.concat();
    for row in normalized.lines().filter(|line| line.contains("项目")) {
        assert_eq!(joined.matches(row).count(), 1, "data rows must be neither lost nor repeated");
    }
    assert_eq!(joined.matches(code).count(), 1);
    listen.abort();
    let _ = listen.await;
}

#[tokio::test]
async fn markdown_delivery_wecom_frame_budget_and_code_preflight_send_nothing() {
    for case in 0..3 {
        let (channel, mut socket, listen) = fixture().await;
        let content = if case == 1 {
            channel.reply_requests.write().await.insert("chat-a".into(), "r".repeat(16 * 1024));
            three_chunks()
        } else if case == 2 {
            format!("{}\n\n{}", "甲".repeat(650), "\u{0001}".repeat(2900))
        } else {
            format!("prefix\n\n~~~rust\n{}\n~~~", "x".repeat(3000))
        };
        let task = chunked_send(&channel, content);
        let error = timeout(Duration::from_secs(2), task).await.unwrap().unwrap().unwrap_err();
        let error = error.downcast_ref::<ChannelDeliveryError>().unwrap();
        assert_eq!(error.kind, DeliveryFailureKind::NotSent);
        assert_eq!(error.confirmed_parts, 0);
        assert!(timeout(Duration::from_millis(40), socket.next()).await.is_err());
        listen.abort();
        let _ = listen.await;
    }
}

#[tokio::test]
async fn markdown_delivery_wecom_partial_unknown_stops_and_late_ack_never_replays() {
    for timed_out in [false, true] {
        let (channel, mut socket, listen) = fixture().await;
        let task = chunked_send(&channel, three_chunks());
        next_frame(&mut socket).await;
        ack(&mut socket, "callback-a", json!(0)).await;
        next_frame(&mut socket).await;
        if timed_out {
            tokio::time::pause();
            tokio::time::advance(Duration::from_secs(6)).await;
        } else {
            ack(&mut socket, "callback-a", Value::Null).await;
        }
        let error = timeout(Duration::from_secs(2), task).await.unwrap().unwrap().unwrap_err();
        let error = error.downcast_ref::<ChannelDeliveryError>().unwrap();
        assert_eq!((error.confirmed_parts, error.total_parts, error.failed_part_index), (1, 3, 1));
        assert_eq!(error.kind, DeliveryFailureKind::Unknown);
        ack(&mut socket, "callback-a", json!(0)).await;
        result(send(&channel, "chat-a"), DeliveryFailureKind::NotSent).await;
        assert!(timeout(Duration::from_millis(40), socket.next()).await.is_err());
        listen.abort();
        let _ = listen.await;
        if timed_out {
            tokio::time::resume();
        }
    }
}

#[tokio::test]
async fn markdown_delivery_wecom_partial_disconnect_reconnect_has_no_old_chunks() {
    let (mut channel, mut old_socket, old_listen) = fixture().await;
    let task = chunked_send(&channel, three_chunks());
    next_frame(&mut old_socket).await;
    ack(&mut old_socket, "callback-a", json!(0)).await;
    next_frame(&mut old_socket).await;
    old_socket.close(None).await.unwrap();
    let error = timeout(Duration::from_secs(2), task).await.unwrap().unwrap().unwrap_err();
    let error = error.downcast_ref::<ChannelDeliveryError>().unwrap();
    assert_eq!((error.confirmed_parts, error.total_parts, error.failed_part_index), (1, 3, 1));
    assert_eq!(error.kind, DeliveryFailureKind::Unknown);
    assert!(timeout(Duration::from_secs(2), old_listen).await.unwrap().unwrap().is_err());
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    channel.endpoint = format!("ws://{}", listener.local_addr().unwrap());
    let cloned = channel.clone();
    let (tx, mut rx) = mpsc::channel(1);
    let listen = tokio::spawn(async move { cloned.listen(tx).await });
    let (stream, _) = timeout(Duration::from_secs(2), listener.accept()).await.unwrap().unwrap();
    let mut socket = accept_async(stream).await.unwrap();
    let subscribe = next_frame(&mut socket).await;
    ack(&mut socket, subscribe["headers"]["req_id"].as_str().unwrap(), json!(0)).await;
    assert!(timeout(Duration::from_millis(40), socket.next()).await.is_err());
    socket
        .send(Message::Text(
            json!({
                "cmd": "aibot_msg_callback", "headers": {"req_id": "callback-new"},
                "body": {"msgtype": "text", "chatid": "chat-a", "from": {"userid": "chat-a"},
                         "text": {"content": "new connection"}}
            })
            .to_string()
            .into(),
        ))
        .await
        .unwrap();
    timeout(Duration::from_secs(2), rx.recv()).await.unwrap().unwrap();
    let task = send(&channel, "chat-a");
    let frame = next_frame(&mut socket).await;
    assert_eq!(frame["headers"]["req_id"], "callback-new");
    assert_eq!(frame["body"]["stream"]["content"], "fixture response");
    ack(&mut socket, "callback-new", json!(0)).await;
    timeout(Duration::from_secs(2), task).await.unwrap().unwrap().unwrap();
    assert!(timeout(Duration::from_millis(40), socket.next()).await.is_err());
    listen.abort();
    let _ = listen.await;
}
