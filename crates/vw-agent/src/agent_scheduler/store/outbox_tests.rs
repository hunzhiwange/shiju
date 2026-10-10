//! 测试发件箱存储的领取租约、重试状态与并发幂等性。
//! 并发领取屏障确认同一消息只有一个持有者，过期后才允许重新领取。

use std::path::Path;
use std::sync::{Arc, Barrier};

use anyhow::Result;
use rusqlite::{Connection, params};
use serde_json::json;
use vw_api_types::agent_scheduler::AgentSchedulerScope;
use vw_api_types::common::TimestampMs;
use vw_api_types::id::{ProjectId, ProjectScope, TenantScope};
use vw_shared::project::{Info as ProjectInfo, TimeInfo as ProjectTimeInfo};

use super::domain::AgentSchedulerContext;
use super::store::outbox::{EnqueueOutboxMessageRequest, OutboxMessageStatus, StoredOutboxMessage};
use super::store::{AgentSchedulerStorageContext, AgentSchedulerStore};
use crate::global_database::GlobalDatabase;
use crate::global_database::test_support::PostgresTestDatabase;
use crate::project::store::ProjectStore;

#[test]
fn outbox_message_status_as_str_returns_stable_storage_values() {
    assert_eq!(OutboxMessageStatus::Pending.as_str(), "pending");
    assert_eq!(OutboxMessageStatus::Claimed.as_str(), "claimed");
    assert_eq!(OutboxMessageStatus::Failed.as_str(), "failed");
    assert_eq!(OutboxMessageStatus::Delivered.as_str(), "delivered");
}


#[test]
fn outbox_store_enqueue_rejects_invalid_requests() -> Result<()> {
    let temp_dir = tempfile::tempdir()?;
    let project_path = temp_dir.path();
    let store = AgentSchedulerStore::new();

    assert_error_contains(
        store.enqueue_outbox_message(
            project_path,
            EnqueueOutboxMessageRequest {
                message_id: " ".to_string(),
                ..outbox_request("outbox-invalid", "invalid-message-id", TimestampMs(1_000))
            },
        ),
        "outbox message id must not be empty",
    );
    assert_error_contains(
        store.enqueue_outbox_message(
            project_path,
            EnqueueOutboxMessageRequest {
                channel: "\t".to_string(),
                ..outbox_request("outbox-invalid", "invalid-channel", TimestampMs(1_000))
            },
        ),
        "outbox channel must not be empty",
    );
    assert_error_contains(
        store.enqueue_outbox_message(
            project_path,
            EnqueueOutboxMessageRequest {
                target: "".to_string(),
                ..outbox_request("outbox-invalid", "invalid-target", TimestampMs(1_000))
            },
        ),
        "outbox target must not be empty",
    );
    assert_error_contains(
        store.enqueue_outbox_message(
            project_path,
            EnqueueOutboxMessageRequest {
                recipient_id: Some(" ".to_string()),
                ..outbox_request("outbox-invalid", "invalid-recipient", TimestampMs(1_000))
            },
        ),
        "outbox recipient id must not be empty",
    );
    assert_error_contains(
        store.enqueue_outbox_message(
            project_path,
            EnqueueOutboxMessageRequest {
                body: "\n".to_string(),
                ..outbox_request("outbox-invalid", "invalid-body", TimestampMs(1_000))
            },
        ),
        "outbox body must not be empty",
    );
    assert_error_contains(
        store.enqueue_outbox_message(
            project_path,
            EnqueueOutboxMessageRequest {
                idempotency_key: "".to_string(),
                ..outbox_request("outbox-invalid", "invalid-idempotency", TimestampMs(1_000))
            },
        ),
        "outbox idempotency key must not be empty",
    );
    assert_error_contains(
        store.enqueue_outbox_message(
            project_path,
            EnqueueOutboxMessageRequest {
                next_attempt_at_ms: Some(TimestampMs(999)),
                ..outbox_request("outbox-invalid", "invalid-next-at", TimestampMs(1_000))
            },
        ),
        "outbox next attempt time must not be before creation time",
    );

    Ok(())
}

