use super::bus_dead_letters::push_dead_letter_locked;
use super::envelope::{CoordinationEnvelope, CoordinationPayload};
use super::state::BusState;
use super::types::InMemoryMessageBusLimits;

#[test]
fn push_dead_letter_locked_applies_capacity_limit() {
    let mut state = BusState::with_limits(InMemoryMessageBusLimits {
        max_dead_letters: 1,
        ..InMemoryMessageBusLimits::recommended()
    });
    let first = CoordinationEnvelope::new_broadcast(
        "agent-a",
        "conversation-a",
        "topic",
        CoordinationPayload::Control { action: "first".to_string(), note: None },
    );
    let second = CoordinationEnvelope::new_broadcast(
        "agent-a",
        "conversation-a",
        "topic",
        CoordinationPayload::Control { action: "second".to_string(), note: None },
    );

    push_dead_letter_locked(&mut state, first, "no target".to_string());
    push_dead_letter_locked(&mut state, second, "still no target".to_string());

    assert_eq!(state.dead_letters.len(), 1);
    assert_eq!(state.stats.dead_letter_evictions_total, 1);
}

#[test]
fn push_dead_letter_locked_records_dead_letter_and_correlation_index() {
    let mut state = BusState::with_limits(InMemoryMessageBusLimits::recommended());
    let mut envelope = CoordinationEnvelope::new_broadcast(
        "agent-a",
        "conversation-a",
        "topic",
        CoordinationPayload::Control { action: "stop".to_string(), note: None },
    );
    envelope.id = "message-a".to_string();
    envelope.correlation_id = Some(" corr-a ".to_string());

    push_dead_letter_locked(&mut state, envelope, "missing recipient".to_string());

    assert_eq!(state.stats.dead_letters_total, 1);
    assert_eq!(state.stats.dead_letter_evictions_total, 0);
    assert_eq!(state.dead_letters.len(), 1);
    assert_eq!(state.dead_letters[0].envelope.id, "message-a");
    assert_eq!(state.dead_letters[0].reason, "missing recipient");
    let entries = state.dead_letters_by_correlation.get("corr-a").expect("correlation index");
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].envelope.id, "message-a");
    assert_eq!(entries[0].reason, "missing recipient");
}

#[test]
fn push_dead_letter_locked_ignores_blank_correlation_id() {
    let mut state = BusState::with_limits(InMemoryMessageBusLimits::recommended());
    let mut envelope = CoordinationEnvelope::new_broadcast(
        "agent-a",
        "conversation-a",
        "topic",
        CoordinationPayload::Control { action: "stop".to_string(), note: None },
    );
    envelope.correlation_id = Some("  ".to_string());

    push_dead_letter_locked(&mut state, envelope, "missing recipient".to_string());

    assert_eq!(state.dead_letters.len(), 1);
    assert!(state.dead_letters_by_correlation.is_empty());
}

#[test]
fn push_dead_letter_locked_removes_empty_correlation_index_on_eviction() {
    let mut state = BusState::with_limits(InMemoryMessageBusLimits {
        max_dead_letters: 1,
        ..InMemoryMessageBusLimits::recommended()
    });
    let mut first = CoordinationEnvelope::new_broadcast(
        "agent-a",
        "conversation-a",
        "topic",
        CoordinationPayload::Control { action: "first".to_string(), note: None },
    );
    first.correlation_id = Some("corr-a".to_string());
    let second = CoordinationEnvelope::new_broadcast(
        "agent-a",
        "conversation-a",
        "topic",
        CoordinationPayload::Control { action: "second".to_string(), note: None },
    );

    push_dead_letter_locked(&mut state, first, "first failure".to_string());
    push_dead_letter_locked(&mut state, second, "second failure".to_string());

    assert_eq!(state.dead_letters.len(), 1);
    assert!(!state.dead_letters_by_correlation.contains_key("corr-a"));
}

#[test]
fn push_dead_letter_locked_keeps_correlation_index_when_later_entries_remain() {
    let mut state = BusState::with_limits(InMemoryMessageBusLimits {
        max_dead_letters: 2,
        ..InMemoryMessageBusLimits::recommended()
    });
    let mut first = CoordinationEnvelope::new_broadcast(
        "agent-a",
        "conversation-a",
        "topic",
        CoordinationPayload::Control { action: "first".to_string(), note: None },
    );
    first.id = "message-first".to_string();
    first.correlation_id = Some("corr-a".to_string());
    let mut second = CoordinationEnvelope::new_broadcast(
        "agent-a",
        "conversation-a",
        "topic",
        CoordinationPayload::Control { action: "second".to_string(), note: None },
    );
    second.id = "message-second".to_string();
    second.correlation_id = Some("corr-a".to_string());
    let mut third = CoordinationEnvelope::new_broadcast(
        "agent-a",
        "conversation-a",
        "topic",
        CoordinationPayload::Control { action: "third".to_string(), note: None },
    );
    third.id = "message-third".to_string();

    push_dead_letter_locked(&mut state, first, "first failure".to_string());
    push_dead_letter_locked(&mut state, second, "second failure".to_string());
    push_dead_letter_locked(&mut state, third, "third failure".to_string());

    assert_eq!(state.dead_letters.len(), 2);
    assert_eq!(state.dead_letters[0].envelope.id, "message-second");
    assert_eq!(state.dead_letters[1].envelope.id, "message-third");
    assert_eq!(state.stats.dead_letter_evictions_total, 1);
    let entries = state.dead_letters_by_correlation.get("corr-a").expect("correlation index");
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].envelope.id, "message-second");
}

#[test]
fn push_dead_letter_locked_tolerates_missing_correlation_index_during_eviction() {
    let mut state = BusState::with_limits(InMemoryMessageBusLimits {
        max_dead_letters: 1,
        ..InMemoryMessageBusLimits::recommended()
    });
    let mut first = CoordinationEnvelope::new_broadcast(
        "agent-a",
        "conversation-a",
        "topic",
        CoordinationPayload::Control { action: "first".to_string(), note: None },
    );
    first.id = "message-first".to_string();
    first.correlation_id = Some("corr-a".to_string());
    let mut second = CoordinationEnvelope::new_broadcast(
        "agent-a",
        "conversation-a",
        "topic",
        CoordinationPayload::Control { action: "second".to_string(), note: None },
    );
    second.id = "message-second".to_string();

    push_dead_letter_locked(&mut state, first, "first failure".to_string());
    state.dead_letters_by_correlation.clear();
    push_dead_letter_locked(&mut state, second, "second failure".to_string());

    assert_eq!(state.dead_letters.len(), 1);
    assert_eq!(state.dead_letters[0].envelope.id, "message-second");
    assert!(state.dead_letters_by_correlation.is_empty());
    assert_eq!(state.stats.dead_letter_evictions_total, 1);
}
