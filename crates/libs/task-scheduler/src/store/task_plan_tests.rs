use anyhow::Result;
use serde_json::json;
use vw_api_agent_scheduler::AgentSchedulerScope;
use vw_api_transport::common::TimestampMs;
use vw_api_transport::id::{ProjectId, ProjectScope, TenantScope};

use agent_database::GlobalDatabase;
use agent_database::config::DatabaseProvider;
use agent_project::store::ProjectStore;
use project_types::{Info, TimeInfo};

use super::AgentSchedulerStore;
use super::task_plan::{TaskPlanEntry, TaskPlanPlacement};
use super::tasks::AgentTaskBoardUpdate;
use crate::domain::{AgentSchedulerContext, TaskId};
use crate::runs_tests::{claim_request, finite_task_with_status, scheduler_store_fixture};
use crate::state_machine::{AgentTaskKind, AgentTaskStatus};

#[test]
fn task_plan_store_round_trip_and_optional_root() -> Result<()> {
    for_each_provider(|store, context, _| {
        let root = TaskId::new("root")?;
        let child = TaskId::new("child")?;
        let ordinary = TaskId::new("ordinary")?;
        let task = finite_task_with_status(&root, AgentTaskStatus::Queued)?;
        store.create_task_with_plan(context, task.clone(), root_batch(2, 12))?;
        store.create_task_with_plan(context, task, root_batch(2, 12))?;
        store.create_task(context, finite_task_with_status(&ordinary, AgentTaskStatus::Queued)?)?;
        store.create_task_with_plan(
            context,
            finite_task_with_status(&child, AgentTaskStatus::Queued)?,
            child_batch(&root, 1),
        )?;
        edit_placement(store, context, &child, Some(child_batch(&root, 10)))?;
        assert_eq!(
            store.list_task_plan_children(context, &root)?,
            vec![TaskPlanEntry { task_id: child.clone(), placement: child_batch(&root, 10) }],
        );
        assert_eq!(store.list_task_plan(context)?.len(), 2);

        edit_placement(store, context, &root, None)?;
        assert_eq!(
            store.list_task_plan(context)?,
            vec![TaskPlanEntry { task_id: child.clone(), placement: child_batch(&root, 10) }],
        );
        assert_eq!(store.load_task(context, &root)?.unwrap().title, "Updated title");
        let events = store.list_events_after(context, None, 20)?;
        assert_eq!(events.len(), 4, "creation retries and ordinary creates add no plan event");
        assert_eq!(events.last().unwrap().payload["task_plan"], serde_json::Value::Null);
        Ok(())
    })
}

