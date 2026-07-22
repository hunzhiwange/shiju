use super::*;
use std::sync::{Arc, Mutex};

struct DummyChannel;

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl Channel for DummyChannel {
    fn name(&self) -> &str {
        "dummy"
    }

    async fn send(&self, _message: &SendMessage) -> anyhow::Result<()> {
        Ok(())
    }

    async fn listen(&self, tx: tokio::sync::mpsc::Sender<ChannelMessage>) -> anyhow::Result<()> {
        tx.send(ChannelMessage {
            id: "1".into(),
            sender: "tester".into(),
            reply_target: "tester".into(),
            content: "hello".into(),
            channel: "dummy".into(),
            timestamp: 123,
            thread_ts: Some("thread-1".into()),
        })
        .await
        .map_err(|error| anyhow::anyhow!(error.to_string()))
    }
}

#[derive(Clone, Default)]
struct RecordingChannel {
    sent: Arc<Mutex<Vec<SendMessage>>>,
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl Channel for RecordingChannel {
    fn name(&self) -> &str {
        "recording"
    }

    async fn send(&self, message: &SendMessage) -> anyhow::Result<()> {
        self.sent.lock().unwrap().push(message.clone());
        Ok(())
    }

    async fn listen(&self, _tx: tokio::sync::mpsc::Sender<ChannelMessage>) -> anyhow::Result<()> {
        Ok(())
    }
}

struct FailingSendChannel;

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl Channel for FailingSendChannel {
    fn name(&self) -> &str {
        "failing"
    }

    async fn send(&self, _message: &SendMessage) -> anyhow::Result<()> {
        anyhow::bail!("send failed")
    }

    async fn listen(&self, _tx: tokio::sync::mpsc::Sender<ChannelMessage>) -> anyhow::Result<()> {
        Ok(())
    }
}

#[test]
fn channel_message_clone_preserves_fields() {
    let message = ChannelMessage {
        id: "42".into(),
        sender: "alice".into(),
        reply_target: "alice-thread".into(),
        content: "ping".into(),
        channel: "dummy".into(),
        timestamp: 999,
        thread_ts: Some("thread-9".into()),
    };

    let cloned = message.clone();

    assert_eq!(cloned.id, "42");
    assert_eq!(cloned.sender, "alice");
    assert_eq!(cloned.reply_target, "alice-thread");
    assert_eq!(cloned.content, "ping");
    assert_eq!(cloned.channel, "dummy");
    assert_eq!(cloned.timestamp, 999);
    assert_eq!(cloned.thread_ts.as_deref(), Some("thread-9"));
}

#[test]
fn send_message_new_sets_only_required_fields() {
    let message = SendMessage::new("hello", "alice");

    assert_eq!(message.content, "hello");
    assert_eq!(message.recipient, "alice");
    assert!(message.subject.is_none());
    assert!(message.thread_ts.is_none());
}

#[test]
fn send_message_with_subject_sets_subject_without_thread() {
    let message = SendMessage::with_subject("body", "bob", "subject");

    assert_eq!(message.content, "body");
    assert_eq!(message.recipient, "bob");
    assert_eq!(message.subject.as_deref(), Some("subject"));
    assert!(message.thread_ts.is_none());
}

#[test]
fn send_message_in_thread_sets_and_clears_thread() {
    let threaded = SendMessage::new("body", "bob").in_thread(Some("thread-1".to_string()));
    assert_eq!(threaded.thread_ts.as_deref(), Some("thread-1"));

    let unthreaded = threaded.in_thread(None);
    assert!(unthreaded.thread_ts.is_none());
}

#[tokio::test]
async fn channel_required_methods_keep_contract() {
    let channel = DummyChannel;
    let message = SendMessage::new("hello", "bob");

    assert_eq!(channel.name(), "dummy");
    assert!(channel.send(&message).await.is_ok());
}

#[tokio::test]
async fn default_health_and_typing_methods_return_success() {
    let channel = DummyChannel;

    assert!(channel.health_check().await);
    assert!(channel.start_typing("bob").await.is_ok());
    assert!(channel.stop_typing("bob").await.is_ok());
}

#[tokio::test]
async fn default_draft_methods_are_noop_successes() {
    let channel = DummyChannel;

    assert!(!channel.supports_draft_updates());
    assert!(channel.send_draft(&SendMessage::new("draft", "bob")).await.unwrap().is_none());
    assert!(channel.update_draft("bob", "msg_1", "text").await.unwrap().is_none());
    assert!(channel.finalize_draft("bob", "msg_1", "final text").await.is_ok());
    assert!(channel.cancel_draft("bob", "msg_1").await.is_ok());
}

#[tokio::test]
async fn default_reaction_methods_are_noop_successes() {
    let channel = DummyChannel;

    assert!(channel.add_reaction("chan_1", "msg_1", "eyes").await.is_ok());
    assert!(channel.remove_reaction("chan_1", "msg_1", "eyes").await.is_ok());
}

#[tokio::test]
async fn default_approval_prompt_sends_short_text_fallback_in_thread() {
    let channel = RecordingChannel::default();
    let args = serde_json::json!({"path": "/tmp/file.txt", "content": "hello"});

    channel
        .send_approval_prompt("alice", "req-1", "write_file", &args, Some("thread-9".to_string()))
        .await
        .unwrap();

    let sent = channel.sent.lock().unwrap();
    assert_eq!(sent.len(), 1);
    assert_eq!(sent[0].recipient, "alice");
    assert_eq!(sent[0].thread_ts.as_deref(), Some("thread-9"));
    assert!(sent[0].content.contains("Approval required for tool `write_file`"));
    assert!(sent[0].content.contains("Request ID: `req-1`"));
    assert!(sent[0].content.contains("Args: `"));
    assert!(sent[0].content.contains("/approve-allow req-1"));
    assert!(sent[0].content.contains("/approve-deny req-1"));
}

#[tokio::test]
async fn default_approval_prompt_truncates_long_argument_preview() {
    let channel = RecordingChannel::default();
    let args = serde_json::json!({"value": "x".repeat(260)});

    channel.send_approval_prompt("alice", "req-long", "tool", &args, None).await.unwrap();

    let sent = channel.sent.lock().unwrap();
    assert_eq!(sent.len(), 1);
    assert_eq!(sent[0].recipient, "alice");
    assert!(sent[0].thread_ts.is_none());
    assert!(sent[0].content.contains("..."));
    assert!(sent[0].content.len() < args.to_string().len() + 120);
}

#[tokio::test]
async fn default_approval_prompt_propagates_send_error() {
    let channel = FailingSendChannel;
    let args = serde_json::json!({"value": "short"});

    let error = channel
        .send_approval_prompt("alice", "req-fail", "tool", &args, None)
        .await
        .expect_err("send failure should propagate");

    assert!(error.to_string().contains("send failed"));
}

#[tokio::test]
async fn listen_sends_message_to_channel() {
    let channel = DummyChannel;
    let (tx, mut rx) = tokio::sync::mpsc::channel(1);

    channel.listen(tx).await.unwrap();

    let received = rx.recv().await.expect("message should be sent");
    assert_eq!(received.id, "1");
    assert_eq!(received.sender, "tester");
    assert_eq!(received.reply_target, "tester");
    assert_eq!(received.content, "hello");
    assert_eq!(received.channel, "dummy");
    assert_eq!(received.timestamp, 123);
    assert_eq!(received.thread_ts.as_deref(), Some("thread-1"));
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn channel_bounds_accept_send_sync_types() {
    fn assert_channel_bounds<T: ChannelBounds>() {}

    assert_channel_bounds::<DummyChannel>();
    assert_channel_bounds::<RecordingChannel>();
}

#[tokio::test]
async fn default_approval_prompt_keeps_full_args_at_boundary() {
    // 参数序列化后长度恰好为 220，应走非截断分支（不含省略号）。
    let channel = RecordingChannel::default();
    // {"v":"..."} 固定开销为 8 个字符，填充值长度 212 使总长正好 220。
    let args = serde_json::json!({"v": "y".repeat(212)});
    assert_eq!(args.to_string().len(), 220);

    channel.send_approval_prompt("alice", "req-edge", "tool", &args, None).await.unwrap();

    let sent = channel.sent.lock().unwrap();
    assert_eq!(sent.len(), 1);
    assert!(!sent[0].content.contains("..."));
    assert!(sent[0].content.contains(&"y".repeat(212)));
}

#[test]
fn channel_message_debug_includes_fields() {
    let message = ChannelMessage {
        id: "id-1".into(),
        sender: "sender-1".into(),
        reply_target: "reply-1".into(),
        content: "body".into(),
        channel: "chan".into(),
        timestamp: 7,
        thread_ts: None,
    };

    let rendered = format!("{message:?}");
    assert!(rendered.contains("ChannelMessage"));
    assert!(rendered.contains("id-1"));
    assert!(rendered.contains("thread_ts"));
}

#[test]
fn send_message_debug_includes_fields() {
    let message = SendMessage::with_subject("body", "bob", "subject");

    let rendered = format!("{message:?}");
    assert!(rendered.contains("SendMessage"));
    assert!(rendered.contains("subject"));
    assert!(rendered.contains("body"));
}
