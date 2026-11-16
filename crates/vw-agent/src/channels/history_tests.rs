use super::*;
fn channel_message(thread_ts: Option<&str>) -> traits::ChannelMessage {
    traits::ChannelMessage {
        instance_id: "default".to_string(),
        id: "msg-1".to_string(),
        sender: "sender-1".to_string(),
        reply_target: "room-1".to_string(),
        content: "hello".to_string(),
        channel: "telegram".to_string(),
        timestamp: 1,
        thread_ts: thread_ts.map(str::to_string),
    }
}

#[test]
fn conversation_keys_include_thread_when_present() {
    let threaded = channel_message(Some("thread-9"));
    let unthreaded = channel_message(None);

    assert_ne!(conversation_memory_key(&threaded), conversation_memory_key(&unthreaded));
    assert_ne!(conversation_history_key(&threaded), conversation_history_key(&unthreaded));
    let identity: serde_json::Value =
        serde_json::from_str(&conversation_history_key(&threaded)).unwrap();
    assert_eq!(identity["thread"], "thread-9");
    assert_eq!(identity["reply_target"], "room-1");
    assert_eq!(identity["instance"], "default");
    assert_eq!(interruption_scope_key(&threaded), conversation_history_key(&threaded));
}
