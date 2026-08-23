//! 测试发件箱分派器的领取、投递、重试与幂等处理。

use anyhow::Result;
use serde_json::json;
use vw_api_types::common::TimestampMs;

use super::adapters::matrix::{
    FakeMatrixDispatchAdapter, MatrixDispatchRequest, NoMatrixDispatchAdapter,
};
use super::outbox_dispatcher::{
    DEFAULT_OUTBOX_RETRY_DELAY_MS, OutboxDispatcher, OutboxDrainSummary,
};
use super::store::AgentSchedulerStore;
use super::store::outbox::EnqueueOutboxMessageRequest;

struct EnabledTraceSubscriber;

impl tracing::Subscriber for EnabledTraceSubscriber {
    fn register_callsite(
        &self,
        _metadata: &'static tracing::Metadata<'static>,
    ) -> tracing::subscriber::Interest {
        tracing::subscriber::Interest::always()
    }

    fn enabled(&self, _metadata: &tracing::Metadata<'_>) -> bool {
        true
    }

    fn max_level_hint(&self) -> Option<tracing::metadata::LevelFilter> {
        Some(tracing::metadata::LevelFilter::TRACE)
    }

    fn new_span(&self, _span: &tracing::span::Attributes<'_>) -> tracing::Id {
        tracing::Id::from_u64(1)
    }

