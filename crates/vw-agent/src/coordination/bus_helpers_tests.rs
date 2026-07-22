use super::bus_helpers::{decrement_correlation_count, increment_correlation_count};
use super::envelope::{CoordinationEnvelope, CoordinationPayload};
use std::collections::HashMap;

#[test]
fn correlation_count_helpers_remove_empty_entries() {
    let mut counts = HashMap::new();
    let mut envelope = CoordinationEnvelope::new_broadcast(
        "agent-a",
        "conversation-a",
        "topic",
        CoordinationPayload::Control { action: "noop".to_string(), note: None },
    );
    envelope.correlation_id = Some("corr-a".to_string());

    increment_correlation_count(&mut counts, &envelope);
    increment_correlation_count(&mut counts, &envelope);
    decrement_correlation_count(&mut counts, &envelope);
    decrement_correlation_count(&mut counts, &envelope);

    assert!(!counts.contains_key("corr-a"));

    decrement_correlation_count(&mut counts, &envelope);
    assert!(counts.is_empty());
}

#[test]
fn correlation_count_helpers_trim_correlation_ids() {
    let mut counts = HashMap::new();
    let mut envelope = CoordinationEnvelope::new_broadcast(
        "agent-a",
        "conversation-a",
        "topic",
        CoordinationPayload::Control { action: "noop".to_string(), note: None },
    );
    envelope.correlation_id = Some(" corr-a ".to_string());

    increment_correlation_count(&mut counts, &envelope);

    assert_eq!(counts.get("corr-a"), Some(&1));
    assert!(!counts.contains_key(" corr-a "));
}

#[test]
fn correlation_count_helpers_ignore_missing_and_blank_correlation_ids() {
    let mut counts = HashMap::from([("corr-a".to_string(), 1)]);
    let mut envelope = CoordinationEnvelope::new_broadcast(
        "agent-a",
        "conversation-a",
        "topic",
        CoordinationPayload::Control { action: "noop".to_string(), note: None },
    );

    increment_correlation_count(&mut counts, &envelope);
    decrement_correlation_count(&mut counts, &envelope);

    envelope.correlation_id = Some("   ".to_string());
    increment_correlation_count(&mut counts, &envelope);
    decrement_correlation_count(&mut counts, &envelope);

    assert_eq!(counts, HashMap::from([("corr-a".to_string(), 1)]));
}
