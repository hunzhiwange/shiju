use super::envelope::{CoordinationEnvelope, CoordinationPayload, DeliveryScope};
use super::errors::CoordinationError;
use serde_json::json;

#[test]
fn direct_envelope_validates_target() {
    let envelope = CoordinationEnvelope::new_direct(
        "agent-a",
        "agent-b",
        "conversation-a",
        "topic",
        CoordinationPayload::DelegateTask {
            task_id: "task-a".to_string(),
            summary: "summary".to_string(),
            metadata: json!({}),
        },
    );

    assert_eq!(envelope.scope, DeliveryScope::Direct);
    assert!(envelope.validate().is_ok());
}

#[test]
fn broadcast_envelope_rejects_direct_only_payload() {
    let envelope = CoordinationEnvelope::new_broadcast(
        "agent-a",
        "conversation-a",
        "topic",
        CoordinationPayload::DelegateTask {
            task_id: "task-a".to_string(),
            summary: "summary".to_string(),
            metadata: json!({}),
        },
    );

    assert!(envelope.validate().is_err());
}

#[test]
fn direct_envelope_rejects_missing_target() {
    let mut envelope = CoordinationEnvelope::new_direct(
        "agent-a",
        "agent-b",
        "conversation-a",
        "topic",
        CoordinationPayload::Ack { acked_message_id: "message-a".to_string() },
    );
    envelope.id = "direct-without-target".to_string();
    envelope.to = Some(" ".to_string());

    assert_eq!(
        envelope.validate(),
        Err(CoordinationError::MissingTarget { message_id: "direct-without-target".to_string() })
    );
}

#[test]
fn broadcast_envelope_rejects_explicit_target() {
    let mut envelope = CoordinationEnvelope::new_broadcast(
        "agent-a",
        "conversation-a",
        "topic",
        CoordinationPayload::Control { action: "pause".to_string(), note: None },
    );
    envelope.id = "broadcast-with-target".to_string();
    envelope.to = Some("agent-b".to_string());

    assert_eq!(
        envelope.validate(),
        Err(CoordinationError::BroadcastHasTarget {
            message_id: "broadcast-with-target".to_string(),
        })
    );
}

#[test]
fn control_payload_rejects_empty_action() {
    let envelope = CoordinationEnvelope::new_broadcast(
        "agent-a",
        "conversation-a",
        "topic",
        CoordinationPayload::Control { action: " ".to_string(), note: None },
    );

    assert_eq!(envelope.validate(), Err(CoordinationError::EmptyField { field: "action" }));
}

#[test]
fn envelope_rejects_empty_optional_ids() {
    let mut envelope = CoordinationEnvelope::new_broadcast(
        "agent-a",
        "conversation-a",
        "topic",
        CoordinationPayload::Control { action: "pause".to_string(), note: None },
    );
    envelope.correlation_id = Some(" ".to_string());

    assert_eq!(envelope.validate(), Err(CoordinationError::EmptyField { field: "correlation_id" }));

    envelope.correlation_id = None;
    envelope.causation_id = Some(" ".to_string());

    assert_eq!(envelope.validate(), Err(CoordinationError::EmptyField { field: "causation_id" }));
}

#[test]
fn context_patch_rejects_empty_key() {
    let envelope = CoordinationEnvelope::new_broadcast(
        "agent-a",
        "conversation-a",
        "topic",
        CoordinationPayload::ContextPatch {
            key: " ".to_string(),
            expected_version: 0,
            value: json!({"state": "ready"}),
        },
    );

    assert_eq!(envelope.validate(), Err(CoordinationError::EmptyField { field: "key" }));
}

#[test]
fn task_result_requires_correlation_id() {
    let mut envelope = CoordinationEnvelope::new_direct(
        "agent-b",
        "agent-a",
        "conversation-a",
        "topic",
        CoordinationPayload::TaskResult {
            task_id: "task-a".to_string(),
            success: true,
            output: "done".to_string(),
        },
    );
    envelope.id = "task-result-without-correlation".to_string();

    assert_eq!(
        envelope.validate(),
        Err(CoordinationError::MissingCorrelationId {
            message_id: "task-result-without-correlation".to_string(),
        })
    );
}

#[test]
fn task_result_rejects_empty_fields() {
    let mut envelope = CoordinationEnvelope::new_direct(
        "agent-b",
        "agent-a",
        "conversation-a",
        "topic",
        CoordinationPayload::TaskResult {
            task_id: " ".to_string(),
            success: false,
            output: "failed".to_string(),
        },
    );
    envelope.correlation_id = Some("message-a".to_string());

    assert_eq!(envelope.validate(), Err(CoordinationError::EmptyField { field: "task_id" }));

    envelope.payload = CoordinationPayload::TaskResult {
        task_id: "task-a".to_string(),
        success: false,
        output: " ".to_string(),
    };

    assert_eq!(envelope.validate(), Err(CoordinationError::EmptyField { field: "output" }));
}

#[test]
fn ack_rejects_empty_message_id() {
    let envelope = CoordinationEnvelope::new_direct(
        "agent-a",
        "agent-b",
        "conversation-a",
        "topic",
        CoordinationPayload::Ack { acked_message_id: " ".to_string() },
    );

    assert_eq!(
        envelope.validate(),
        Err(CoordinationError::EmptyField { field: "acked_message_id" })
    );
}
