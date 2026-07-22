use super::bus_context::apply_context_patch_locked;
use super::envelope::{CoordinationEnvelope, CoordinationPayload};
use super::errors::CoordinationError;
use super::state::BusState;
use super::types::InMemoryMessageBusLimits;
use serde_json::json;

fn context_envelope(correlation_id: Option<&str>) -> CoordinationEnvelope {
    let mut envelope = CoordinationEnvelope::new_broadcast(
        "agent-a",
        "conversation-a",
        "context.patch",
        CoordinationPayload::ContextPatch {
            key: "shared/key".to_string(),
            expected_version: 0,
            value: json!({"ok": true}),
        },
    );
    envelope.correlation_id = correlation_id.map(str::to_string);
    envelope
}

#[test]
fn apply_context_patch_locked_increments_version_and_indexes_correlation() {
    let mut state = BusState::with_limits(InMemoryMessageBusLimits::recommended());
    let envelope = context_envelope(Some("corr-a"));

    apply_context_patch_locked(&mut state, &envelope, "shared/key", 0, &json!("value"))
        .expect("initial patch should be accepted");

    let entry = state.context.get("shared/key").expect("entry should exist");
    assert_eq!(entry.version, 1);
    assert_eq!(state.context_order_by_correlation["corr-a"][0], "shared/key");
}

#[test]
fn apply_context_patch_locked_rejects_stale_version() {
    let mut state = BusState::with_limits(InMemoryMessageBusLimits::recommended());
    let envelope = context_envelope(None);

    apply_context_patch_locked(&mut state, &envelope, "shared/key", 1, &json!("value"))
        .expect_err("stale expected version should fail");
}

#[test]
fn apply_context_patch_locked_rejects_invalid_delegate_key() {
    let mut state = BusState::with_limits(InMemoryMessageBusLimits::recommended());
    let envelope = context_envelope(Some("corr-a"));

    let error =
        apply_context_patch_locked(&mut state, &envelope, "delegate/corr-a", 0, &json!("value"))
            .expect_err("delegate key without tail should fail");

    assert_eq!(
        error,
        CoordinationError::InvalidDelegateContextKey {
            key: "delegate/corr-a".to_string(),
            message_id: envelope.id,
        }
    );
}

#[test]
fn apply_context_patch_locked_rejects_delegate_key_without_correlation() {
    let mut state = BusState::with_limits(InMemoryMessageBusLimits::recommended());
    let envelope = context_envelope(None);

    let error = apply_context_patch_locked(
        &mut state,
        &envelope,
        "delegate/corr-a/state",
        0,
        &json!("value"),
    )
    .expect_err("delegate key without envelope correlation should fail");

    assert_eq!(
        error,
        CoordinationError::MissingDelegateContextCorrelation {
            key: "delegate/corr-a/state".to_string(),
            message_id: envelope.id,
        }
    );
}

#[test]
fn apply_context_patch_locked_rejects_delegate_correlation_mismatch() {
    let mut state = BusState::with_limits(InMemoryMessageBusLimits::recommended());
    let envelope = context_envelope(Some("corr-b"));

    let error = apply_context_patch_locked(
        &mut state,
        &envelope,
        "delegate/corr-a/state",
        0,
        &json!("value"),
    )
    .expect_err("delegate key correlation must match envelope correlation");

    assert_eq!(
        error,
        CoordinationError::DelegateContextCorrelationMismatch {
            key: "delegate/corr-a/state".to_string(),
            message_id: envelope.id,
            key_correlation_id: "corr-a".to_string(),
            envelope_correlation_id: "corr-b".to_string(),
        }
    );
}

#[test]
fn apply_context_patch_locked_updates_existing_key_and_replaces_correlation_index() {
    let mut state = BusState::with_limits(InMemoryMessageBusLimits::recommended());
    let first = context_envelope(Some("corr-a"));
    let second = context_envelope(None);
    let third = context_envelope(Some("corr-b"));

    apply_context_patch_locked(&mut state, &first, "shared/key", 0, &json!("first"))
        .expect("initial patch should be accepted");
    apply_context_patch_locked(&mut state, &second, "shared/key", 1, &json!("second"))
        .expect("second patch should be accepted");
    apply_context_patch_locked(&mut state, &third, "shared/key", 2, &json!("third"))
        .expect("third patch should be accepted");

    let entry = state.context.get("shared/key").expect("entry should exist");
    assert_eq!(entry.version, 3);
    assert_eq!(entry.value, json!("third"));
    assert_eq!(state.context_order.iter().filter(|key| key.as_str() == "shared/key").count(), 1);
    assert!(!state.context_order_by_correlation.contains_key("corr-a"));
    assert_eq!(state.context_order_by_correlation["corr-b"][0], "shared/key");
}

#[test]
fn apply_context_patch_locked_rewrites_delegate_indexes_on_update() {
    let mut state = BusState::with_limits(InMemoryMessageBusLimits::recommended());
    let envelope = context_envelope(Some("corr-a"));

    apply_context_patch_locked(&mut state, &envelope, "delegate/corr-a/state", 0, &json!("first"))
        .expect("initial delegate patch should be accepted");
    apply_context_patch_locked(&mut state, &envelope, "delegate/corr-a/state", 1, &json!("second"))
        .expect("second delegate patch should be accepted");

    assert_eq!(state.context["delegate/corr-a/state"].version, 2);
    assert_eq!(state.delegate_context_order.len(), 1);
    assert_eq!(state.delegate_context_order_by_correlation["corr-a"].len(), 1);
}

#[test]
fn apply_context_patch_locked_evicts_delegate_indexes_when_capacity_is_full() {
    let mut state = BusState::with_limits(InMemoryMessageBusLimits {
        max_context_entries: 1,
        ..InMemoryMessageBusLimits::recommended()
    });
    let delegate = context_envelope(Some("corr-a"));
    let shared = context_envelope(Some("corr-b"));

    apply_context_patch_locked(&mut state, &delegate, "delegate/corr-a/state", 0, &json!("first"))
        .expect("delegate patch should be accepted");
    apply_context_patch_locked(&mut state, &shared, "shared/key", 0, &json!("second"))
        .expect("shared patch should evict delegate key");

    assert!(!state.context.contains_key("delegate/corr-a/state"));
    assert!(state.context.contains_key("shared/key"));
    assert!(state.delegate_context_order.is_empty());
    assert!(!state.delegate_context_order_by_correlation.contains_key("corr-a"));
    assert!(!state.context_order_by_correlation.contains_key("corr-a"));
    assert_eq!(state.stats.context_evictions_total, 1);
}

#[test]
fn apply_context_patch_locked_evicts_uncorrelated_shared_key_when_capacity_is_full() {
    let mut state = BusState::with_limits(InMemoryMessageBusLimits {
        max_context_entries: 1,
        ..InMemoryMessageBusLimits::recommended()
    });
    let envelope = context_envelope(None);

    apply_context_patch_locked(&mut state, &envelope, "shared/first", 0, &json!("first"))
        .expect("first shared patch should be accepted");
    apply_context_patch_locked(&mut state, &envelope, "shared/second", 0, &json!("second"))
        .expect("second shared patch should evict the oldest key");

    assert!(!state.context.contains_key("shared/first"));
    assert_eq!(state.context["shared/second"].value, json!("second"));
    assert_eq!(
        state.context_order.iter().cloned().collect::<Vec<_>>(),
        vec!["shared/second".to_string()]
    );
    assert!(state.context_order_by_correlation.is_empty());
    assert!(state.context_correlation_by_key.is_empty());
    assert_eq!(state.stats.context_evictions_total, 1);
}
