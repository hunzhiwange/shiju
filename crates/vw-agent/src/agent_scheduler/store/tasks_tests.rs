use std::path::Path;

use anyhow::Result;
use serde_json::json;
use vw_api_types::agent_scheduler::AgentSchedulerScope;
use vw_api_types::common::TimestampMs;
use vw_api_types::id::{ProjectId, ProjectScope, TenantScope};
use vw_config_types::staff::{
    StaffMemberTaskAssignmentConfig, StaffProjectStateConfig, StaffStageActionConfig,
    StaffStageActionInputsConfig, StaffStageAssignmentConfig,
};

use super::domain::{AgentSchedulerContext, RunId, TaskId, TeamId, WorkerName};
use super::state_machine::{AgentTaskKind, AgentTaskStatus};
use super::store::AgentSchedulerStore;
use super::store::artifacts::RecordTaskArtifactRequest;
use super::store::logs::{AgentSchedulerWorkspaceContext, AppendAgentTaskLogRequest};
use super::store::runs::ClaimTaskRunRequest;
use super::store::tasks::{AgentTaskBoardUpdate, AgentTaskDagNode, AgentTaskFilter, NewAgentTask};
use crate::global_database::GlobalDatabase;
use crate::global_database::test_support::PostgresTestDatabase;
use crate::project::store::ProjectStore;
use vw_shared::project::{Info as ProjectInfo, TimeInfo as ProjectTimeInfo};

#[test]
fn agent_scheduler_task_graph_isolated_by_full_scope() -> Result<()> {
    let temp_dir = tempfile::tempdir()?;
    let store = AgentSchedulerStore::new();
    let scope_a = scheduler_context(temp_dir.path(), "platform-a", "company-a", "project")?;
    let scope_b = scheduler_context(temp_dir.path(), "platform-b", "company-b", "project")?;
    let task_id = TaskId::new("shared-task")?;
    let run_id = RunId::new("shared-run")?;

    store.initialize_project(temp_dir.path())?;
    store.create_task(&scope_a, finite_task(task_id.as_str())?)?;
    assert!(store.load_task(&scope_b, &task_id)?.is_none());
    assert!(
        store
            .transition_task_status(
                &scope_b,
                &task_id,
                AgentTaskStatus::Dispatched,
                TimestampMs(1_001),
            )?
            .is_none()
    );

    let mut tenant_b_task = finite_task(task_id.as_str())?;
    tenant_b_task.title = "Tenant B task".to_string();
    store.create_task(&scope_b, tenant_b_task)?;
    store.transition_task_status(
        &scope_a,
        &task_id,
        AgentTaskStatus::Dispatched,
        TimestampMs(1_002),
    )?;
    assert_eq!(store.load_task(&scope_b, &task_id)?.expect("tenant B task").title, "Tenant B task");
    assert_eq!(
        store.load_task(&scope_b, &task_id)?.expect("tenant B task").status,
        AgentTaskStatus::Assigned
    );

    assert!(
        store
            .claim_task_run(
                &scope_a,
                ClaimTaskRunRequest {
                    run_id: run_id.clone(),
                    task_id: task_id.clone(),
                    worker_id: WorkerName::new("worker-a")?,
                    lease_acquired_at_ms: TimestampMs(1_010),
                    lease_expires_at_ms: Some(TimestampMs(2_010)),
                },
            )?
            .is_some()
    );
    assert!(store.load_run(&scope_b, &run_id)?.is_none());
    assert!(
        store.heartbeat_run(&scope_b, &run_id, TimestampMs(1_020), TimestampMs(2_020))?.is_none()
    );
    assert!(
        store
            .claim_task_run(
                &scope_b,
                ClaimTaskRunRequest {
                    run_id: run_id.clone(),
                    task_id: task_id.clone(),
                    worker_id: WorkerName::new("worker-b")?,
                    lease_acquired_at_ms: TimestampMs(1_030),
                    lease_expires_at_ms: Some(TimestampMs(2_030)),
                },
            )?
            .is_some()
    );

    store.append_task_log(
        &scope_a,
        AppendAgentTaskLogRequest {
            task_id: task_id.clone(),
            run_id: Some(run_id.clone()),
            timestamp_ms: TimestampMs(1_040),
            from_status: None,
            to_status: None,
            from_status_key: None,
            to_status_key: None,
            message: "tenant A log".to_string(),
        },
    )?;
    store.append_task_log(
        &scope_b,
        AppendAgentTaskLogRequest {
            task_id: task_id.clone(),
            run_id: Some(run_id.clone()),
            timestamp_ms: TimestampMs(1_041),
            from_status: None,
            to_status: None,
            from_status_key: None,
            to_status_key: None,
            message: "tenant B log".to_string(),
        },
    )?;
    assert_eq!(store.list_task_logs(&scope_a, &task_id)?[0].message, "tenant A log");
    assert_eq!(store.list_task_logs(&scope_b, &task_id)?[0].message, "tenant B log");

    store.record_run_workspace_context(
        &scope_a,
        &task_id,
        &run_id,
        AgentSchedulerWorkspaceContext {
            slot_id: "slot-a".to_string(),
            path: "/workspace/a".to_string(),
            branch: "agent/a".to_string(),
        },
        TimestampMs(1_050),
    )?;
    store.record_run_workspace_context(
        &scope_b,
        &task_id,
        &run_id,
        AgentSchedulerWorkspaceContext {
            slot_id: "slot-b".to_string(),
            path: "/workspace/b".to_string(),
            branch: "agent/b".to_string(),
        },
        TimestampMs(1_051),
    )?;
    assert_eq!(
        store
            .load_run_workspace_context(&scope_a, &run_id)?
            .expect("tenant A workspace")
            .workspace
            .slot_id,
        "slot-a"
    );
    assert_eq!(
        store
            .load_run_workspace_context(&scope_b, &run_id)?
            .expect("tenant B workspace")
            .workspace
            .slot_id,
        "slot-b"
    );

    store.record_task_artifact(
        &scope_a,
        RecordTaskArtifactRequest {
            task_id: task_id.clone(),
            run_id: Some(run_id.clone()),
            kind: "report".to_string(),
            name: "tenant-a".to_string(),
            path: None,
            payload: json!({}),
            created_at_ms: TimestampMs(1_060),
        },
    )?;
    store.record_task_artifact(
        &scope_b,
        RecordTaskArtifactRequest {
            task_id: task_id.clone(),
            run_id: Some(run_id),
            kind: "report".to_string(),
            name: "tenant-b".to_string(),
            path: None,
            payload: json!({}),
            created_at_ms: TimestampMs(1_061),
        },
    )?;
    assert_eq!(store.list_task_artifacts(&scope_a, &task_id)?[0].name, "tenant-a");
    assert_eq!(store.list_task_artifacts(&scope_b, &task_id)?[0].name, "tenant-b");

    Ok(())
}