#[test]
fn outbox_store_accepts_absent_recipient_and_retry_at_creation_time() -> Result<()> {
    let temp_dir = tempfile::tempdir()?;
    let project_path = temp_dir.path();
    let store = AgentSchedulerStore::new();
    let created_at_ms = TimestampMs(1_000);

    let stored = store.enqueue_outbox_message(
        project_path,
        EnqueueOutboxMessageRequest {
            recipient_id: None,
            next_attempt_at_ms: Some(created_at_ms),
            ..outbox_request("optional-recipient", "optional-recipient-key", created_at_ms)
        },
    )?;

    assert_eq!(stored.recipient_id, None);
    assert_eq!(stored.next_attempt_at_ms, Some(created_at_ms));
    assert!(store.claim_outbox_batch(project_path, TimestampMs(999), 1)?.is_empty());
    let claimed = store.claim_outbox_batch(project_path, created_at_ms, 1)?;
    assert_eq!(claimed.len(), 1);
    assert_eq!(claimed[0].id, "optional-recipient");

    Ok(())
}

#[test]
fn outbox_store_claims_due_messages_in_storage_order_and_respects_limit() -> Result<()> {
    for_each_outbox_store_provider(assert_outbox_claims_due_messages_in_storage_order)
}

fn assert_outbox_claims_due_messages_in_storage_order(
    store: &AgentSchedulerStore,
    context: &AgentSchedulerContext,
    isolated_context: &AgentSchedulerContext,
) -> Result<()> {
    store.enqueue_outbox_message(
        context,
        EnqueueOutboxMessageRequest {
            next_attempt_at_ms: Some(TimestampMs(1_300)),
            ..outbox_request("future", "future-key", TimestampMs(1_000))
        },
    )?;
    store.enqueue_outbox_message(
        context,
        EnqueueOutboxMessageRequest {
            next_attempt_at_ms: Some(TimestampMs(900)),
            ..outbox_request("due-by-next-at", "due-next-key", TimestampMs(800))
        },
    )?;
    store.enqueue_outbox_message(
        context,
        outbox_request("due-by-created-at-a", "due-created-a-key", TimestampMs(1_000)),
    )?;
    store.enqueue_outbox_message(
        context,
        outbox_request("due-by-created-at-b", "due-created-b-key", TimestampMs(1_000)),
    )?;

    store.enqueue_outbox_message(
        isolated_context,
        outbox_request("due-by-created-at-a", "due-created-a-key", TimestampMs(1_000)),
    )?;

    assert!(store.claim_outbox_batch(context, TimestampMs(1_200), 0)?.is_empty());
    let claimed = store.claim_outbox_batch(context, TimestampMs(1_200), 3)?;
    assert_eq!(
        claimed.iter().map(|message| message.id.as_str()).collect::<Vec<_>>(),
        vec!["due-by-next-at", "due-by-created-at-a", "due-by-created-at-b"]
    );
    assert!(claimed.iter().all(|message| message.status == OutboxMessageStatus::Claimed));
    assert!(claimed.iter().all(|message| message.attempts == 1));
    assert!(claimed.iter().all(|message| message.next_attempt_at_ms.is_none()));
    assert!(claimed.iter().all(|message| message.updated_at_ms == TimestampMs(1_200)));

    assert!(store.claim_outbox_batch(context, TimestampMs(1_200), 10)?.is_empty());
    let isolated = store.claim_outbox_batch(isolated_context, TimestampMs(1_200), 10)?;
    assert_eq!(isolated.len(), 1);
    assert_eq!(isolated[0].id, "due-by-created-at-a");
    let future = store.claim_outbox_batch(context, TimestampMs(1_300), 10)?;
    assert_eq!(future.len(), 1);
    assert_eq!(future[0].id, "future");

    Ok(())
}

