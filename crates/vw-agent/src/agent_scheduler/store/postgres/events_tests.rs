use anyhow::Result;
use serde_json::json;
use vw_api_types::common::TimestampMs;

use super::map_event_row;
use crate::agent_scheduler::events::AgentEventKind;
use crate::agent_scheduler::state_machine::AgentTaskStatus;
use crate::global_database::test_support::PostgresTestDatabase;

#[test]
fn map_event_row_preserves_optional_identifiers_status_keys_and_payload() -> Result<()> {
    let fixture = crate::global_database::test_support::postgres_test_database!();
    let event = {
        let mut connection = fixture.database().connect_postgres()?;
        let row = connection.query_one(
            "SELECT 7::BIGINT, 'task-1'::TEXT, 'run-1'::TEXT, 'worker-a'::TEXT,
                    'team-a'::TEXT, 'task_assigned'::TEXT, 'queued'::TEXT,
                    'assigned'::TEXT, 'custom-queued'::TEXT, 'custom-assigned'::TEXT,
                    'assigned worker'::TEXT, '{\"nested\":[1,true]}'::TEXT, 1234::BIGINT",
            &[],
        )?;
        map_event_row(&row)?
    };

    assert_eq!(event.cursor, 7);
    assert_eq!(event.task_id.as_ref().map(|id| id.as_str()), Some("task-1"));
    assert_eq!(event.run_id.as_ref().map(|id| id.as_str()), Some("run-1"));
    assert_eq!(event.worker_id.as_ref().map(|id| id.as_str()), Some("worker-a"));
    assert_eq!(event.team_id.as_ref().map(|id| id.as_str()), Some("team-a"));
    assert_eq!(event.kind, AgentEventKind::TaskAssigned);
    assert_eq!(event.from_status, Some(AgentTaskStatus::Queued));
    assert_eq!(event.to_status, Some(AgentTaskStatus::Assigned));
    assert_eq!(event.from_status_key.as_deref(), Some("custom-queued"));
    assert_eq!(event.to_status_key.as_deref(), Some("custom-assigned"));
    assert_eq!(event.message, "assigned worker");
    assert_eq!(event.payload, json!({"nested": [1, true]}));
    assert_eq!(event.created_at_ms, TimestampMs(1_234));

    fixture.cleanup()
}

#[test]
fn map_event_row_rejects_unknown_kind_and_malformed_payload() -> Result<()> {
    let fixture = crate::global_database::test_support::postgres_test_database!();
    {
        let mut connection = fixture.database().connect_postgres()?;
        let unknown_kind = connection.query_one(
            "SELECT 1::BIGINT, NULL::TEXT, NULL::TEXT, NULL::TEXT, NULL::TEXT,
                    'unknown'::TEXT, NULL::TEXT, NULL::TEXT, NULL::TEXT, NULL::TEXT,
                    'message'::TEXT, '{}'::TEXT, 1::BIGINT",
            &[],
        )?;
        let error = map_event_row(&unknown_kind).expect_err("unknown event kind must fail");
        assert!(error.to_string().contains("parse PostgreSQL event kind"));

        let malformed_payload = connection.query_one(
            "SELECT 1::BIGINT, NULL::TEXT, NULL::TEXT, NULL::TEXT, NULL::TEXT,
                    'human_intervened'::TEXT, NULL::TEXT, NULL::TEXT, NULL::TEXT,
                    NULL::TEXT, 'message'::TEXT, '{'::TEXT, 1::BIGINT",
            &[],
        )?;
        let error =
            map_event_row(&malformed_payload).expect_err("malformed event payload must fail");
        assert!(error.to_string().contains("parse PostgreSQL scheduler event payload"));
    }
    fixture.cleanup()
}