#[test]
fn agent_task_store_tracks_active_finite_tasks_without_state_json() -> Result<()> {
    for_each_task_store_provider(assert_task_store_tracks_active_finite_tasks)
}

fn assert_task_store_tracks_active_finite_tasks(
    store: &AgentSchedulerStore,
    context: &AgentSchedulerContext,
) -> Result<()> {
    let project_path = context.project_path();

    store.initialize_project(context)?;
    assert!(!state_json_path(project_path).exists());

    let created = store.create_task(context, finite_task("task-finite-1")?)?;
    assert_eq!(created.id.as_str(), "task-finite-1");
    assert_eq!(created.status, AgentTaskStatus::Assigned);
    assert_eq!(created.assigned_to.as_ref().map(WorkerName::as_str), Some("worker-a"));
    assert_eq!(created.spec_uri, "shared/tasks/task-finite-1/spec.md");
    assert_eq!(created.meta_uri, "shared/tasks/task-finite-1/meta.json");

    let loaded =
        store.load_task(context, &TaskId::new("task-finite-1")?)?.expect("created task loads");
    assert_eq!(loaded, created);

    let duplicate = store.create_task(context, finite_task("task-finite-1")?)?;
    assert_eq!(duplicate, created);

    let active = store.list_active_tasks(context, AgentTaskFilter::default())?;
    assert_eq!(active.len(), 1);

    let by_status =
        store.list_active_tasks(context, AgentTaskFilter::for_status(AgentTaskStatus::Assigned))?;
    assert_eq!(by_status, vec![created.clone()]);

    let by_worker = store
        .list_active_tasks(context, AgentTaskFilter::for_worker(WorkerName::new("worker-a")?))?;
    assert_eq!(by_worker, vec![created.clone()]);

    let by_task = store
        .list_active_tasks(context, AgentTaskFilter::for_task(TaskId::new("task-finite-1")?))?;
    assert_eq!(by_task, vec![created.clone()]);

    let dispatched = store
        .transition_task_status(
            context,
            &TaskId::new("task-finite-1")?,
            AgentTaskStatus::Dispatched,
            TimestampMs(1_783_000_200_000),
        )?
        .expect("task still exists");
    assert_eq!(dispatched.status, AgentTaskStatus::Dispatched);

    for status in [
        AgentTaskStatus::Acknowledged,
        AgentTaskStatus::Running,
        AgentTaskStatus::Submitted,
        AgentTaskStatus::Completed,
    ] {
        store.transition_task_status(
            context,
            &TaskId::new("task-finite-1")?,
            status,
            TimestampMs(1_783_000_200_001),
        )?;
    }

    let active_after_completion = store
        .list_active_tasks(context, AgentTaskFilter::for_task(TaskId::new("task-finite-1")?))?;
    assert!(active_after_completion.is_empty());
    assert!(!state_json_path(project_path).exists());

    Ok(())
}

#[test]
fn agent_task_store_rejects_invalid_status_transition() -> Result<()> {
    for_each_task_store_provider(assert_task_store_rejects_invalid_status_transition)
}

#[test]
fn agent_task_store_rejects_empty_required_fields() -> Result<()> {
    let temp_dir = tempfile::tempdir()?;
    let project_path = temp_dir.path();
    let store = AgentSchedulerStore::new();

    for (field, expected) in [
        ("title", "task title must not be empty"),
        ("spec_uri", "task spec uri must not be empty"),
        ("meta_uri", "task meta uri must not be empty"),
    ] {
        let mut task = finite_task(&format!("task-invalid-{field}"))?;
        match field {
            "title" => task.title = " \t".to_string(),
            "spec_uri" => task.spec_uri = String::new(),
            "meta_uri" => task.meta_uri = "\n".to_string(),
            _ => unreachable!("all required fields are enumerated"),
        }
        let error = store
            .create_task(project_path, task)
            .expect_err("empty required task field must be rejected");
        assert!(error.to_string().contains(expected), "unexpected error: {error:#}");
    }

    Ok(())
}

