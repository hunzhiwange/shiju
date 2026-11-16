use crate::session_events::to_session_history;
use crate::history::conversation_history_key as scoped_key;
fn conversation_history_key(msg: &channel_types::ChannelMessage) -> String {
    scoped_key(msg, vw_api_transport::TenantScope::default())
}
use model_types::ChatMessage;
use channel_types as traits;
pub(crate) fn channel_message(thread_ts: Option<&str>) -> traits::ChannelMessage {
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
fn to_session_history_maps_known_roles_and_skips_unknown_roles() {
    let turns = vec![
        ChatMessage::system("system"),
        ChatMessage::user("user"),
        ChatMessage::assistant("assistant"),
        ChatMessage::tool("tool"),
        ChatMessage { role: "unknown".to_string(), content: "skip".to_string() },
    ];

    let history = to_session_history(&turns);

    assert_eq!(history.len(), 4);
    assert!(matches!(history[0].role, session_port::ui_types::ChatRole::System));
    assert!(matches!(history[1].role, session_port::ui_types::ChatRole::User));
    assert!(matches!(history[2].role, session_port::ui_types::ChatRole::Assistant));
    assert!(matches!(history[3].role, session_port::ui_types::ChatRole::Tool));
    assert!(history.iter().all(|message| message.think_timing.is_empty()));
}

#[test]
pub(crate) fn channel_project_scope_rooms_have_independent_history() {
    let a = channel_message(None);
    let mut b = a.clone();
    b.reply_target = "room-2".into();
    assert_ne!(conversation_history_key(&a), conversation_history_key(&b));
}

#[test]
pub(crate) fn channel_project_scope_instances_have_independent_history() {
    let a = channel_message(None);
    let mut b = a.clone();
    b.instance_id = "second".into();
    assert_ne!(conversation_history_key(&a), conversation_history_key(&b));
}

#[test]
pub(crate) fn channel_project_scope_threads_have_independent_history() {
    let a = channel_message(None);
    let mut b = a.clone();
    b.thread_ts = Some("thread-2".into());
    assert_ne!(conversation_history_key(&a), conversation_history_key(&b));
}

#[test]
pub(crate) fn channel_project_scope_senders_have_independent_history() {
    let a = channel_message(None);
    let mut b = a.clone();
    b.sender = "sender-2".into();
    assert_ne!(conversation_history_key(&a), conversation_history_key(&b));
}

#[test]
pub(crate) fn channel_project_scope_identity_delimiters_do_not_collide() {
    let mut a = channel_message(None);
    a.channel = "a_b".into();
    a.sender = "c".into();
    let mut b = a.clone();
    b.channel = "a".into();
    b.sender = "b_c".into();
    assert_ne!(conversation_history_key(&a), conversation_history_key(&b));
}