#[test]
fn outbox_store_postgres_claims_message_once_under_concurrency() -> Result<()> {
    let fixture = crate::global_database::test_support::postgres_test_database!();
    let project_path = tempfile::tempdir()?;
    let database = fixture.database().clone();
    let (store, context, _) = outbox_store_fixture(database.clone(), project_path.path())?;
    let competing_store = AgentSchedulerStore::from_database(database);
    store.enqueue_outbox_message(
        &context,
        outbox_request("concurrent-message", "concurrent-key", TimestampMs(1_000)),
    )?;

    let barrier = Arc::new(Barrier::new(3));
    let claims = std::thread::scope(|scope| {
        let first_barrier = Arc::clone(&barrier);
        let first_store = &store;
        let first_context = &context;
        let first = scope.spawn(move || {
            first_barrier.wait();
            first_store.claim_outbox_batch(first_context, TimestampMs(1_100), 1)
        });
        let second_barrier = Arc::clone(&barrier);
        let second_store = &competing_store;
        let second_context = &context;
        let second = scope.spawn(move || {
            second_barrier.wait();
            second_store.claim_outbox_batch(second_context, TimestampMs(1_100), 1)
        });
        barrier.wait();
        Ok::<_, anyhow::Error>([
            first.join().expect("first outbox claim thread")?,
            second.join().expect("second outbox claim thread")?,
        ])
    })?;

    assert_eq!(claims.iter().map(Vec::len).sum::<usize>(), 1);
    fixture.cleanup()
}

#[test]
fn outbox_store_claim_limit_reports_out_of_range() -> Result<()> {
    let temp_dir = tempfile::tempdir()?;
    let error = AgentSchedulerStore::new()
        .claim_outbox_batch(temp_dir.path(), TimestampMs(1_000), usize::MAX)
        .expect_err("usize::MAX cannot fit in i64 on supported targets");

    assert!(error.to_string().contains("outbox claim limit out of range"));
    Ok(())
}

#[test]
fn outbox_store_delivery_only_updates_claimed_messages_and_clears_retry_state() -> Result<()> {
    let temp_dir = tempfile::tempdir()?;
    let project_path = temp_dir.path();
    let store = AgentSchedulerStore::new();

    store.enqueue_outbox_message(
        project_path,
        outbox_request("deliverable", "deliverable-key", TimestampMs(1_000)),
    )?;
    assert!(
        store.mark_outbox_delivered(project_path, "deliverable", TimestampMs(1_100))?.is_none()
    );

    let claimed = store.claim_outbox_batch(project_path, TimestampMs(1_100), 1)?;
    assert_eq!(claimed.len(), 1);
    let failed = store
        .mark_outbox_failed(
            project_path,
            "deliverable",
            "send timed out".to_string(),
            TimestampMs(1_150),
            TimestampMs(1_200),
        )?
        .expect("claimed message can fail");
    assert_failed_message(&failed, "send timed out", TimestampMs(1_150), TimestampMs(1_200), 1);

    let retried = store.claim_outbox_batch(project_path, TimestampMs(1_200), 1)?;
    assert_eq!(retried.len(), 1);
    assert_eq!(retried[0].attempts, 2);
    assert_eq!(retried[0].last_error.as_deref(), Some("send timed out"));

    let delivered = store
        .mark_outbox_delivered(project_path, "deliverable", TimestampMs(1_250))?
        .expect("claimed retry can be delivered");
    assert_eq!(delivered.status, OutboxMessageStatus::Delivered);
    assert_eq!(delivered.delivered_at_ms, Some(TimestampMs(1_250)));
    assert_eq!(delivered.updated_at_ms, TimestampMs(1_250));
    assert_eq!(delivered.next_attempt_at_ms, None);
    assert_eq!(delivered.last_error, None);
    assert!(
        store.mark_outbox_delivered(project_path, "deliverable", TimestampMs(1_300))?.is_none()
    );
    assert!(store.mark_outbox_delivered(project_path, "missing", TimestampMs(1_300))?.is_none());

    Ok(())
}