#[test]
fn agent_task_store_updates_recurring_execution_and_lists_the_task() -> Result<()> {
    let temp_dir = tempfile::tempdir()?;
    let project_path = temp_dir.path();
    let store = AgentSchedulerStore::new();
    let task_id = TaskId::new("task-infinite-1")?;

    store.create_task(project_path, infinite_task()?)?;
    let updated = store
        .update_recurring_task_execution(
            project_path,
            &task_id,
            "2026-07-17T08:00:00Z",
            "2026-07-17T08:15:00Z",
            TimestampMs(1_783_000_200_000),
        )?
        .expect("infinite task accepts recurring execution timestamps");
    assert_eq!(updated.last_executed_at.as_deref(), Some("2026-07-17T08:00:00Z"));
    assert_eq!(updated.next_scheduled_at.as_deref(), Some("2026-07-17T08:15:00Z"));
    assert_eq!(updated.updated_at_ms, TimestampMs(1_783_000_200_000));

    let listed = store.list_tasks(project_path, AgentTaskFilter::for_task(task_id))?;
    assert_eq!(listed, vec![updated]);

    Ok(())
}

#[test]
fn agent_task_store_returns_none_for_missing_task_mutations_and_empty_meta() -> Result<()> {
    let temp_dir = tempfile::tempdir()?;
    let project_path = temp_dir.path();
    let store = AgentSchedulerStore::new();
    let missing = TaskId::new("task-missing")?;

    store.initialize_project(project_path)?;

    assert!(store.load_task(project_path, &missing)?.is_none());
    assert!(
        store
            .update_task_board_task(
                project_path,
                &missing,
                AgentTaskBoardUpdate {
                    title: Some("Ignored".to_string()),
                    status: Some(AgentTaskStatus::Running),
                    metadata: json!({"task_board.status_key": "doing"}),
                    updated_at_ms: TimestampMs(1_783_000_200_010),
                },
            )?
            .is_none()
    );
    assert!(
        store
            .transition_task_status(
                project_path,
                &missing,
                AgentTaskStatus::Running,
                TimestampMs(1_783_000_200_011),
            )?
            .is_none()
    );
    assert!(
        store
            .transition_task_status_with_metadata(
                project_path,
                &missing,
                AgentTaskStatus::Running,
                json!({}),
                TimestampMs(1_783_000_200_012),
            )?
            .is_none()
    );
    assert!(
        store
            .update_recurring_task_execution(
                project_path,
                &missing,
                "2026-07-17T08:00:00Z",
                "2026-07-17T08:15:00Z",
                TimestampMs(1_783_000_200_013),
            )?
            .is_none()
    );
    assert!(store.load_task_board_workflow(project_path)?.is_none());
    assert!(store.load_task_board_staff_project_state(project_path)?.is_none());

    Ok(())
}

#[test]
fn agent_task_store_rejects_recurring_update_for_finite_task() -> Result<()> {
    let temp_dir = tempfile::tempdir()?;
    let project_path = temp_dir.path();
    let store = AgentSchedulerStore::new();
    let task_id = TaskId::new("task-finite-recurring")?;

    store.initialize_project(project_path)?;
    store.create_task(project_path, finite_task("task-finite-recurring")?)?;

    let error = store
        .update_recurring_task_execution(
            project_path,
            &task_id,
            "2026-07-17T08:00:00Z",
            "2026-07-17T08:15:00Z",
            TimestampMs(1_783_000_200_014),
        )
        .expect_err("finite tasks cannot record recurring execution");

    assert!(error.to_string().contains("can only be recorded for infinite task"));
    assert!(error.to_string().contains("task-finite-recurring"));
    Ok(())
}

#[test]
fn agent_task_store_rejects_empty_task_board_update_title() -> Result<()> {
    let temp_dir = tempfile::tempdir()?;
    let project_path = temp_dir.path();
    let store = AgentSchedulerStore::new();
    let task_id = TaskId::new("task-board-empty-title")?;

    store.initialize_project(project_path)?;
    store.create_task(
        project_path,
        board_task(
            "task-board-empty-title",
            "backlog",
            3,
            1,
            false,
            TimestampMs(1_783_000_100_000),
        )?,
    )?;

    let error = store
        .update_task_board_task(
            project_path,
            &task_id,
            AgentTaskBoardUpdate {
                title: Some("  ".to_string()),
                status: None,
                metadata: json!({"task_board.status_key": "backlog"}),
                updated_at_ms: TimestampMs(1_783_000_101_000),
            },
        )
        .expect_err("blank board titles are rejected");

    assert!(error.to_string().contains("task title must not be empty"));
    Ok(())
}

