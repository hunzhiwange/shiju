use super::bus_publish::publish_envelope;
use super::envelope::{CoordinationEnvelope, CoordinationPayload};
use super::errors::CoordinationError;
use super::state::BusState;
use super::types::InMemoryMessageBusLimits;
use serde_json::json;

fn ack_envelope(id: &str, target: &str) -> CoordinationEnvelope {
    let mut envelope = CoordinationEnvelope::new_direct(
        "agent-a",
        target,
        "conversation-a",
        "topic",
        CoordinationPayload::Ack { acked_message_id: format!("acked-{id}") },
    );
    envelope.id = id.to_string();
    envelope
}

fn broadcast_control(id: &str, action: &str) -> CoordinationEnvelope {
    let mut envelope = CoordinationEnvelope::new_broadcast(
        "agent-a",
        "conversation-a",
        "topic",
        CoordinationPayload::Control { action: action.to_string(), note: None },
    );
    envelope.id = id.to_string();
    envelope
}

fn context_patch(id: &str, key: &str, expected_version: u64) -> CoordinationEnvelope {
    let mut envelope = CoordinationEnvelope::new_broadcast(
        "agent-a",
        "conversation-a",
        "context.patch",
        CoordinationPayload::ContextPatch {
            key: key.to_string(),
            expected_version,
            value: json!({"message": id}),
        },
    );
    envelope.id = id.to_string();
    envelope
}

#[test]
fn publish_envelope_invalid_message_is_dead_lettered_before_attempt_stats() {
    let mut state = BusState::with_limits(InMemoryMessageBusLimits::recommended());
    let mut envelope = ack_envelope("msg-invalid", "agent-b");
    envelope.to = None;

    let error = publish_envelope(&mut state, envelope).expect_err("invalid envelope should fail");

    assert_eq!(error, CoordinationError::MissingTarget { message_id: "msg-invalid".to_string() });
    assert_eq!(state.dead_letters.len(), 1);
    assert_eq!(state.dead_letters[0].envelope.id, "msg-invalid");
    assert_eq!(state.stats.publish_attempts_total, 0);
}

#[test]
fn publish_envelope_duplicate_message_is_dead_lettered_without_delivery() {
    let mut state = BusState::with_limits(InMemoryMessageBusLimits::recommended());
    state.inboxes.entry("agent-b".to_string()).or_default();

    publish_envelope(&mut state, ack_envelope("msg-duplicate", "agent-b"))
        .expect("first publish should succeed");
    let error = publish_envelope(&mut state, ack_envelope("msg-duplicate", "agent-b"))
        .expect_err("duplicate publish should fail");

    assert_eq!(
        error,
        CoordinationError::DuplicateMessageId { message_id: "msg-duplicate".to_string() }
    );
    assert_eq!(state.inboxes["agent-b"].len(), 1);
    assert_eq!(state.dead_letters.len(), 1);
    assert_eq!(state.dead_letters[0].envelope.id, "msg-duplicate");
    assert_eq!(state.stats.publish_attempts_total, 2);
    assert_eq!(state.stats.deliveries_total, 1);
}

#[test]
fn publish_envelope_direct_delivers_to_registered_agent() {
    let mut state = BusState::with_limits(InMemoryMessageBusLimits::recommended());
    state.inboxes.entry("agent-b".to_string()).or_default();
    let envelope = CoordinationEnvelope::new_direct(
        "agent-a",
        "agent-b",
        "conversation-a",
        "topic",
        CoordinationPayload::Ack { acked_message_id: "message-a".to_string() },
    );

    let receipt = publish_envelope(&mut state, envelope).expect("publish should succeed");

    assert_eq!(receipt.delivered_to, 1);
    assert_eq!(state.inboxes["agent-b"].len(), 1);
}

#[test]
fn publish_envelope_direct_unknown_target_is_dead_lettered() {
    let mut state = BusState::with_limits(InMemoryMessageBusLimits::recommended());
    let error = publish_envelope(&mut state, ack_envelope("msg-unknown", "missing"))
        .expect_err("unknown target should fail");

    assert_eq!(
        error,
        CoordinationError::UnknownTarget {
            agent: "missing".to_string(),
            message_id: "msg-unknown".to_string(),
        }
    );
    assert_eq!(state.dead_letters.len(), 1);
    assert_eq!(state.dead_letters[0].envelope.id, "msg-unknown");
}

#[test]
fn publish_envelope_direct_overflow_dead_letters_oldest_message() {
    let mut state = BusState::with_limits(InMemoryMessageBusLimits {
        max_inbox_messages_per_agent: 1,
        ..InMemoryMessageBusLimits::recommended()
    });
    state.inboxes.entry("agent-b".to_string()).or_default();

    publish_envelope(&mut state, ack_envelope("msg-1", "agent-b"))
        .expect("first publish should succeed");
    publish_envelope(&mut state, ack_envelope("msg-2", "agent-b"))
        .expect("second publish should succeed");

    assert_eq!(state.inboxes["agent-b"][0].envelope.id, "msg-2");
    assert_eq!(state.dead_letters.len(), 1);
    assert_eq!(state.dead_letters[0].envelope.id, "msg-1");
    assert_eq!(state.stats.inbox_overflow_evictions_total, 1);
}

