use crate::channel_reply_fixture::*;
use parking_lot::Mutex;
type AppState = gateway_types::AppState<(), (), ()>;
fn state() -> AppState { super::status::state(false, &[]) }
fn successful_gateway_chat<'a>(
    _state: &'a AppState,
    _message: &'a str,
    _key: &'a str,
) -> GatewayChatFuture<'a> {
    Box::pin(async { Ok("\nreply from injected runner\n".to_string()) })
}

#[tokio::test]
async fn process_channel_messages_with_chat_sends_successful_reply() {
    let state = state();
    let channel = RecordingChannel::default();
    let messages = vec![channel_message()];

    process_channel_messages_with_chat(
        &state,
        "Recording",
        &channel,
        &messages,
        |msg| format!("recording:{}", msg.sender),
        "recording message",
        true,
        successful_gateway_chat,
    )
    .await;

    let sent = channel.sent.lock();
    assert_eq!(sent.len(), 1);
    assert_eq!(sent[0].recipient, "target-1");
    assert_eq!(sent[0].thread_ts.as_deref(), Some("thread-1"));
    assert_eq!(sent[0].content, "reply from injected runner");
}

#[tokio::test]
async fn send_successful_channel_reply_sanitizes_and_omits_thread_when_disabled() {
    let channel = RecordingChannel::default();
    let msg = channel_message();
    let tools = Vec::new();

    send_successful_channel_reply(
        "Recording",
        &channel,
        &msg,
        "\n\nsuccessful reply\n\n",
        &tools,
        false,
    )
    .await;

    let sent = channel.sent.lock();
    assert_eq!(sent.len(), 1);
    assert_eq!(sent[0].recipient, "target-1");
    assert!(sent[0].thread_ts.is_none());
    assert_eq!(sent[0].content, "successful reply");
}

#[tokio::test]
async fn send_successful_channel_reply_logs_send_errors_without_panicking() {
    let channel = RecordingChannel { sent: Mutex::new(Vec::new()), fail_send: true };
    let msg = channel_message();
    let tools = Vec::new();

    send_successful_channel_reply("Recording", &channel, &msg, "reply", &tools, true).await;

    let sent = channel.sent.lock();
    assert_eq!(sent.len(), 1);
    assert_eq!(sent[0].thread_ts.as_deref(), Some("thread-1"));
}
