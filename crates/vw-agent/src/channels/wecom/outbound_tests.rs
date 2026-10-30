use super::*;
use crate::app::agent::channels::outbound_delivery::{ChannelDeliveryError, DeliveryFailureKind};
use tokio::net::{TcpListener, TcpStream};
use tokio::task::JoinHandle;
use tokio::time::timeout;
use tokio_tungstenite::{WebSocketStream, accept_async};

type Socket = WebSocketStream<TcpStream>;

async fn fixture() -> (WeComChannel, Socket, JoinHandle<anyhow::Result<()>>) {
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