#[test]
fn outbox_store_failure_validates_inputs_and_only_updates_claimed_messages() -> Result<()> {
    let temp_dir = tempfile::tempdir()?;
    let project_path = temp_dir.path();
    let store = AgentSchedulerStore::new();

    store.enqueue_outbox_message(
        project_path,
        outbox_request("failable", "failable-key", TimestampMs(1_000)),
    )?;

    assert_error_contains(
        store.mark_outbox_delivered(project_path, "", TimestampMs(1_100)),
        "outbox message id must not be empty",
    );
    assert_error_contains(
        store.mark_outbox_failed(
            project_path,
            " ",
            "send failed".to_string(),
            TimestampMs(1_100),
            TimestampMs(1_200),
        ),
        "outbox message id must not be empty",
    );
    assert_error_contains(
        store.mark_outbox_failed(
            project_path,
            "failable",
            "".to_string(),
            TimestampMs(1_100),
            TimestampMs(1_200),
        ),
        "outbox failure message must not be empty",
    );
    assert_error_contains(
        store.mark_outbox_failed(
            project_path,
            "failable",
            "send failed".to_string(),
            TimestampMs(1_200),
            TimestampMs(1_199),
        ),
        "outbox next attempt time must not be before failure time",
    );
    assert!(
        store
            .mark_outbox_failed(
                project_path,
                "failable",
                "pending cannot fail".to_string(),
                TimestampMs(1_100),
                TimestampMs(1_200),
            )?
            .is_none()
    );
    assert!(
        store
            .mark_outbox_failed(
                project_path,
                "missing",
                "missing cannot fail".to_string(),
                TimestampMs(1_100),
                TimestampMs(1_200),
            )?
            .is_none()
    );

    let claimed = store.claim_outbox_batch(project_path, TimestampMs(1_100), 1)?;
    assert_eq!(claimed.len(), 1);

    let failed = store
        .mark_outbox_failed(
            project_path,
            "failable",
            "gateway rejected message".to_string(),
            TimestampMs(1_150),
            TimestampMs(1_250),
        )?
        .expect("claimed message can fail");
    assert_failed_message(
        &failed,
        "gateway rejected message",
        TimestampMs(1_150),
        TimestampMs(1_250),
        1,
    );

    Ok(())
}

#[test]
fn outbox_store_reports_invalid_stored_rows_when_loading_by_idempotency_key() -> Result<()> {
    let temp_dir = tempfile::tempdir()?;
    let project_path = temp_dir.path();
    let store = AgentSchedulerStore::new();

    store.initialize_project(project_path)?;
    insert_raw_outbox_message(
        &store,
        project_path,
        "bad-status",
        "unknown",
        "bad-status-key",
        0,
        "{}",
    )?;
    insert_raw_outbox_message(
        &store,
        project_path,
        "bad-attempts",
        "pending",
        "bad-attempts-key",
        -1,
        "{}",
    )?;
    insert_raw_outbox_message(
        &store,
        project_path,
        "bad-payload",
        "pending",
        "bad-payload-key",
        0,
        "{",
    )?;

    assert_error_contains(
        store.enqueue_outbox_message(
            project_path,
            outbox_request("ignored-status", "bad-status-key", TimestampMs(2_000)),
        ),
        "unsupported outbox message status: unknown",
    );
    assert_error_contains(
        store.enqueue_outbox_message(
            project_path,
            outbox_request("ignored-attempts", "bad-attempts-key", TimestampMs(2_000)),
        ),
        "load outbox message by idempotency key: bad-attempts-key",
    );
    assert_error_contains(
        store.enqueue_outbox_message(
            project_path,
            outbox_request("ignored-payload", "bad-payload-key", TimestampMs(2_000)),
        ),
        "load outbox message by idempotency key: bad-payload-key",
    );

    Ok(())
}


fn assert_error_contains<T: std::fmt::Debug>(result: Result<T>, expected: &str) {
    let error = result.expect_err("operation should fail");
    assert!(
        error.chain().any(|cause| cause.to_string().contains(expected)),
        "expected error to contain {expected:?}, got {error:?}"
    );
}

fn assert_failed_message(
    message: &StoredOutboxMessage,
    expected_error: &str,
    failed_at_ms: TimestampMs,
    next_attempt_at_ms: TimestampMs,
    attempts: u32,
) {
    assert_eq!(message.status, OutboxMessageStatus::Failed);
    assert_eq!(message.attempts, attempts);
    assert_eq!(message.next_attempt_at_ms, Some(next_attempt_at_ms));
    assert_eq!(message.delivered_at_ms, None);
    assert_eq!(message.last_error.as_deref(), Some(expected_error));
    assert_eq!(message.updated_at_ms, failed_at_ms);
}