#[test]
fn agent_task_store_list_filters_apply_to_task_board_and_all_tasks() -> Result<()> {
    let temp_dir = tempfile::tempdir()?;
    let project_path = temp_dir.path();
    let store = AgentSchedulerStore::new();

    store.initialize_project(project_path)?;
    let mut assigned =
        board_task("task-filter-assigned", "backlog", 1, 0, false, TimestampMs(1_783_000_100_000))?;
    assigned.assigned_to = Some(WorkerName::new("worker-filter")?);
    store.create_task(project_path, assigned)?;
    store.create_task(
        project_path,
        board_task("task-filter-other", "doing", 2, 0, false, TimestampMs(1_783_000_100_001))?,
    )?;

    let mut filter = AgentTaskFilter::for_worker(WorkerName::new("worker-filter")?);
    filter.status = Some(AgentTaskStatus::Queued);
    filter.task_id = Some(TaskId::new("task-filter-assigned")?);
    let board = store.list_task_board_tasks(project_path, filter.clone())?;
    assert_eq!(
        board.iter().map(|task| task.id.as_str()).collect::<Vec<_>>(),
        vec!["task-filter-assigned"]
    );

    let all = store.list_tasks(project_path, filter)?;
    assert_eq!(
        all.iter().map(|task| task.id.as_str()).collect::<Vec<_>>(),
        vec!["task-filter-assigned"]
    );

    Ok(())
}

#[test]
fn agent_task_store_reports_invalid_task_board_meta_json() -> Result<()> {
    let temp_dir = tempfile::tempdir()?;
    let project_path = temp_dir.path();
    let store = AgentSchedulerStore::new();
    let context = scheduler_context(project_path, "platform-meta", "company-meta", "project-meta")?;

    store.initialize_project(&context)?;
    let connection = rusqlite::Connection::open(store.database_path())?;
    connection.execute(
        "INSERT INTO scheduler_meta (platform_id, company_id, project_id, key, value)
         VALUES (?1, ?2, ?3, 'task_board.workflow', '{bad json')",
        ["platform-meta", "company-meta", "project-meta"],
    )?;

    let error = store.load_task_board_workflow(&context).expect_err("invalid workflow json fails");
    assert!(error.to_string().contains("parse task board workflow"));

    connection.execute(
        "INSERT INTO scheduler_meta (platform_id, company_id, project_id, key, value)
         VALUES (?1, ?2, ?3, 'task_board.staff_project_state', '{bad json')",
        ["platform-meta", "company-meta", "project-meta"],
    )?;
    let error = store
        .load_task_board_staff_project_state(&context)
        .expect_err("invalid staff project state json fails");
    assert!(error.to_string().contains("parse staff project state"));

    Ok(())
}

