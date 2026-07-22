use super::super::InMemoryMessageBus;
use crate::app::agent::coordination::{
    CoordinationEnvelope, CoordinationError, CoordinationPayload,
};

fn ack_envelope(message_id: &str) -> CoordinationEnvelope {
    let mut envelope = CoordinationEnvelope::new_direct(
        "agent-a",
        "agent-b",
        "conversation-a",
        "topic",
        CoordinationPayload::Ack { acked_message_id: message_id.to_string() },
    );
    envelope.id = format!("envelope-{message_id}");
    envelope
}

#[test]
fn inbox_readers_peek_and_drain_registered_agent() {
    let bus = InMemoryMessageBus::new();
    bus.register_agent("agent-b").expect("register should succeed");
    let envelope = ack_envelope("message-a");

    bus.publish(envelope).expect("publish should succeed");

    assert_eq!(bus.pending_for_agent("agent-b").expect("pending count"), 1);
    assert_eq!(bus.peek_for_agent("agent-b", 1).expect("peek").len(), 1);
    assert_eq!(bus.drain_for_agent("agent-b", 0).expect("drain").len(), 1);
    assert_eq!(bus.pending_for_agent("agent-b").expect("pending count"), 0);
}

#[test]
fn inbox_readers_cover_errors_empty_correlation_and_unbounded_peek() {
    let bus = InMemoryMessageBus::new();

    assert!(matches!(
        bus.drain_for_agent("missing", 0),
        Err(CoordinationError::UnknownAgent { agent }) if agent == "missing"
    ));
    assert!(matches!(
        bus.pending_for_agent("missing"),
        Err(CoordinationError::UnknownAgent { agent }) if agent == "missing"
    ));
    assert_eq!(bus.pending_for_agent_correlation("missing", " "), Ok(0));
    assert!(matches!(
        bus.pending_for_agent_correlation("missing", "corr-a"),
        Err(CoordinationError::UnknownAgent { agent }) if agent == "missing"
    ));

    bus.register_agent("agent-b").expect("register should succeed");
    let mut first = ack_envelope("message-a");
    first.correlation_id = Some(" corr-a ".to_string());
    let mut second = ack_envelope("message-b");
    second.correlation_id = Some("corr-a".to_string());
    bus.publish(first).expect("first publish should succeed");
    bus.publish(second).expect("second publish should succeed");

    assert_eq!(bus.peek_for_agent_with_offset("agent-b", 1, 0).expect("peek").len(), 1);
    assert!(
        bus.peek_for_agent_correlation_with_offset("agent-b", " ", 0, 1)
            .expect("blank correlation should succeed")
            .is_empty()
    );
    assert!(matches!(
        bus.peek_for_agent_correlation_with_offset("missing", "corr-a", 0, 1),
        Err(CoordinationError::UnknownAgent { agent }) if agent == "missing"
    ));

    assert_eq!(bus.pending_for_agent_correlation("agent-b", "corr-a").expect("count"), 2);
    assert_eq!(bus.drain_for_agent("agent-b", 1).expect("drain one").len(), 1);
    assert_eq!(bus.pending_for_agent_correlation("agent-b", "corr-a").expect("count"), 1);
}

#[test]
fn inbox_peek_applies_offset_and_limit_without_consuming() {
    let bus = InMemoryMessageBus::new();
    bus.register_agent("agent-b").expect("register should succeed");

    for message_id in ["message-a", "message-b", "message-c"] {
        bus.publish(ack_envelope(message_id)).expect("publish should succeed");
    }

    let first_two = bus.peek_for_agent("agent-b", 2).expect("peek first two");
    assert_eq!(first_two.len(), 2);
    assert_eq!(first_two[0].envelope.id, "envelope-message-a");
    assert_eq!(first_two[1].envelope.id, "envelope-message-b");
    assert_eq!(bus.pending_for_agent("agent-b").expect("pending count"), 3);

    let offset_one = bus.peek_for_agent_with_offset("agent-b", 1, 1).expect("peek offset");
    assert_eq!(offset_one.len(), 1);
    assert_eq!(offset_one[0].envelope.id, "envelope-message-b");
    assert!(bus.peek_for_agent_with_offset("agent-b", 8, 0).expect("peek past end").is_empty());

    assert!(matches!(
        bus.peek_for_agent("missing", 0),
        Err(CoordinationError::UnknownAgent { agent }) if agent == "missing"
    ));
}

#[test]
fn inbox_correlation_readers_filter_page_and_clear_after_drain() {
    let bus = InMemoryMessageBus::new();
    bus.register_agent("agent-b").expect("register should succeed");

    let mut first = ack_envelope("message-a");
    first.correlation_id = Some("corr-a".to_string());
    let mut second = ack_envelope("message-b");
    second.correlation_id = Some("corr-b".to_string());
    let mut third = ack_envelope("message-c");
    third.correlation_id = Some(" corr-a ".to_string());

    bus.publish(first).expect("first publish should succeed");
    bus.publish(second).expect("second publish should succeed");
    bus.publish(third).expect("third publish should succeed");

    assert_eq!(bus.pending_for_agent_correlation("agent-b", "corr-a").expect("corr-a count"), 2);
    assert_eq!(bus.pending_for_agent_correlation("agent-b", "corr-b").expect("corr-b count"), 1);
    assert_eq!(bus.pending_for_agent_correlation("agent-b", "missing").expect("missing count"), 0);

    let first_corr =
        bus.peek_for_agent_correlation_with_offset("agent-b", "corr-a", 0, 1).expect("first corr");
    assert_eq!(first_corr.len(), 1);
    assert_eq!(first_corr[0].envelope.id, "envelope-message-a");

    let second_corr =
        bus.peek_for_agent_correlation_with_offset("agent-b", "corr-a", 1, 0).expect("second corr");
    assert_eq!(second_corr.len(), 1);
    assert_eq!(second_corr[0].envelope.id, "envelope-message-c");
    assert!(
        bus.peek_for_agent_correlation_with_offset("agent-b", "corr-a", 4, 0)
            .expect("corr past end")
            .is_empty()
    );

    assert_eq!(bus.drain_for_agent("agent-b", 0).expect("drain all").len(), 3);
    assert_eq!(bus.pending_for_agent_correlation("agent-b", "corr-a").expect("corr-a count"), 0);
    assert!(
        bus.peek_for_agent_correlation_with_offset("agent-b", "corr-a", 0, 0)
            .expect("empty corr")
            .is_empty()
    );
}