#[test]
fn task_plan_store_rejects_invalid_scope_or_reorder() -> Result<()> {
    for_each_provider(|store, context, database| {
        let parent = TaskId::new("parent")?;
        let child = TaskId::new("child")?;
        let other_context = fixture_project(database, TenantScope::default(), "other-project")?;
        let other_tenant = fixture_project(
            database, TenantScope::new("other-platform", "other-company")?, "run-store-project",
        )?;
        store.create_task(&other_context, finite_task_with_status(&parent, AgentTaskStatus::Queued)?)?;
        store.create_task(&other_tenant, finite_task_with_status(&parent, AgentTaskStatus::Queued)?)?;
        let task = finite_task_with_status(&child, AgentTaskStatus::Queued)?;
        let error = store.create_task_with_plan(context, task.clone(), child_batch(&parent, 1))
            .expect_err("foreign parent must be rejected");
        assert!(error.to_string().contains(parent.as_str()));
        assert!(store.load_task(context, &child)?.is_none());

        let mut recurring = finite_task_with_status(&parent, AgentTaskStatus::Queued)?;
        recurring.kind = AgentTaskKind::Infinite;
        let recurring_id = TaskId::new("recurring")?;
        recurring.id = recurring_id.clone();
        store.create_task(context, recurring)?;
        assert!(store.create_task_with_plan(context, task.clone(), child_batch(&recurring_id, 1)).is_err());
        assert!(store.create_task_with_plan(context, task.clone(), child_batch(&child, 1)).is_err());
        assert!(store.create_task_with_plan(context, task.clone(), root_batch(0, 1)).is_err());
        store.create_task(context, finite_task_with_status(&parent, AgentTaskStatus::Queued)?)?;
        store.create_task_with_plan(context, task, child_batch(&parent, 1))?;
        let grandchild = finite_task_with_status(&TaskId::new("grandchild")?, AgentTaskStatus::Queued)?;
        assert!(store.create_task_with_plan(context, grandchild, child_batch(&child, 1)).is_err());
        assert!(edit_placement(store, context, &child, None).is_err());
        let error = store.update_task_board_task(context, &parent, AgentTaskBoardUpdate {
            title: None, status: None, metadata: json!({"task_board.deleted": true}),
            updated_at_ms: TimestampMs(3000),
        }).expect_err("parent with a child cannot be deleted");
        assert!(error.to_string().contains(child.as_str()));

        let late = TaskId::new("late")?;
        store.create_task_with_plan(
            context, finite_task_with_status(&late, AgentTaskStatus::Queued)?, root_batch(1, 10),
        )?;
        let run = store.claim_ready_task_run(context, crate::store::runs::ClaimReadyTaskRunRequest {
            run_id: crate::domain::RunId::new("late-run")?,
            task_id: late.clone(),
            lease_acquired_at_ms: TimestampMs(1000),
            lease_expires_at_ms: Some(TimestampMs(2000)),
        })?;
        assert!(run.is_some());
        assert!(edit_placement(store, context, &late, Some(root_batch(1, 11))).is_err());
        assert!(edit_placement(store, context, &late, None).is_err());

        let pending = TaskId::new("pending")?;
        store.create_task_with_plan(
            context, finite_task_with_status(&pending, AgentTaskStatus::Queued)?, root_batch(1, 11),
        )?;
        let error = edit_placement(store, context, &pending, Some(root_batch(1, 2)))
            .expect_err("cannot insert an unfinished task before a claimed task");
        assert!(error.to_string().contains(late.as_str()));
        assert_eq!(store.load_task(context, &pending)?.unwrap().title, "Run lease storage");
        let error = store.update_task_board_task(context, &late, AgentTaskBoardUpdate {
            title: None, status: Some(AgentTaskStatus::Archived), metadata: json!({}),
            updated_at_ms: TimestampMs(3500),
        }).expect_err("cannot hide an unsuccessful predecessor");
        assert!(error.to_string().contains(pending.as_str()));
        Ok(())
    })
}

#[test]
fn task_plan_store_upgrade_keeps_existing_tasks() -> Result<()> {
    for_each_provider(|store, context, database| {
        let id = TaskId::new("existing")?;
        let task = store.create_task(context, finite_task_with_status(&id, AgentTaskStatus::Dispatched)?)?;
        let request = claim_request("existing-run", &id, "worker-a");
        let run = store.claim_task_run(context, request)?.expect("run before upgrade");
        // The pre-upgrade schema is identical apart from the new sparse table and its index.
        match database.provider() {
            DatabaseProvider::Sqlite => database.connect_sqlite()?.execute_batch("DROP TABLE scheduler_task_plans")?,
            DatabaseProvider::Postgres => database.connect_postgres()?.batch_execute("DROP TABLE scheduler_task_plans")?,
        }
        let reopened = AgentSchedulerStore::from_database(database.clone());
        reopened.initialize_project(context)?;
        assert!(reopened.list_task_plan(context)?.is_empty());
        assert_eq!(reopened.load_task(context, &id)?.unwrap(), task);
        assert_eq!(reopened.load_run(context, &run.id)?.unwrap(), run);
        Ok(())
    })
}

