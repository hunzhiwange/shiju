//! 测试 PostgreSQL 发件箱行映射保留投递、失败、重试状态与载荷。
//! 负数尝试次数和畸形 JSON 被视为损坏记录，而非自动修正。

use anyhow::Result;
use serde_json::json;
use vw_api_types::common::TimestampMs;

use super::map_outbox_message_row;
use crate::agent_scheduler::store::outbox::OutboxMessageStatus;
use crate::global_database::test_support::PostgresTestDatabase;

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
            "SELECT 'message-1'::TEXT, 'local'::TEXT, 'runner'::TEXT, NULL::TEXT,
                    'body'::TEXT, 'pending'::TEXT, 'key'::TEXT, (-1)::BIGINT,
                    NULL::BIGINT, NULL::BIGINT, NULL::TEXT, '{}'::TEXT, 1::BIGINT, 1::BIGINT",
            &[],
        )?;
        let error = map_outbox_message_row(&negative_attempts)
            .expect_err("negative delivery attempts must fail");
        assert!(error.to_string().contains("PostgreSQL outbox attempts out of range: -1"));

        let malformed_payload = connection.query_one(
            "SELECT 'message-1'::TEXT, 'local'::TEXT, 'runner'::TEXT, NULL::TEXT,
                    'body'::TEXT, 'pending'::TEXT, 'key'::TEXT, 0::BIGINT,
                    NULL::BIGINT, NULL::BIGINT, NULL::TEXT, '{'::TEXT, 1::BIGINT, 1::BIGINT",
            &[],
        )?;
        let error = map_outbox_message_row(&malformed_payload)
            .expect_err("malformed outbox payload must fail");
        assert!(error.to_string().contains("parse PostgreSQL outbox payload"));

        let unknown_status = connection.query_one(
            "SELECT 'message-1'::TEXT, 'local'::TEXT, 'runner'::TEXT, NULL::TEXT,
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