#[test]
fn agent_task_store_dag_parser_rejects_invalid_domain_values() -> Result<()> {
    for (raw, expected) in [
        ("not json", "expected ident"),
        (r#"[{"task_id":"","title":"Plan","status":"queued"}]"#, "task id must not be empty"),
        (
            r#"[{"task_id":"node","title":"Plan","depends_on":[""],"status":"queued"}]"#,
            "task id must not be empty",
        ),
        (
            r#"[{"task_id":"node","title":"Plan","assigned_to":"","status":"queued"}]"#,
            "worker name must not be empty",
        ),
        (
            r#"[{"task_id":"node","title":"Plan","status":"not-a-status"}]"#,
            "unsupported agent task status",
        ),
    ] {
        let error = super::store::tasks::parse_dag_value(raw)
            .expect_err("invalid DAG values must fail")
            .to_string();
        assert!(error.contains(expected), "expected {expected:?} in {error:?}");
    }

    Ok(())
}

#[test]
fn agent_task_store_task_board_projection_handles_non_objects_and_fallback_roles() -> Result<()> {
    assert_eq!(
        super::store::tasks::task_board_projection_metadata(json!(null), AgentTaskStatus::Draft),
        json!(null)
    );

    for (status, expected_role, expected_key) in [
        (AgentTaskStatus::Draft, "backlog", "draft"),
        (AgentTaskStatus::Revision, "review", "revision"),
        (AgentTaskStatus::Blocked, "paused", "blocked"),
        (AgentTaskStatus::Paused, "paused", "paused"),
        (AgentTaskStatus::Completed, "done", "completed"),
        (AgentTaskStatus::Failed, "failed", "failed"),
        (AgentTaskStatus::Cancelled, "archived", "cancelled"),
        (AgentTaskStatus::Archived, "archived", "archived"),
    ] {
        let metadata = super::store::tasks::task_board_projection_metadata(json!({}), status);
        assert_eq!(metadata["task_board.status_role"], expected_role);
        assert_eq!(metadata["task_board.status_key"], expected_key);
    }

    Ok(())
}

fn assert_task_store_rejects_invalid_status_transition(
    store: &AgentSchedulerStore,
    context: &AgentSchedulerContext,
) -> Result<()> {
    store.initialize_project(context)?;
    store.create_task(context, finite_task("task-finite-2")?)?;

    let error = store
        .transition_task_status(
            context,
            &TaskId::new("task-finite-2")?,
            AgentTaskStatus::Completed,
            TimestampMs(1_783_000_200_000),
        )
        .expect_err("assigned finite task cannot complete directly");
    assert!(error.to_string().contains("assigned -> completed"));

    Ok(())
}

#[test]
fn agent_task_store_transition_updates_task_board_projection_from_status_map() -> Result<()> {
    let temp_dir = tempfile::tempdir()?;
    let project_path = temp_dir.path();
    let store = AgentSchedulerStore::new();
    let task_id = TaskId::new("task-board-projection")?;

    store.initialize_project(project_path)?;
    let mut task = board_task(
        "task-board-projection",
        "pending",
        1,
        0,
        false,
        TimestampMs(1_783_000_100_000),
    )?;
    task.metadata = json!({
        "task_board.status_key": "pending",
        "task_board.status_role": "queued",
        "task_board.status_key_map": {
            "queued": "pending",
            "assigned": "pending",
            "dispatched": "pending",
            "acknowledged": "pending",
            "running": "running",
            "submitted": "code_complete"
        }
    });
    store.create_task(project_path, task)?;

    for status in
        [AgentTaskStatus::Assigned, AgentTaskStatus::Dispatched, AgentTaskStatus::Acknowledged]
    {
        store.transition_task_status(
            project_path,
            &task_id,
            status,
            TimestampMs(1_783_000_100_100),
        )?;
    }
    let acknowledged = store.load_task(project_path, &task_id)?.expect("acknowledged task");
    assert_eq!(acknowledged.metadata["task_board.status_key"], "pending");
    assert_eq!(acknowledged.metadata["task_board.status_role"], "queued");

    let running = store
        .transition_task_status(
            project_path,
            &task_id,
            AgentTaskStatus::Running,
            TimestampMs(1_783_000_100_200),
        )?
        .expect("running task");
    assert_eq!(running.metadata["task_board.status_key"], "running");
    assert_eq!(running.metadata["task_board.status_role"], "executing");

    let submitted = store
        .transition_task_status(
            project_path,
            &task_id,
            AgentTaskStatus::Submitted,
            TimestampMs(1_783_000_100_300),
        )?
        .expect("submitted task");
    assert_eq!(submitted.metadata["task_board.status_key"], "code_complete");
    assert_eq!(submitted.metadata["task_board.status_role"], "review_ready");

    Ok(())
}

#[test]
fn agent_task_store_transition_with_metadata_preserves_lifecycle_projection() -> Result<()> {
    let temp_dir = tempfile::tempdir()?;
    let project_path = temp_dir.path();
    let store = AgentSchedulerStore::new();
    let task_id = TaskId::new("task-board-lifecycle-projection")?;

    store.initialize_project(project_path)?;
    let mut task = finite_task("task-board-lifecycle-projection")?;
    task.status = AgentTaskStatus::Running;
    task.assigned_to = None;
    task.metadata = json!({
        "task_board.status_key": "custom_running",
        "task_board.status_role": "executing"
    });
    store.create_task(project_path, task)?;

    let updated = store
        .transition_task_status_with_metadata(
            project_path,
            &task_id,
            AgentTaskStatus::Submitted,
            json!({
                "task_board.status_key": "custom_review_ready",
                "task_board.status_role": "review_ready"
            }),
            TimestampMs(1_783_000_100_400),
        )?
        .expect("submitted task");

    assert_eq!(updated.status, AgentTaskStatus::Submitted);
    assert_eq!(updated.metadata["task_board.status_key"], "custom_review_ready");
    assert_eq!(updated.metadata["task_board.status_role"], "review_ready");
    Ok(())
}

#[test]
fn agent_scheduler_store_task_board_runtime_state_keeps_member_assignments_only() -> Result<()> {
    let temp_dir = tempfile::tempdir()?;
    let project_path = temp_dir.path();
    let store = AgentSchedulerStore::new();
    let project_state = StaffProjectStateConfig {
        stage_assignments: vec![StaffStageAssignmentConfig {
            stage_id: "planning".to_string(),
            primary_member_id: Some("staff-planner".to_string()),
            backup_member_ids: Vec::new(),
            manager_member_ids: vec!["staff-orchestrator".to_string()],
            autonomous: true,
        }],
        stage_actions: vec![StaffStageActionConfig {
            stage_id: "planning".to_string(),
            capability_id: "execute_task".to_string(),
            workflow_uuid: None,
            inputs: StaffStageActionInputsConfig::default(),
        }],
        member_task_assignments: vec![StaffMemberTaskAssignmentConfig {
            member_id: "staff-executor".to_string(),
            task_ids: vec!["task-runtime-owned".to_string()],
        }],
        ..StaffProjectStateConfig::default()
    };

    store.save_task_board_staff_project_state(project_path, &project_state)?;

    let loaded =
        store.load_task_board_staff_project_state(project_path)?.expect("staff project state");
    assert!(loaded.capabilities.is_empty());
    assert!(loaded.stage_assignments.is_empty());
    assert!(loaded.stage_workflow_bindings.is_empty());
    assert!(loaded.stage_actions.is_empty());
    assert_eq!(loaded.member_task_assignments, project_state.member_task_assignments);

    let raw_project_state = raw_staff_project_state_json(&store)?;
    assert!(raw_project_state.contains("member_task_assignments"));
    assert!(!raw_project_state.contains("capabilities"));
    assert!(!raw_project_state.contains("stage_assignments"));
    assert!(!raw_project_state.contains("stage_workflow_bindings"));
    assert!(!raw_project_state.contains("stage_actions"));

    Ok(())
}

#[test]
fn agent_task_store_preserves_infinite_schedule_and_team_dag_fields() -> Result<()> {
    let temp_dir = tempfile::tempdir()?;
    let project_path = temp_dir.path();
    let store = AgentSchedulerStore::new();

    store.initialize_project(project_path)?;

    let infinite = store.create_task(project_path, infinite_task()?)?;
    assert_eq!(infinite.kind, AgentTaskKind::Infinite);
    assert_eq!(infinite.schedule.as_deref(), Some("*/15 * * * *"));
    assert_eq!(infinite.timezone.as_deref(), Some("Asia/Shanghai"));
    assert_eq!(infinite.next_scheduled_at.as_deref(), Some("2026-07-02T10:15:00Z"));

    let team_dag = store.create_task(project_path, team_dag_task()?)?;
    assert_eq!(team_dag.kind, AgentTaskKind::TeamDag);
    assert_eq!(team_dag.delegated_to_team.as_ref().map(TeamId::as_str), Some("team-alpha"));
    assert_eq!(team_dag.dag.len(), 2);
    assert_eq!(team_dag.dag[1].depends_on[0].as_str(), "node-plan");
    assert_eq!(team_dag.dag[1].assigned_to.as_ref().map(WorkerName::as_str), Some("worker-ui"));

    Ok(())
}

#[test]
fn agent_scheduler_store_sorts_task_board_snapshot_and_filters_soft_deleted() -> Result<()> {
    let temp_dir = tempfile::tempdir()?;
    let project_path = temp_dir.path();
    let store = AgentSchedulerStore::new();

    store.initialize_project(project_path)?;
    for task in [
        board_task("task-doing", "doing", 2, 0, false, TimestampMs(1_783_000_100_300))?,
        board_task("task-backlog-later", "backlog", 2, 2, false, TimestampMs(1_783_000_100_200))?,
        board_task("task-backlog-first", "backlog", 1, 5, false, TimestampMs(1_783_000_100_100))?,
        board_task("task-deleted", "backlog", 0, 0, true, TimestampMs(1_783_000_100_000))?,
    ] {
        store.create_task(project_path, task)?;
    }

    let tasks = store.list_task_board_tasks(project_path, AgentTaskFilter::default())?;
    assert_eq!(
        tasks.iter().map(|task| task.id.as_str()).collect::<Vec<_>>(),
        vec!["task-backlog-first", "task-backlog-later", "task-doing"]
    );

    Ok(())
}

#[test]
fn agent_scheduler_store_updates_task_board_fields_and_workflow_snapshot() -> Result<()> {
    let temp_dir = tempfile::tempdir()?;
    let project_path = temp_dir.path();
    let store = AgentSchedulerStore::new();
    let task_id = TaskId::new("task-board-update")?;

    store.initialize_project(project_path)?;
    store.create_task(
        project_path,
        board_task("task-board-update", "backlog", 3, 1, false, TimestampMs(1_783_000_100_000))?,
    )?;

    let updated = store
        .update_task_board_task(
            project_path,
            &task_id,
            super::store::tasks::AgentTaskBoardUpdate {
                title: Some("Updated board task".to_string()),
                status: Some(AgentTaskStatus::Running),
                metadata: json!({
                    "task_board.status_key": "doing",
                    "priority": 1,
                    "order_no": 0
                }),
                updated_at_ms: TimestampMs(1_783_000_101_000),
            },
        )?
        .expect("updated task");
    assert_eq!(updated.title, "Updated board task");
    assert_eq!(updated.status, AgentTaskStatus::Running);
    assert_eq!(updated.metadata["task_board.status_key"], "doing");
    assert_eq!(updated.updated_at_ms, TimestampMs(1_783_000_101_000));

    let workflow = vw_api_types::task::TaskPoolWorkflowSnapshotDto {
        id: "custom_flow".to_string(),
        label: "Custom Flow".to_string(),
        statuses: vec![vw_api_types::task::TaskPoolWorkflowStatusDto {
            key: "doing".to_string(),
            label: "Doing".to_string(),
            role: vw_api_types::task::TaskPoolWorkflowStatusRoleDto::Executing,
            enabled: true,
            terminal: false,
            next: None,
            transitions: Vec::new(),
        }],
    };
    store.save_task_board_workflow(project_path, &workflow)?;
    assert_eq!(store.load_task_board_workflow(project_path)?, Some(workflow));

    Ok(())
}

#[test]
fn agent_scheduler_store_repairs_missing_failed_retry_transition() -> Result<()> {
    let temp_dir = tempfile::tempdir()?;
    let project_path = temp_dir.path();
    let store = AgentSchedulerStore::new();
    store.initialize_project(project_path)?;
    let workflow = vw_api_types::task::TaskPoolWorkflowSnapshotDto {
        id: "dev_lifecycle".to_string(),
        label: "Development Lifecycle".to_string(),
        statuses: vec![
            vw_api_types::task::TaskPoolWorkflowStatusDto {
                key: "failed".to_string(),
                label: "Failed".to_string(),
                role: vw_api_types::task::TaskPoolWorkflowStatusRoleDto::Failed,
                enabled: true,
                terminal: false,
                next: Some("pending".to_string()),
                transitions: Vec::new(),
            },
            vw_api_types::task::TaskPoolWorkflowStatusDto {
                key: "pending".to_string(),
                label: "Pending".to_string(),
                role: vw_api_types::task::TaskPoolWorkflowStatusRoleDto::Queued,
                enabled: true,
                terminal: false,
                next: None,
                transitions: Vec::new(),
            },
        ],
    };

    store.save_task_board_workflow(project_path, &workflow)?;
    let connection = rusqlite::Connection::open(store.database_path())?;
    connection.execute(
        "UPDATE scheduler_meta SET value = ?1
         WHERE key = 'task_board.workflow'",
        [serde_json::to_string(&workflow)?],
    )?;
    drop(connection);

    let loaded =
        store.load_task_board_workflow(project_path)?.expect("stored workflow should load");
    let failed = loaded
        .statuses
        .iter()
        .find(|status| status.key == "failed")
        .expect("failed status should load");
    assert_eq!(failed.transitions.len(), 1);
    assert_eq!(failed.transitions[0].outcome, "retry");
    assert_eq!(failed.transitions[0].target, "pending");

    Ok(())
}

#[test]
fn agent_scheduler_store_keeps_existing_retry_and_non_failed_workflow_statuses() -> Result<()> {
    let temp_dir = tempfile::tempdir()?;
    let project_path = temp_dir.path();
    let store = AgentSchedulerStore::new();
    store.initialize_project(project_path)?;
    let workflow = vw_api_types::task::TaskPoolWorkflowSnapshotDto {
        id: "no_repair_needed".to_string(),
        label: "No repair needed".to_string(),
        statuses: vec![
            vw_api_types::task::TaskPoolWorkflowStatusDto {
                key: "failed-with-retry".to_string(),
                label: "Failed with retry".to_string(),
                role: vw_api_types::task::TaskPoolWorkflowStatusRoleDto::Failed,
                enabled: true,
                terminal: false,
                next: Some("pending".to_string()),
                transitions: vec![vw_api_types::task::TaskPoolWorkflowTransitionDto {
                    outcome: " Retry ".to_string(),
                    label: Some("Try again".to_string()),
                    target: "pending".to_string(),
                }],
            },
            vw_api_types::task::TaskPoolWorkflowStatusDto {
                key: "failed-terminal".to_string(),
                label: "Failed terminal".to_string(),
                role: vw_api_types::task::TaskPoolWorkflowStatusRoleDto::Failed,
                enabled: true,
                terminal: true,
                next: None,
                transitions: Vec::new(),
            },
            vw_api_types::task::TaskPoolWorkflowStatusDto {
                key: "queued".to_string(),
                label: "Queued".to_string(),
                role: vw_api_types::task::TaskPoolWorkflowStatusRoleDto::Queued,
                enabled: true,
                terminal: false,
                next: Some("running".to_string()),
                transitions: Vec::new(),
            },
        ],
    };

    store.save_task_board_workflow(project_path, &workflow)?;
    let loaded =
        store.load_task_board_workflow(project_path)?.expect("stored workflow should load");

    assert_eq!(loaded.statuses[0].transitions.len(), 1);
    assert_eq!(loaded.statuses[0].transitions[0].outcome, " Retry ");
    assert!(loaded.statuses[1].transitions.is_empty());
    assert!(loaded.statuses[2].transitions.is_empty());
    Ok(())
}

fn finite_task(id: &str) -> Result<NewAgentTask> {
    Ok(NewAgentTask {
        id: TaskId::new(id)?,
        title: "Implement focused scheduler storage".to_string(),
        kind: AgentTaskKind::Finite,
        status: AgentTaskStatus::Assigned,
        spec_uri: format!("shared/tasks/{id}/spec.md"),
        meta_uri: format!("shared/tasks/{id}/meta.json"),
        assigned_to: Some(WorkerName::new("worker-a")?),
        team_id: None,
        room_id: Some("!worker-room:example.test".to_string()),
        project_room_id: Some("!project-room:example.test".to_string()),
        delegated_to_team: None,
        schedule: None,
        timezone: None,
        next_scheduled_at: None,
        last_executed_at: None,
        dag: Vec::new(),
        metadata: json!({"source": "test"}),
        created_at_ms: TimestampMs(1_783_000_100_000),
    })
}

fn board_task(
    id: &str,
    status_key: &str,
    priority: u32,
    order_no: i64,
    deleted: bool,
    created_at_ms: TimestampMs,
) -> Result<NewAgentTask> {
    Ok(NewAgentTask {
        id: TaskId::new(id)?,
        title: format!("Board task {id}"),
        kind: AgentTaskKind::Finite,
        status: AgentTaskStatus::Queued,
        spec_uri: format!("shared/tasks/{id}/spec.md"),
        meta_uri: format!("shared/tasks/{id}/meta.json"),
        assigned_to: None,
        team_id: None,
        room_id: None,
        project_room_id: None,
        delegated_to_team: None,
        schedule: None,
        timezone: None,
        next_scheduled_at: None,
        last_executed_at: None,
        dag: Vec::new(),
        metadata: json!({
            "task_board.status_key": status_key,
            "task_board.deleted": deleted,
            "priority": priority,
            "order_no": order_no
        }),
        created_at_ms,
    })
}

fn infinite_task() -> Result<NewAgentTask> {
    Ok(NewAgentTask {
        id: TaskId::new("task-infinite-1")?,
        title: "Monitor project queue".to_string(),
        kind: AgentTaskKind::Infinite,
        status: AgentTaskStatus::Active,
        spec_uri: "shared/tasks/task-infinite-1/spec.md".to_string(),
        meta_uri: "shared/tasks/task-infinite-1/meta.json".to_string(),
        assigned_to: Some(WorkerName::new("worker-monitor")?),
        team_id: None,
        room_id: Some("!monitor-room:example.test".to_string()),
        project_room_id: None,
        delegated_to_team: None,
        schedule: Some("*/15 * * * *".to_string()),
        timezone: Some("Asia/Shanghai".to_string()),
        next_scheduled_at: Some("2026-07-02T10:15:00Z".to_string()),
        last_executed_at: None,
        dag: Vec::new(),
        metadata: json!({"source": "test"}),
        created_at_ms: TimestampMs(1_783_000_100_000),
    })
}

fn team_dag_task() -> Result<NewAgentTask> {
    Ok(NewAgentTask {
        id: TaskId::new("task-team-1")?,
        title: "Ship team feature".to_string(),
        kind: AgentTaskKind::TeamDag,
        status: AgentTaskStatus::Queued,
        spec_uri: "shared/tasks/task-team-1/spec.md".to_string(),
        meta_uri: "shared/tasks/task-team-1/meta.json".to_string(),
        assigned_to: Some(WorkerName::new("leader-a")?),
        team_id: Some(TeamId::new("team-alpha")?),
        room_id: Some("!leader-room:example.test".to_string()),
        project_room_id: Some("!project-room:example.test".to_string()),
        delegated_to_team: Some(TeamId::new("team-alpha")?),
        schedule: None,
        timezone: None,
        next_scheduled_at: None,
        last_executed_at: None,
        dag: vec![
            AgentTaskDagNode {
                task_id: TaskId::new("node-plan")?,
                title: "Plan work".to_string(),
                depends_on: Vec::new(),
                assigned_to: Some(WorkerName::new("leader-a")?),
                status: AgentTaskStatus::Queued,
            },
            AgentTaskDagNode {
                task_id: TaskId::new("node-ui")?,
                title: "Build UI".to_string(),
                depends_on: vec![TaskId::new("node-plan")?],
                assigned_to: Some(WorkerName::new("worker-ui")?),
                status: AgentTaskStatus::Queued,
            },
        ],
        metadata: json!({"source": "test"}),
        created_at_ms: TimestampMs(1_783_000_100_000),
    })
}

fn raw_staff_project_state_json(store: &AgentSchedulerStore) -> Result<String> {
    let conn = rusqlite::Connection::open(store.database_path())?;
    Ok(conn.query_row(
        "SELECT value FROM scheduler_meta
         WHERE key = ?1",
        ["task_board.staff_project_state"],
        |row| row.get(0),
    )?)
}

fn scheduler_context(
    project_path: &Path,
    platform_id: &str,
    company_id: &str,
    project_id: &str,
) -> Result<AgentSchedulerContext> {
    let tenant = TenantScope::new(platform_id, company_id)?;
    let project = ProjectScope::new(tenant, ProjectId::from(project_id))?;
    AgentSchedulerContext::new(AgentSchedulerScope::new(project), project_path)
}

fn state_json_path(project_path: &Path) -> std::path::PathBuf {
    project_path.join("state.json")
}

fn for_each_task_store_provider(
    mut check: impl FnMut(&AgentSchedulerStore, &AgentSchedulerContext) -> Result<()>,
) -> Result<()> {
    let sqlite_home = tempfile::tempdir()?;
    let sqlite_project = sqlite_home.path().join("project");
    let sqlite_database = GlobalDatabase::for_home(sqlite_home.path())?;
    let (sqlite_store, sqlite_context) = scheduler_store_fixture(sqlite_database, &sqlite_project)?;
    check(&sqlite_store, &sqlite_context)?;

    let postgres_fixture = crate::global_database::test_support::postgres_test_database!();
    let postgres_project = tempfile::tempdir()?;
    let (postgres_store, postgres_context) =
        scheduler_store_fixture(postgres_fixture.database().clone(), postgres_project.path())?;
    check(&postgres_store, &postgres_context)?;
    postgres_fixture.cleanup()
}

fn scheduler_store_fixture(
    database: GlobalDatabase,
    project_path: &Path,
) -> Result<(AgentSchedulerStore, AgentSchedulerContext)> {
    let tenant = TenantScope::default();
    ProjectStore::new(database.clone())?.create(
        &tenant,
        &ProjectInfo {
            id: "task-store-project".to_owned(),
            worktree: project_path.to_string_lossy().into_owned(),
            vcs: None,
            name: Some("Task store project".to_owned()),
            icon: None,
            commands: None,
            time: ProjectTimeInfo { created: 0, updated: 0, initialized: None },
            sandboxes: Vec::new(),
        },
    )?;
    let context = scheduler_context(
        project_path,
        tenant.platform_id().as_ref(),
        tenant.company_id().as_ref(),
        "task-store-project",
    )?;
    Ok((AgentSchedulerStore::from_database(database), context))
}