#[test]
fn task_plan_store_rolls_back_when_audit_fails() -> Result<()> {
    for_each_provider(|store, context, database| {
        let existing = TaskId::new("existing")?;
        let rejected = TaskId::new("rejected")?;
        let before = store.create_task(
            context, finite_task_with_status(&existing, AgentTaskStatus::Queued)?,
        )?;
        match database.provider() {
            DatabaseProvider::Sqlite => database.connect_sqlite()?.execute_batch(
                "CREATE TRIGGER reject_plan_audit BEFORE INSERT ON scheduler_events
                 WHEN NEW.message = 'task plan placement changed'
                 BEGIN SELECT RAISE(ABORT, 'audit unavailable'); END;",
            )?,
            DatabaseProvider::Postgres => database.connect_postgres()?.batch_execute(
                "ALTER TABLE scheduler_events ADD CONSTRAINT reject_plan_audit
                 CHECK (message <> 'task plan placement changed')",
            )?,
        }

        assert!(store.create_task_with_plan(
            context, finite_task_with_status(&rejected, AgentTaskStatus::Queued)?, root_batch(1, 1),
        ).is_err());
        assert!(edit_placement(store, context, &existing, Some(root_batch(1, 1))).is_err());
        assert!(store.load_task(context, &rejected)?.is_none());
        assert_eq!(store.load_task(context, &existing)?.unwrap(), before);
        assert!(store.list_task_plan(context)?.is_empty());
        assert!(store.list_events_after(context, None, 20)?.is_empty());
        Ok(())
    })
}

fn root_batch(outer_batch: u32, inner_batch: u32) -> TaskPlanPlacement {
    TaskPlanPlacement::Root { outer_batch, inner_batch }
}

fn child_batch(parent: &TaskId, batch: u32) -> TaskPlanPlacement {
    TaskPlanPlacement::Child { parent_task_id: parent.clone(), batch }
}

fn edit_placement(
    store: &AgentSchedulerStore,
    context: &AgentSchedulerContext,
    id: &TaskId,
    placement: Option<TaskPlanPlacement>,
) -> Result<()> {
    store.edit_task_with_plan(context, id, placement, Default::default(), |current| {
        Ok(AgentTaskBoardUpdate {
            title: Some("Updated title".to_string()),
            status: None,
            metadata: current.metadata.clone(),
            updated_at_ms: TimestampMs(2500),
        })
    })?.expect("task to edit");
    Ok(())
}

fn fixture_project(
    database: &GlobalDatabase, tenant: TenantScope, id: &str,
) -> Result<AgentSchedulerContext> {
    let directory = tempfile::tempdir()?;
    let path = directory.path();
    ProjectStore::new(database.clone())?.create(
        &tenant,
        &Info {
            id: id.to_owned(),
            worktree: path.to_string_lossy().into_owned(),
            vcs: None,
            name: None,
            icon: None,
            commands: None,
            time: TimeInfo { created: 0, updated: 0, initialized: None },
            sandboxes: Vec::new(),
        },
    )?;
    let scope = AgentSchedulerScope::new(ProjectScope::new(tenant, ProjectId::from(id))?);
    AgentSchedulerContext::new(scope, path)
}

fn for_each_provider(
    mut check: impl FnMut(&AgentSchedulerStore, &AgentSchedulerContext, &GlobalDatabase) -> Result<()>,
) -> Result<()> {
    let sqlite_home = tempfile::tempdir()?;
    let sqlite_database = GlobalDatabase::for_home(sqlite_home.path())?;
    let (store, context) =
        scheduler_store_fixture(sqlite_database.clone(), &sqlite_home.path().join("project"))?;
    check(&store, &context, &sqlite_database)?;

    let fixture = agent_database::test_support::postgres_test_database!();
    let project = tempfile::tempdir()?;
    let (store, context) = scheduler_store_fixture(fixture.database().clone(), project.path())?;
    check(&store, &context, fixture.database())?;
    fixture.cleanup()
}

