//! 测试调度器跨存储、策略与运行器组件的集成流程。
//! 使用真实存储门面串联任务、工作代理和运行记录，覆盖组件边界上的状态一致性。

use std::fs;
use std::path::Path;

use anyhow::Result;
use rusqlite::{Connection, params};
use serde_json::{Value, json};
use vw_api_types::agent_scheduler::AgentSchedulerScope;
use vw_api_types::common::TimestampMs;
use vw_api_types::id::{ProjectId, ProjectScope, TenantScope};
use vw_api_types::task::TaskPoolWorkflowSnapshotDto;
use vw_api_types::task_lifecycle::TaskLifecycleWorkflowUpsertBody;
use vw_shared::project::{Info, TimeInfo};
use vw_shared::session::info::{Info as AgentSessionInfo, TimeInfo as AgentSessionTimeInfo};
use vw_shared::session::ui_types::{
    ChatMessage, ChatRole, ChatSession, ChatSessionStep, SessionTodoItem, TokenUsage,
};

use super::artifacts::TaskArtifactWriter;
use super::domain::{
    AgentRunStatus, AgentSchedulerContext, HolderId, ResourceId, ResourceLeaseId, RunId, TaskId,
    TeamId, AssigneeId,
};
use super::engine::dispatch::DispatchEngine;
use super::events::AgentEventKind;
use super::outbox_dispatcher::{OutboxDispatcher, OutboxDrainSummary};
use super::state_machine::{AgentTaskKind, AgentTaskStatus};
use super::store::AgentSchedulerStore;
use super::store::events::AppendAgentEventRequest;
use super::store::logs::{AgentSchedulerWorkspaceContext, AppendAgentTaskLogRequest};
use super::store::outbox::EnqueueOutboxMessageRequest;
use super::store::resources::{AcquireResourceLeaseRequest, ResourceKind};
use super::store::runs::AgentRunFinishStatus;
use super::store::runs::ClaimTaskRunRequest;
use super::store::tasks::NewAgentTask;
use super::task_graph::AgentTaskDependency;
use crate::global_database::GlobalDatabase;
use crate::global_database::test_support::PostgresTestDatabase;
use crate::project::store::ProjectStore;
use crate::session::store::SessionStore;

const GLOBAL_SCOPED_TABLES: [&str; 19] = [
    "projects",
    "scheduler_meta",
    "scheduler_tasks",
    "scheduler_task_dependencies",
    "scheduler_runs",
    "scheduler_task_logs",
    "scheduler_task_artifacts",
    "scheduler_run_workspaces",
    "scheduler_events",
    "scheduler_outbox_messages",
    "scheduler_workers",
    "scheduler_teams",
    "scheduler_resource_leases",
    "session_index",
    "session_messages",
    "session_calls",
    "session_steps",
    "session_todos",
    "session_agent_sessions",
];
const GLOBAL_APPLICATION_TABLES: [&str; 23] = [
    "platform",
    "company",
    "projects",
    "scheduler_meta",
    "scheduler_tasks",
    "scheduler_task_dependencies",
    "scheduler_runs",
    "scheduler_task_logs",
    "scheduler_task_artifacts",
    "scheduler_run_workspaces",
    "scheduler_events",
    "scheduler_outbox_messages",
    "scheduler_workers",
    "scheduler_teams",
    "scheduler_resource_leases",
    "session_index",
    "session_messages",
    "session_calls",
    "session_steps",
    "session_todos",
    "session_agent_sessions",
    "workflow_lifecycle_workflows",
    "workflow_workflows",
];

