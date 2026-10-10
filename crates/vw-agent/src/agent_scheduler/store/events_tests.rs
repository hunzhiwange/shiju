//! 测试调度事件存储的追加顺序、过滤与作用域隔离。
//! 固定时间与序号夹具验证追加顺序和游标边界，跨项目事件不可混入。

use anyhow::Result;
use rusqlite::params;
use serde_json::json;
use vw_api_types::agent_scheduler::AgentSchedulerScope;
use vw_api_types::common::TimestampMs;
use vw_api_types::id::{ProjectId, ProjectScope, TenantScope};
use vw_shared::project::{Info as ProjectInfo, TimeInfo as ProjectTimeInfo};

use super::domain::{AgentSchedulerContext, AssigneeId, RunId, TaskId};
use super::events::AgentEventKind;
use super::state_machine::AgentTaskStatus;
use super::store::AgentSchedulerStore;
use super::store::events::AppendAgentEventRequest;
use super::store::outbox::{EnqueueOutboxMessageRequest, OutboxMessageStatus};
use crate::global_database::GlobalDatabase;
use crate::global_database::test_support::PostgresTestDatabase;
use crate::project::store::ProjectStore;

#[test]
fn agent_scheduler_events_and_outbox_are_scope_local() -> Result<()> {
    let temp_dir = tempfile::tempdir()?;
    let store = AgentSchedulerStore::new();
    let scope_a = scheduler_context(temp_dir.path(), "platform-a", "company-a", "project")?;
    let scope_b = scheduler_context(temp_dir.path(), "platform-b", "company-b", "project")?;

    store.initialize_project(temp_dir.path())?;
    let event_a = store.append_event(&scope_a, scope_event("tenant A", TimestampMs(1_000)))?;
    let event_b = store.append_event(&scope_b, scope_event("tenant B", TimestampMs(1_001)))?;
    assert_eq!(store.list_events_after(&scope_a, None, 10)?, vec![event_a]);
    assert_eq!(store.list_events_after(&scope_b, None, 10)?, vec![event_b]);

    let message_a = store.enqueue_outbox_message(&scope_a, scope_message("tenant A"))?;
    let message_b = store.enqueue_outbox_message(&scope_b, scope_message("tenant B"))?;
    assert_eq!(message_a.id, message_b.id);
    assert_eq!(message_a.idempotency_key, message_b.idempotency_key);

    let claimed_a = store.claim_outbox_batch(&scope_a, TimestampMs(1_100), 10)?;
    assert_eq!(claimed_a.len(), 1);
    assert_eq!(claimed_a[0].body, "tenant A");
    assert!(store.claim_outbox_batch(&scope_a, TimestampMs(1_100), 10)?.is_empty());
    let claimed_b = store.claim_outbox_batch(&scope_b, TimestampMs(1_100), 10)?;
    assert_eq!(claimed_b.len(), 1);
    assert_eq!(claimed_b[0].body, "tenant B");

    store.mark_outbox_delivered(&scope_a, &message_a.id, TimestampMs(1_200))?;
    store.mark_outbox_failed(
        &scope_b,
        &message_b.id,
        "tenant B retry".to_string(),
        TimestampMs(1_200),
        TimestampMs(1_300),
    )?;
    assert!(store.claim_outbox_batch(&scope_a, TimestampMs(1_300), 10)?.is_empty());
    assert_eq!(store.claim_outbox_batch(&scope_b, TimestampMs(1_300), 10)?.len(), 1);

    Ok(())
}

fn scheduler_context(
    project_path: &std::path::Path,
    platform_id: &str,
    company_id: &str,
    project_id: &str,
) -> Result<AgentSchedulerContext> {
    let tenant = TenantScope::new(platform_id, company_id)?;
    let project = ProjectScope::new(tenant, ProjectId::from(project_id))?;
    AgentSchedulerContext::new(AgentSchedulerScope::new(project), project_path)
}

fn scope_event(message: &str, created_at_ms: TimestampMs) -> AppendAgentEventRequest {
    AppendAgentEventRequest {
        task_id: None,
        run_id: None,
        worker_id: None,
        team_id: None,
        kind: AgentEventKind::HumanIntervened,
        from_status: None,
        to_status: None,
        from_status_key: None,
        to_status_key: None,
        message: message.to_string(),
        payload: json!({}),
        created_at_ms,
    }
}

#[test]
fn agent_scheduler_event_store_pages_ordered_scope_local_events() -> Result<()> {
    for_each_event_store_provider(assert_pages_ordered_scope_local_events)
}