    fn record(&self, _span: &tracing::Id, _values: &tracing::span::Record<'_>) {}

    fn record_follows_from(&self, _span: &tracing::Id, _follows: &tracing::Id) {}

    fn event(&self, _event: &tracing::Event<'_>) {}

    fn enter(&self, _span: &tracing::Id) {}

    fn exit(&self, _span: &tracing::Id) {}
}

#[tokio::test]
async fn agent_outbox_dispatcher_marks_delivery_and_retries_failures() -> Result<()> {
    let temp_dir = tempfile::tempdir()?;
    let project_path = temp_dir.path();
    let store = AgentSchedulerStore::new();
    store.initialize_project(project_path)?;

    enqueue_message(
        &store,
        project_path,
        "outbox-delivered",
        "@worker-delivered:example.test already delivered",
        "task-delivered:dispatch",
        TimestampMs(900),
    )?;
    let delivered_claim = store.claim_outbox_batch(project_path, TimestampMs(950), 1)?;
    assert_eq!(delivered_claim.len(), 1);
    store.mark_outbox_delivered(project_path, "outbox-delivered", TimestampMs(960))?;

    enqueue_message(
        &store,
        project_path,
        "outbox-success",
        "@worker-a:example.test New task [task-success]: ship it",
        "task-success:dispatch",
        TimestampMs(1_000),
    )?;
    enqueue_message(
        &store,
        project_path,
        "outbox-failure",
        "@worker-b:example.test New task [task-failure]: retry it",
        "task-failure:dispatch",
        TimestampMs(1_001),
    )?;

    let adapter = FakeMatrixDispatchAdapter::new();
    adapter.succeed_next();
    adapter.fail_next("matrix send timed out");
    let dispatcher = OutboxDispatcher::new(&store, &adapter);

    let summary = dispatcher.drain_once(project_path, TimestampMs(1_100), 10).await?;
    assert_eq!(summary, OutboxDrainSummary { claimed: 2, delivered: 1, failed: 1 });

    assert_eq!(
        adapter.recorded_requests(),
        vec![
            MatrixDispatchRequest {
                channel: "matrix".to_string(),
                target: "room:!worker-room:example.test".to_string(),
                recipient_id: Some("@worker-a:example.test".to_string()),
                body: "@worker-a:example.test New task [task-success]: ship it".to_string(),
                payload_json: json!({"task_id": "outbox-success", "kind": "dispatch"}),
            },
            MatrixDispatchRequest {
                channel: "matrix".to_string(),
                target: "room:!worker-room:example.test".to_string(),
                recipient_id: Some("@worker-b:example.test".to_string()),
                body: "@worker-b:example.test New task [task-failure]: retry it".to_string(),
                payload_json: json!({"task_id": "outbox-failure", "kind": "dispatch"}),
            },
        ]
    );

    assert!(store.claim_outbox_batch(project_path, TimestampMs(1_200), 10)?.is_empty());
    let retried_at_due = store.claim_outbox_batch(
        project_path,
        TimestampMs(1_100 + DEFAULT_OUTBOX_RETRY_DELAY_MS),
        10,
    )?;
    assert_eq!(retried_at_due.len(), 1);
    assert_eq!(retried_at_due[0].id, "outbox-failure");
    assert_eq!(retried_at_due[0].attempts, 2);
    assert_eq!(retried_at_due[0].last_error.as_deref(), Some("matrix send timed out"));

    Ok(())
}

#[test]
fn agent_outbox_dispatcher_evaluates_trace_fields_when_subscriber_is_enabled() -> Result<()> {
    let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build()?;
    let temp_dir = tempfile::tempdir()?;
    let project_path = temp_dir.path();
    let store = AgentSchedulerStore::new();
    store.initialize_project(project_path)?;
    enqueue_message(
        &store,
        project_path,
        "outbox-trace-success",
        "@worker-a:example.test New task [trace-success]: send it",
        "task-trace-success:dispatch",
        TimestampMs(5_000),
    )?;
    enqueue_message(
        &store,
        project_path,
        "outbox-trace-failure",
        "@worker-b:example.test New task [trace-failure]: retry it",
        "task-trace-failure:dispatch",
        TimestampMs(5_001),
    )?;
    store.enqueue_outbox_message(
        project_path,
        EnqueueOutboxMessageRequest {
            message_id: "outbox-trace-local".to_string(),
            channel: "local".to_string(),
            target: "worker:worker-local".to_string(),
            recipient_id: Some("worker-local".to_string()),
            body: "local task ready".to_string(),
            idempotency_key: "task-trace-local:dispatch".to_string(),
            payload: json!({"task_id": "task-trace-local", "kind": "dispatch"}),
            created_at_ms: TimestampMs(5_002),
            next_attempt_at_ms: None,
        },
    )?;
    let adapter = FakeMatrixDispatchAdapter::new();
    adapter.succeed_next();
    adapter.fail_next("trace send failed");
    let dispatcher = OutboxDispatcher::new(&store, &adapter);
    let dispatch = tracing::Dispatch::new(EnabledTraceSubscriber);

    tracing::dispatcher::with_default(&dispatch, || {
        runtime.block_on(async {
            let summary = dispatcher.drain_once(project_path, TimestampMs(5_100), 10).await?;
            assert_eq!(summary, OutboxDrainSummary { claimed: 3, delivered: 1, failed: 2 });
            Result::<()>::Ok(())
        })
    })?;

    assert_eq!(adapter.recorded_requests().len(), 2);
    Ok(())
}

#[tokio::test]
async fn agent_outbox_dispatcher_retries_unsupported_channels_without_matrix_send() -> Result<()> {
    let temp_dir = tempfile::tempdir()?;
    let project_path = temp_dir.path();
    let store = AgentSchedulerStore::new();
    store.initialize_project(project_path)?;

    store.enqueue_outbox_message(
        project_path,
        EnqueueOutboxMessageRequest {
            message_id: "outbox-local".to_string(),
            channel: "local".to_string(),
            target: "worker:worker-local".to_string(),
            recipient_id: Some("worker-local".to_string()),
            body: "local task ready".to_string(),
            idempotency_key: "task-local:dispatch".to_string(),
            payload: json!({"task_id": "task-local", "kind": "dispatch"}),
            created_at_ms: TimestampMs(2_000),
            next_attempt_at_ms: None,
        },
    )?;

    let adapter = FakeMatrixDispatchAdapter::new();
    let dispatcher = OutboxDispatcher::new(&store, &adapter);

    let summary = dispatcher.drain_once(project_path, TimestampMs(2_100), 10).await?;
    assert_eq!(summary, OutboxDrainSummary { claimed: 1, delivered: 0, failed: 1 });
    assert!(adapter.recorded_requests().is_empty());

    assert!(store.claim_outbox_batch(project_path, TimestampMs(2_200), 10)?.is_empty());
    let retried_at_due = store.claim_outbox_batch(
        project_path,
        TimestampMs(2_100 + DEFAULT_OUTBOX_RETRY_DELAY_MS),
        10,
    )?;
    assert_eq!(retried_at_due.len(), 1);
    assert_eq!(retried_at_due[0].id, "outbox-local");
    assert_eq!(retried_at_due[0].attempts, 2);
    assert_eq!(
        retried_at_due[0].last_error.as_deref(),
        Some("unsupported outbox channel for Matrix dispatcher: local")
    );

    Ok(())
}

#[tokio::test]
async fn agent_scheduler_matrix_dispatch_is_optional_and_explicit() -> Result<()> {
    let temp_dir = tempfile::tempdir()?;
    let project_path = temp_dir.path();
    let store = AgentSchedulerStore::new();
    store.initialize_project(project_path)?;

    let no_matrix = NoMatrixDispatchAdapter;
    let dispatcher = OutboxDispatcher::new(&store, &no_matrix);

    assert_eq!(
        dispatcher.drain_once(project_path, TimestampMs(2_900), 10).await?,
        OutboxDrainSummary { claimed: 0, delivered: 0, failed: 0 }
    );

    enqueue_message(
        &store,
        project_path,
        "outbox-needs-matrix",
        "@worker-matrix:example.test New task [task-matrix]: deliver externally",
        "task-matrix:dispatch",
        TimestampMs(3_000),
    )?;

    let summary = dispatcher.drain_once(project_path, TimestampMs(3_100), 10).await?;
    assert_eq!(summary, OutboxDrainSummary { claimed: 1, delivered: 0, failed: 1 });

    let retry = store.claim_outbox_batch(
        project_path,
        TimestampMs(3_100 + DEFAULT_OUTBOX_RETRY_DELAY_MS),
        10,
    )?;
    assert_eq!(retry.len(), 1);
    assert_eq!(retry[0].id, "outbox-needs-matrix");
    assert_eq!(retry[0].attempts, 2);
    assert_eq!(
        retry[0].last_error.as_deref(),
        Some(
            "Matrix outbox dispatch is not configured; configure a Matrix dispatch adapter to deliver Matrix outbox messages"
        )
    );

    Ok(())
}

#[tokio::test]
async fn agent_outbox_dispatcher_normalizes_blank_failure_and_returns_claimed_messages()
-> Result<()> {
    let temp_dir = tempfile::tempdir()?;
    let project_path = temp_dir.path();
    let store = AgentSchedulerStore::new();
    store.initialize_project(project_path)?;
    enqueue_message(
        &store,
        project_path,
        "outbox-blank-error",
        "deliver with normalized failure",
        "task-blank-error:dispatch",
        TimestampMs(4_000),
    )?;
    let adapter = FakeMatrixDispatchAdapter::new();
    adapter.fail_next("   ");

    let (summary, claimed) = OutboxDispatcher::new(&store, &adapter)
        .drain_messages_once(project_path, TimestampMs(4_100), 1)
        .await?;

    assert_eq!(summary, OutboxDrainSummary { claimed: 1, delivered: 0, failed: 1 });
    assert_eq!(claimed.len(), 1);
    assert_eq!(claimed[0].id, "outbox-blank-error");
    let retry = store.claim_outbox_batch(
        project_path,
        TimestampMs(4_100 + DEFAULT_OUTBOX_RETRY_DELAY_MS),
        1,
    )?;
    assert_eq!(retry[0].last_error.as_deref(), Some("matrix dispatch failed"));
    Ok(())
}

fn enqueue_message(
    store: &AgentSchedulerStore,
    project_path: &std::path::Path,
    message_id: &str,
    body: &str,
    idempotency_key: &str,
    created_at_ms: TimestampMs,
) -> Result<()> {
    let worker = if message_id.ends_with("failure") { "worker-b" } else { "worker-a" };
    store.enqueue_outbox_message(
        project_path,
        EnqueueOutboxMessageRequest {
            message_id: message_id.to_string(),
            channel: "matrix".to_string(),
            target: "room:!worker-room:example.test".to_string(),
            recipient_id: Some(format!("@{worker}:example.test")),
            body: body.to_string(),
            idempotency_key: idempotency_key.to_string(),
            payload: json!({"task_id": message_id, "kind": "dispatch"}),
            created_at_ms,
            next_attempt_at_ms: None,
        },
    )?;
    Ok(())
}