#[test]
fn agent_scheduler_requires_same_scope_parent_project() -> Result<()> {
    let sqlite_home = tempfile::tempdir()?;
    let sqlite_database = GlobalDatabase::for_home(sqlite_home.path())?;
    assert_scheduler_requires_same_scope_parent_project(sqlite_database)?;

    let postgres_fixture = crate::global_database::test_support::postgres_test_database!();
    assert_scheduler_requires_same_scope_parent_project(postgres_fixture.database().clone())?;
    postgres_fixture.cleanup()?;
    Ok(())
}
fn assert_scheduler_requires_same_scope_parent_project(database: GlobalDatabase) -> Result<()> {
    let store = AgentSchedulerStore::from_database(database.clone());
    let project_path = Path::new("scheduler-parent-project-test");
    let project_scope = ProjectScope::new(
        TenantScope::new("platform-a", "company-a")?,
        ProjectId::from("shared-project"),
    )?;
    let context =
        AgentSchedulerContext::new(AgentSchedulerScope::new(project_scope.clone()), project_path)?;

    let missing_error = store
        .initialize_project(&context)
        .expect_err("scheduler schema must require an initialized projects table");
    assert!(missing_error.to_string().contains("projects table"));

    let projects = ProjectStore::new(database)?;
    projects.create(
        &TenantScope::new("platform-a", "company-b")?,
        &isolation_project_info("shared-project", project_path, "wrong-tenant"),
    )?;
    let wrong_scope_error = store
        .initialize_project(&context)
        .expect_err("same project id in another tenant must not satisfy the parent check");
    assert!(wrong_scope_error.to_string().contains("parent project does not exist"));

    projects.create(
        project_scope.tenant_scope(),
        &isolation_project_info("shared-project", project_path, "matching-parent"),
    )?;
    store.initialize_project(&context)
}


fn isolation_session(label: &str) -> ChatSession {
    ChatSession {
        id: "shared-session".to_string(),
        title: label.to_string(),
        messages: vec![ChatMessage {
            role: ChatRole::User,
            content: format!("{label}-message"),
            model: None,
            think_timing: Vec::new(),
            processing_timing: None,
        }],
        message_ids: vec![Some("shared-message".to_string())],
        calls: vec![json!({"scope": label})],
        steps: vec![ChatSessionStep {
            index: 0,
            turn_id: None,
            started_ms: 1,
            finished_ms: Some(2),
            start_snapshot_path: None,
            finish_snapshot_path: None,
            usage: TokenUsage {
                input_tokens: 1,
                output_tokens: 1,
                cached_tokens: 0,
                reasoning_tokens: 0,
            },
            cost_usd: None,
            finish_reason: Some("stop".to_string()),
            model: Some("test-model".to_string()),
            tool_timings: Vec::new(),
        }],
        created_ms: 1,
        updated_ms: 2,
    }
}

fn isolation_agent_session(project_id: &str, label: &str) -> AgentSessionInfo {
    AgentSessionInfo {
        id: "shared-agent-session".to_string(),
        slug: format!("slug-{label}"),
        project_id: project_id.to_string(),
        directory: format!("/work/{label}"),
        parent_id: None,
        summary: None,
        share: None,
        title: format!("{label}-agent"),
        version: "test".to_string(),
        time: AgentSessionTimeInfo { created: 1, updated: 2, compacting: None, archived: None },
        permission: None,
        revert: None,
    }
}



fn append_lifecycle_event(
    store: &AgentSchedulerStore,
    project_path: &Path,
    task_id: &TaskId,
    run_id: &RunId,
    worker_id: &AssigneeId,
    kind: AgentEventKind,
    from_status: AgentTaskStatus,
    to_status: AgentTaskStatus,
    message: &str,
    payload: Value,
    created_at_ms: TimestampMs,
) -> Result<()> {
    store.append_event(
        project_path,
        AppendAgentEventRequest {
            task_id: Some(task_id.clone()),
            run_id: Some(run_id.clone()),
            worker_id: Some(worker_id.clone()),
            team_id: Some(TeamId::new("team-e2e")?),
            kind,
            from_status: Some(from_status),
            to_status: Some(to_status),
            from_status_key: None,
            to_status_key: None,
            message: message.to_string(),
            payload,
            created_at_ms,
        },
    )?;
    Ok(())
}

fn finite_task(task_id: &TaskId) -> Result<NewAgentTask> {
    Ok(NewAgentTask {
        id: task_id.clone(),
        title: "Agent scheduler E2E finite task".to_string(),
        kind: AgentTaskKind::Finite,
        status: AgentTaskStatus::Queued,
        spec_uri: format!("shared/tasks/{}/spec.md", task_id.as_str()),
        meta_uri: format!("shared/tasks/{}/meta.json", task_id.as_str()),
        assigned_to: None,
        team_id: Some(TeamId::new("team-e2e")?),
        room_id: None,
        project_room_id: Some("!project-e2e-room:example.test".to_string()),
        delegated_to_team: None,
        schedule: None,
        timezone: None,
        next_scheduled_at: None,
        last_executed_at: None,
        dag: Vec::new(),
        metadata: json!({
            "required_skills": ["scheduler-e2e"],
            "preferred_runtime": "copaw",
            "requirements": "Complete the scheduler E2E lifecycle.",
            "acceptance_criteria": [
                "dispatch selects the idle worker",
                "worker acknowledgment and completion are persisted",
                "outbox delivery and compatibility export remain readable"
            ],
            "context": "Covers create task -> dispatch -> ack -> complete -> outbox delivered.",
            "prompt": "Complete the scheduler E2E lifecycle.",
            "result": "E2E lifecycle completed."
        }),
        created_at_ms: TimestampMs(1_783_001_000_000),
    })
}