#[test]
fn agent_scheduler_event_store_preserves_explicit_status_keys() -> Result<()> {
    let temp_dir = tempfile::tempdir()?;
    let store = AgentSchedulerStore::new();
    store.initialize_project(temp_dir.path())?;

    let event = store.append_event(
        temp_dir.path(),
        AppendAgentEventRequest {
            task_id: None,
            run_id: None,
            worker_id: None,
            team_id: None,
            kind: AgentEventKind::HumanIntervened,
            from_status: Some(AgentTaskStatus::Blocked),
            to_status: Some(AgentTaskStatus::Queued),
            from_status_key: Some("external_blocked".to_string()),
            to_status_key: Some("external_queue".to_string()),
            message: "external workflow resumed".to_string(),
            payload: json!({"source": "external"}),
            created_at_ms: TimestampMs(1_250),
        },
    )?;

    assert_eq!(event.from_status, Some(AgentTaskStatus::Blocked));
    assert_eq!(event.to_status, Some(AgentTaskStatus::Queued));
    assert_eq!(event.from_status_key.as_deref(), Some("external_blocked"));
    assert_eq!(event.to_status_key.as_deref(), Some("external_queue"));

    Ok(())
}

#[test]
fn agent_scheduler_event_store_rejects_malformed_payload_rows() -> Result<()> {
    let temp_dir = tempfile::tempdir()?;
    let store = AgentSchedulerStore::new();
    let context =
        scheduler_context(temp_dir.path(), "events-platform-bad", "events-company-bad", "project")?;
    store.initialize_project(&context)?;
    let scope = context.scope().project_scope();
    let conn = rusqlite::Connection::open(store.database_path())?;

    conn.execute(
        "INSERT INTO scheduler_events (
             platform_id, company_id, project_id, kind, message, payload_json, created_at_ms
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            scope.platform_id().as_ref(),
            scope.company_id().as_ref(),
            scope.project_id().as_ref(),
            AgentEventKind::TaskAssigned.as_str(),
            "malformed payload",
            "{not-json",
            1_500_i64,
        ],
    )?;

    let error = store
        .list_events_after(&context, None, 10)
        .expect_err("malformed payload row should fail conversion");

    assert!(error.to_string().contains("list agent events"));
    Ok(())
}

fn assert_pages_ordered_scope_local_events(
    store: &AgentSchedulerStore,
    context: &AgentSchedulerContext,
    isolated_context: &AgentSchedulerContext,
) -> Result<()> {
    store.initialize_project(context)?;
    store.initialize_project(isolated_context)?;

    let first = store.append_event(context, scope_event("first event", TimestampMs(1_000)))?;
    store.append_event(isolated_context, scope_event("isolated event", TimestampMs(1_050)))?;
    let second = store.append_event(context, scope_event("second event", TimestampMs(1_100)))?;

    assert!(second.cursor > first.cursor);
    assert_eq!(
        store
            .list_events_after(context, None, 10)?
            .iter()
            .map(|event| event.message.as_str())
            .collect::<Vec<_>>(),
        vec!["first event", "second event"]
    );
    assert_eq!(store.list_events_after(context, Some(first.cursor), 1)?, vec![second]);
    let isolated_events = store.list_events_after(isolated_context, None, 10)?;
    assert_eq!(isolated_events.len(), 1);
    assert_eq!(isolated_events[0].message, "isolated event");
    assert!(store.list_events_after(context, None, 0)?.is_empty());

    Ok(())
}

// 跨后端复用顺序和作用域断言，防止实现间出现可见性差异。
fn for_each_event_store_provider(
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
        event_store_fixture(sqlite_database, &sqlite_project)?;
    check(&sqlite_store, &sqlite_context, &sqlite_isolated_context)?;

    let postgres_fixture = crate::global_database::test_support::postgres_test_database!();
    let postgres_project = tempfile::tempdir()?;
    let (postgres_store, postgres_context, postgres_isolated_context) =
        event_store_fixture(postgres_fixture.database().clone(), postgres_project.path())?;
    check(&postgres_store, &postgres_context, &postgres_isolated_context)?;
    postgres_fixture.cleanup()
}

fn event_store_fixture(
    database: GlobalDatabase,
    project_path: &std::path::Path,
) -> Result<(AgentSchedulerStore, AgentSchedulerContext, AgentSchedulerContext)> {
    let primary_tenant = TenantScope::new("events-platform", "events-company")?;
    let isolated_tenant = TenantScope::new("events-platform-isolated", "events-company-isolated")?;
    let project_id = ProjectId::from("events-store-project");
    let project_store = ProjectStore::new(database.clone())?;
    for tenant in [&primary_tenant, &isolated_tenant] {
        project_store.create(
            tenant,
            &ProjectInfo {
                id: project_id.as_ref().to_owned(),
                worktree: project_path.to_string_lossy().into_owned(),
                vcs: None,
                name: Some("Events store project".to_owned()),
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

fn scope_message(body: &str) -> EnqueueOutboxMessageRequest {
    EnqueueOutboxMessageRequest {
        message_id: "shared-message".to_string(),
        channel: "local".to_string(),
        target: "room:shared".to_string(),
        recipient_id: None,
        body: body.to_string(),
        idempotency_key: "shared-idempotency".to_string(),
        payload: json!({}),
        created_at_ms: TimestampMs(1_050),
        next_attempt_at_ms: None,
    }
}
