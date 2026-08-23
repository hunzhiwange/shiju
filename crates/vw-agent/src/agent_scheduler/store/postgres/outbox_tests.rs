//! 测试 PostgreSQL 发件箱的并发领取、租约与重试状态。

use anyhow::Result;
use serde_json::json;
use vw_api_types::common::TimestampMs;

use super::map_outbox_message_row;
use crate::agent_scheduler::store::outbox::OutboxMessageStatus;
use crate::global_database::test_support::PostgresTestDatabase;

#[test]
fn map_outbox_message_row_preserves_delivery_state_and_payload() -> Result<()> {
    let fixture = crate::global_database::test_support::postgres_test_database!();
    let message = {
        let mut connection = fixture.database().connect_postgres()?;
        let row = connection.query_one(
            "SELECT 'message-1'::TEXT, 'matrix'::TEXT, 'room:ops'::TEXT,
                    '@agent:example.test'::TEXT, 'delivered body'::TEXT, 'delivered'::TEXT,
                    'dispatch-task-1'::TEXT, 2::BIGINT, NULL::BIGINT, 1400::BIGINT,
                    NULL::TEXT, '{\"task_id\":\"task-1\"}'::TEXT, 1000::BIGINT, 1400::BIGINT",
            &[],
        )?;
        map_outbox_message_row(&row)?
    };

    assert_eq!(message.id, "message-1");
    assert_eq!(message.channel, "matrix");
    assert_eq!(message.target, "room:ops");
    assert_eq!(message.recipient_id.as_deref(), Some("@agent:example.test"));
    assert_eq!(message.body, "delivered body");
    assert_eq!(message.status, OutboxMessageStatus::Delivered);
    assert_eq!(message.idempotency_key, "dispatch-task-1");
    assert_eq!(message.attempts, 2);
    assert_eq!(message.next_attempt_at_ms, None);
    assert_eq!(message.delivered_at_ms, Some(TimestampMs(1_400)));
    assert_eq!(message.last_error, None);
    assert_eq!(message.payload, json!({"task_id": "task-1"}));
    assert_eq!(message.created_at_ms, TimestampMs(1_000));
    assert_eq!(message.updated_at_ms, TimestampMs(1_400));

    fixture.cleanup()
}

#[test]
fn map_outbox_message_row_preserves_failure_retry_state() -> Result<()> {
    let fixture = crate::global_database::test_support::postgres_test_database!();
    let message = {
        let mut connection = fixture.database().connect_postgres()?;
        let row = connection.query_one(
            "SELECT 'message-2'::TEXT, 'email'::TEXT, 'team:ops'::TEXT, NULL::TEXT,
                    'retry body'::TEXT, 'failed'::TEXT, 'dispatch-task-2'::TEXT, 3::BIGINT,
                    2500::BIGINT, NULL::BIGINT, 'rate limited'::TEXT,
                    '[\"task-2\"]'::TEXT, 1000::BIGINT, 1500::BIGINT",
            &[],
        )?;
        map_outbox_message_row(&row)?
    };

    assert_eq!(message.status, OutboxMessageStatus::Failed);
    assert_eq!(message.recipient_id, None);
    assert_eq!(message.attempts, 3);
    assert_eq!(message.next_attempt_at_ms, Some(TimestampMs(2_500)));
    assert_eq!(message.delivered_at_ms, None);
    assert_eq!(message.last_error.as_deref(), Some("rate limited"));
    assert_eq!(message.payload, json!(["task-2"]));

    fixture.cleanup()
}

#[test]
fn map_outbox_message_row_rejects_negative_attempts_and_malformed_payload() -> Result<()> {
    let fixture = crate::global_database::test_support::postgres_test_database!();
    {
        let mut connection = fixture.database().connect_postgres()?;
        let negative_attempts = connection.query_one(
            "SELECT 'message-1'::TEXT, 'matrix'::TEXT, 'room:ops'::TEXT, NULL::TEXT,
                    'body'::TEXT, 'pending'::TEXT, 'key'::TEXT, (-1)::BIGINT,
                    NULL::BIGINT, NULL::BIGINT, NULL::TEXT, '{}'::TEXT, 1::BIGINT, 1::BIGINT",
            &[],
        )?;
        let error = map_outbox_message_row(&negative_attempts)
            .expect_err("negative delivery attempts must fail");
        assert!(error.to_string().contains("PostgreSQL outbox attempts out of range: -1"));

        let malformed_payload = connection.query_one(
            "SELECT 'message-1'::TEXT, 'matrix'::TEXT, 'room:ops'::TEXT, NULL::TEXT,
                    'body'::TEXT, 'pending'::TEXT, 'key'::TEXT, 0::BIGINT,
                    NULL::BIGINT, NULL::BIGINT, NULL::TEXT, '{'::TEXT, 1::BIGINT, 1::BIGINT",
            &[],
        )?;
        let error = map_outbox_message_row(&malformed_payload)
            .expect_err("malformed outbox payload must fail");
        assert!(error.to_string().contains("parse PostgreSQL outbox payload"));

        let unknown_status = connection.query_one(
            "SELECT 'message-1'::TEXT, 'matrix'::TEXT, 'room:ops'::TEXT, NULL::TEXT,
                    'body'::TEXT, 'paused'::TEXT, 'key'::TEXT, 0::BIGINT,
                    NULL::BIGINT, NULL::BIGINT, NULL::TEXT, '{}'::TEXT, 1::BIGINT, 1::BIGINT",
            &[],
        )?;
        let error =
            map_outbox_message_row(&unknown_status).expect_err("unknown outbox status must fail");
        assert!(error.to_string().contains("parse PostgreSQL outbox status"));
    }
    fixture.cleanup()
}