fn assert_run_persisted(
    store: &AgentSchedulerStore,
    run_id: &RunId,
    expected_status: &str,
    finished_at_ms: TimestampMs,
) -> Result<()> {
    let conn = Connection::open(store.database_path())?;
    let (status, finished_at): (String, Option<i64>) = conn.query_row(
        "SELECT status, finished_at_ms FROM scheduler_runs WHERE id = ?1",
        [run_id.as_str()],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )?;
    assert_eq!(status, expected_status);
    assert_eq!(finished_at, Some(finished_at_ms.0));
    Ok(())
}

fn assert_outbox_delivered(
    store: &AgentSchedulerStore,
    message_id: &str,
    delivered_at_ms: TimestampMs,
) -> Result<()> {
    let conn = Connection::open(store.database_path())?;
    let (status, attempts, delivered_at): (String, i64, Option<i64>) = conn.query_row(
        "SELECT status, attempts, delivered_at_ms FROM scheduler_outbox_messages WHERE id = ?1",
        [message_id],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
    )?;
    assert_eq!(status, "delivered");
    assert_eq!(attempts, 1);
    assert_eq!(delivered_at, Some(delivered_at_ms.0));
    Ok(())
}

fn isolation_project_info(id: &str, project_path: &Path, label: &str) -> Info {
    Info {
        id: id.to_string(),
        worktree: project_path.to_string_lossy().into_owned(),
        vcs: None,
        name: Some(label.to_string()),
        icon: None,
        commands: None,
        time: TimeInfo { created: 1, updated: 1, initialized: None },
        sandboxes: Vec::new(),
    }
}

fn isolation_task(label: &str) -> Result<NewAgentTask> {
    Ok(NewAgentTask {
        id: TaskId::new("shared-task")?,
        title: format!("{label}-task"),
        kind: AgentTaskKind::Finite,
        status: AgentTaskStatus::Assigned,
        spec_uri: "shared/tasks/shared-task/spec.md".to_string(),
        meta_uri: "shared/tasks/shared-task/meta.json".to_string(),
        assigned_to: Some(AssigneeId::new("shared-worker")?),
        team_id: Some(TeamId::new("shared-team")?),
        room_id: None,
        project_room_id: None,
        delegated_to_team: None,
        schedule: None,
        timezone: None,
        next_scheduled_at: None,
        last_executed_at: None,
        dag: Vec::new(),
        metadata: json!({"scope": label}),
        created_at_ms: TimestampMs(900),
    })
}



fn assert_project_local_databases_absent(project_path: &Path) {
    let project_config = project_path.join(vw_config_types::paths::HOME_CONFIG_DIR_NAME);
    for domain in ["agent_scheduler", "tasks"] {
        let database = project_config.join(domain).join("_index.sqlite3");
        assert!(!database.exists(), "project-local database was recreated: {}", database.display());
        assert!(!Path::new(&format!("{}-wal", database.display())).exists());
        assert!(!Path::new(&format!("{}-shm", database.display())).exists());
        assert!(!database.with_file_name("_index.lock").exists());
    }
}

#[cfg(unix)]
fn assert_global_database_permissions(database_path: &Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;

    let directory = database_path.parent().expect("global database parent");
    assert_eq!(fs::metadata(directory)?.permissions().mode() & 0o777, 0o700);
    assert_eq!(fs::metadata(database_path)?.permissions().mode() & 0o777, 0o600);
    Ok(())
}

#[cfg(not(unix))]
fn assert_global_database_permissions(_database_path: &Path) -> Result<()> {
    Ok(())
}

#[derive(Debug, Clone, Copy)]
struct TestClock {
    now_ms: TimestampMs,
}

impl TestClock {
    fn at(now_ms: i64) -> Self {
        Self { now_ms: TimestampMs(now_ms) }
    }

    fn advance_ms(&mut self, delta_ms: i64) -> TimestampMs {
        self.now_ms = TimestampMs(self.now_ms.0 + delta_ms);
        self.now_ms
    }
}