#[test]
fn task_plan_store_publish_rolls_back_and_replays() -> Result<()> {
    use vw_api_agent_scheduler::{TaskPlanSubTask, WorkPlanPublication};
    use crate::domain::RunId;

    for_each_provider(|store, context, _| {
        let parent = TaskId::new("publish-parent")?;
        let first = super::task_plan::generated_child_id(&parent, 0)?;
        let collision = super::task_plan::generated_child_id(&parent, 1)?;
        store.create_task(context, finite_task_with_status(&parent, AgentTaskStatus::Queued)?)?;
        store.create_task(context, finite_task_with_status(&collision, AgentTaskStatus::Draft)?)?;
        let now = TimestampMs(2_000);
        let run = store.claim_ready_task_run(context, crate::store::runs::ClaimReadyTaskRunRequest {
            task_id: parent.clone(),
            run_id: RunId::new("planning-run")?,
            lease_acquired_at_ms: now,
            lease_expires_at_ms: Some(TimestampMs(4_000)),
        })?.unwrap();
        for status in [AgentTaskStatus::Dispatched, AgentTaskStatus::Acknowledged, AgentTaskStatus::Running] {
            store.transition_task_status(context, &parent, status, now)?;
        }
        let item = |title: &str, batch| TaskPlanSubTask {
            title: title.into(),
            boundary: "修改规划范围内的文件".into(),
            acceptance_criteria: vec!["检查产物".into()],
            target_files: vec![format!("{title}.rs")],
            batch,
            depends_on_batches: if batch == 1 { vec![] } else { vec![1] },
            execution_prompt: title.into(),
        };
        let request = WorkPlanPublication::Generated {
            planning_attempt_id: run.id.as_str().into(),
            subtasks: vec![item("first", 1), item("second", 2)],
        };
        assert!(store.publish_work_plan(context, &parent, &request, now).is_err());
        assert!(store.load_task(context, &first)?.is_none());
        assert_eq!(store.load_task(context, &parent)?.unwrap().status, AgentTaskStatus::Running);
        assert!(store.load_run(context, &run.id)?.unwrap().finished_at_ms.is_none());

        let accepted = WorkPlanPublication::Generated {
            planning_attempt_id: run.id.as_str().into(),
            subtasks: vec![item("first", 1)],
        };
        let ids = store.publish_work_plan(context, &parent, &accepted, now)?;
        assert_eq!(ids, vec![first.as_str().to_owned()]);
        assert_eq!(store.publish_work_plan(context, &parent, &accepted, now)?, ids);
        assert!(store.publish_work_plan(context, &parent, &request, now).is_err());
        assert_eq!(store.load_task(context, &parent)?.unwrap().status, AgentTaskStatus::Blocked);
        assert!(store.load_run(context, &run.id)?.unwrap().finished_at_ms.is_some());
        assert_eq!(store.list_task_plan_children(context, &parent)?.len(), 1);
        Ok(())
    })
}

#[test]
fn task_plan_store_manual_publication_freezes_children() -> Result<()> {
    use vw_api_agent_scheduler::WorkPlanPublication;

    for_each_provider(|store, context, _| {
        let parent = TaskId::new("manual-parent")?;
        let child = TaskId::new("manual-child")?;
        let extra = TaskId::new("manual-extra")?;
        let now = TimestampMs(3_000);
        store.create_task(context, finite_task_with_status(&parent, AgentTaskStatus::Queued)?)?;
        store.create_task_with_plan(
            context, finite_task_with_status(&child, AgentTaskStatus::Draft)?, child_batch(&parent, 1),
        )?;
        assert!(store.claim_ready_task_run(context, crate::store::runs::ClaimReadyTaskRunRequest {
            task_id: parent.clone(),
            run_id: crate::domain::RunId::new("unpublished-parent-run")?,
            lease_acquired_at_ms: now,
            lease_expires_at_ms: None,
        })?.is_none());
        let ids = store.publish_work_plan(context, &parent, &WorkPlanPublication::Manual, now)?;
        assert_eq!(ids, vec![child.as_str()]);
        assert_eq!(store.publish_work_plan(context, &parent, &WorkPlanPublication::Manual, now)?, ids);
        assert_eq!(store.load_task(context, &child)?.unwrap().status, AgentTaskStatus::Queued);
        assert!(edit_placement(store, context, &child, Some(child_batch(&parent, 2))).is_err());
        assert!(store.create_task_with_plan(
            context, finite_task_with_status(&extra, AgentTaskStatus::Draft)?, child_batch(&parent, 1),
        ).is_err());
        assert!(store.load_task(context, &extra)?.is_none());
        Ok(())
    })
}