#[test]
fn publish_envelope_context_patch_updates_shared_context() {
    let mut state = BusState::with_limits(InMemoryMessageBusLimits::recommended());
    let mut envelope = context_patch("context-1", "shared/key", 0);
    envelope.correlation_id = Some("corr-a".to_string());

    let receipt = publish_envelope(&mut state, envelope).expect("context patch should succeed");

    assert_eq!(receipt.sequence, 1);
    assert_eq!(receipt.delivered_to, 0);
    assert_eq!(state.context["shared/key"].version, 1);
    assert_eq!(state.context["shared/key"].updated_by, "agent-a");
    assert_eq!(state.context["shared/key"].last_message_id, "context-1");
    assert_eq!(state.context_order_by_correlation["corr-a"][0], "shared/key");
    assert_eq!(state.stats.deliveries_total, 0);
}

#[test]
fn publish_envelope_context_patch_conflict_is_dead_lettered() {
    let mut state = BusState::with_limits(InMemoryMessageBusLimits::recommended());

    publish_envelope(&mut state, context_patch("context-1", "shared/key", 0))
        .expect("initial context patch should succeed");
    let error = publish_envelope(&mut state, context_patch("context-2", "shared/key", 0))
        .expect_err("stale context patch should fail");

    assert_eq!(
        error,
        CoordinationError::ContextVersionMismatch {
            key: "shared/key".to_string(),
            expected: 0,
            actual: 1,
        }
    );
    assert_eq!(state.context["shared/key"].last_message_id, "context-1");
    assert_eq!(state.dead_letters.len(), 1);
    assert_eq!(state.dead_letters[0].envelope.id, "context-2");
    assert!(state.seen_message_ids.contains("context-2"));
    assert_eq!(state.next_sequence, 2);
}

#[test]
fn publish_envelope_broadcast_without_agents_still_advances_dedupe_window() {
    let mut state = BusState::with_limits(InMemoryMessageBusLimits {
        max_seen_message_ids: 1,
        ..InMemoryMessageBusLimits::recommended()
    });

    let first =
        publish_envelope(&mut state, broadcast_control("broadcast-1", "pause")).expect("first");
    let second =
        publish_envelope(&mut state, broadcast_control("broadcast-2", "resume")).expect("second");

    assert_eq!(first.delivered_to, 0);
    assert_eq!(second.delivered_to, 0);
    assert!(!state.seen_message_ids.contains("broadcast-1"));
    assert!(state.seen_message_ids.contains("broadcast-2"));
    assert_eq!(state.stats.seen_message_id_evictions_total, 1);
}

#[test]
fn publish_envelope_broadcast_overflow_dead_letters_dropped_messages() {
    let mut state = BusState::with_limits(InMemoryMessageBusLimits {
        max_inbox_messages_per_agent: 1,
        ..InMemoryMessageBusLimits::recommended()
    });
    state.inboxes.entry("agent-b".to_string()).or_default();
    state.inboxes.entry("agent-c".to_string()).or_default();

    publish_envelope(&mut state, broadcast_control("broadcast-1", "pause")).expect("first");
    let receipt =
        publish_envelope(&mut state, broadcast_control("broadcast-2", "resume")).expect("second");

    assert_eq!(receipt.delivered_to, 2);
    assert_eq!(state.inboxes["agent-b"][0].envelope.id, "broadcast-2");
    assert_eq!(state.inboxes["agent-c"][0].envelope.id, "broadcast-2");
    assert_eq!(state.dead_letters.len(), 2);
    assert!(state.dead_letters.iter().all(|entry| entry.envelope.id == "broadcast-1"));
    assert_eq!(state.stats.inbox_overflow_evictions_total, 2);
}

#[test]
fn publish_envelope_dedupe_eviction_tolerates_stale_order_entry() {
    let mut state = BusState::with_limits(InMemoryMessageBusLimits {
        max_seen_message_ids: 1,
        ..InMemoryMessageBusLimits::recommended()
    });
    state.seen_message_ids.insert("retained-message".to_string());
    state.seen_message_order.push_back("stale-message".to_string());

    let receipt = publish_envelope(&mut state, broadcast_control("new-message", "resume"))
        .expect("stale order entry must not prevent publishing");

    assert_eq!(receipt.delivered_to, 0);
    assert!(state.seen_message_ids.contains("retained-message"));
    assert!(state.seen_message_ids.contains("new-message"));
    assert_eq!(
        state.seen_message_order.iter().cloned().collect::<Vec<_>>(),
        vec!["new-message".to_string()]
    );
    assert_eq!(state.stats.seen_message_id_evictions_total, 0);
}