// 复用同一领取与重试场景，验证不同后端遵守相同发件箱状态机。
fn for_each_outbox_store_provider(
    mut check: impl FnMut(
        &AgentSchedulerStore,
        &AgentSchedulerContext,
        &AgentSchedulerContext,
    ) -> Result<()>,
) -> Result<()> {
    let sqlite_home = tempfile::tempdir()?;
    let sqlite_project = sqlite_home.path().join("project");
    let sqlite_database = GlobalDatabase::for_home(sqlite_home.path())?;
    let (sqlite_store, sqlite_context, sqlite_isolated_context) =
        outbox_store_fixture(sqlite_database, &sqlite_project)?;
    check(&sqlite_store, &sqlite_context, &sqlite_isolated_context)?;

    let postgres_fixture = crate::global_database::test_support::postgres_test_database!();
    let postgres_project = tempfile::tempdir()?;
    let (postgres_store, postgres_context, postgres_isolated_context) =
        outbox_store_fixture(postgres_fixture.database().clone(), postgres_project.path())?;
    check(&postgres_store, &postgres_context, &postgres_isolated_context)?;
    postgres_fixture.cleanup()
}

fn outbox_store_fixture(
    database: GlobalDatabase,
    project_path: &Path,
) -> Result<(AgentSchedulerStore, AgentSchedulerContext, AgentSchedulerContext)> {
    let primary_tenant = TenantScope::new("outbox-platform", "outbox-company")?;
    let isolated_tenant = TenantScope::new("outbox-platform-isolated", "outbox-company-isolated")?;
    let project_id = ProjectId::from("outbox-store-project");
    let project_store = ProjectStore::new(database.clone())?;
    for tenant in [&primary_tenant, &isolated_tenant] {
        project_store.create(
            tenant,
            &ProjectInfo {
                id: project_id.as_ref().to_owned(),
                worktree: project_path.to_string_lossy().into_owned(),
                vcs: None,
                name: Some("Outbox store project".to_owned()),
                icon: None,
                commands: None,
                time: ProjectTimeInfo { created: 0, updated: 0, initialized: None },
                sandboxes: Vec::new(),
            },
        )?;
    }
    let context = AgentSchedulerContext::new(
        AgentSchedulerScope::new(ProjectScope::new(primary_tenant, project_id.clone())?),
        project_path,
    )?;
    let isolated_context = AgentSchedulerContext::new(
        AgentSchedulerScope::new(ProjectScope::new(isolated_tenant, project_id)?),
        project_path,
    )?;
    Ok((AgentSchedulerStore::from_database(database), context, isolated_context))
}

fn outbox_request(
    message_id: &str,
    idempotency_key: &str,
    created_at_ms: TimestampMs,
) -> EnqueueOutboxMessageRequest {
    EnqueueOutboxMessageRequest {
        message_id: message_id.to_string(),
        channel: "local".to_string(),
        target: "!room:example.test".to_string(),
        recipient_id: Some("@worker:example.test".to_string()),
        body: "Task ready".to_string(),
        idempotency_key: idempotency_key.to_string(),
        payload: json!({"task_id": "task-1"}),
        created_at_ms,
        next_attempt_at_ms: None,
    }
}

fn insert_raw_outbox_message(
    store: &AgentSchedulerStore,
    project_path: &Path,
    message_id: &str,
    status: &str,
    idempotency_key: &str,
    attempts: i64,
    payload: &str,
) -> Result<()> {
    let conn = Connection::open(store.database_path())?;
    let scope = project_path.storage_scope()?;
    let project = scope.project_scope();
    conn.execute(
        "INSERT INTO scheduler_outbox_messages (
             platform_id, company_id, project_id, id, target_kind, target_id, recipient_id,
             body, status, idempotency_key,
             attempts, next_attempt_at_ms, delivered_at_ms, last_error, payload_json,
             created_at_ms, updated_at_ms
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, NULL, ?7, ?8, ?9, ?10, NULL, NULL, NULL, ?11, ?12, ?12)",
        params![
            project.tenant_scope().platform_id().as_ref(),
            project.tenant_scope().company_id().as_ref(),
            project.project_id().as_ref(),
            message_id,
            "local",
            "!room:example.test",
            "Raw message",
            status,
            idempotency_key,
            attempts,
            payload,
            1_000_i64,
        ],
    )?;
    Ok(())
}