#[test]
fn work_plan_parent_completion_claim_rechecks_after_reopen() -> Result<()> {
    use crate::domain::RunId;
    use crate::store::runs::ClaimReadyTaskRunRequest;
    use vw_api_agent_scheduler::WorkPlanPublication;

    for_each_provider(|store, context, database| {
        let parent = TaskId::new("summary-parent")?;
        let child = TaskId::new("summary-child")?;
        store.create_task(context, finite_task_with_status(&parent, AgentTaskStatus::Queued)?)?;
        store.create_task_with_plan(
            context,
            finite_task_with_status(&child, AgentTaskStatus::Queued)?,
            child_batch(&parent, 1),
        )?;
        store.publish_work_plan(context, &parent, &WorkPlanPublication::Manual, TimestampMs(1))?;
        let set_child = |status, key| store.update_task_board_task(context, &child, AgentTaskBoardUpdate {
            title: None,
            status: Some(status),
            metadata: json!({"task_board.status_key": key}),
            updated_at_ms: TimestampMs(2),
        });
        // Runtime completion alone cannot override the workflow's Done role.
        set_child(AgentTaskStatus::Completed, "running")?;
        assert_eq!(store.queue_completed_work_plans(context, TimestampMs(3))?, 0);
        set_child(AgentTaskStatus::Completed, "completed")?;
        store.transition_task_status(context, &parent, AgentTaskStatus::Paused, TimestampMs(3))?;
        assert_eq!(store.queue_completed_work_plans(context, TimestampMs(4))?, 0);
        let board = crate::task_board::AgentTaskBoardService::new(store);
        board.archive_task(context, &parent, None, TimestampMs(4))?.expect("archive plan parent");
        assert_eq!(store.queue_completed_work_plans(context, TimestampMs(4))?, 0);
        board.restore_task(context, &parent, None, TimestampMs(4))?.expect("restore plan parent");
        store.update_task_board_task(context, &parent, AgentTaskBoardUpdate {
            title: None,
            status: Some(AgentTaskStatus::Queued),
            metadata: json!({"task_board.status_key": "pending"}),
            updated_at_ms: TimestampMs(4),
        })?;
        assert_eq!(store.queue_completed_work_plans(context, TimestampMs(4))?, 1);

        let reopened = AgentSchedulerStore::from_database(database.clone());
        let run_id = RunId::new("summary-run")?;
        let request = ClaimReadyTaskRunRequest {
            run_id: run_id.clone(),
            task_id: parent.clone(),
            lease_acquired_at_ms: TimestampMs(5),
            lease_expires_at_ms: None,
        };
        set_child(AgentTaskStatus::Queued, "pending")?;
        assert!(reopened.claim_ready_task_run(context, request.clone())?.is_none());
        set_child(AgentTaskStatus::Completed, "completed")?;
        assert_eq!(reopened.queue_completed_work_plans(context, TimestampMs(5))?, 0);
        assert!(reopened.claim_ready_task_run(context, request)?.is_some());
        assert!(reopened.is_work_plan_summary_run(context, &parent, &run_id)?);
        assert!(set_child(AgentTaskStatus::Queued, "pending").is_err());
        assert_eq!(reopened.load_task(context, &child)?.unwrap().status, AgentTaskStatus::Completed);
        Ok(())
    })
}
