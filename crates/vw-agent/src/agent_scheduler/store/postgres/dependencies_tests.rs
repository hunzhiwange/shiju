use anyhow::Result;
use serde_json::json;
use vw_api_types::agent_scheduler::AgentSchedulerScope;
use vw_api_types::common::TimestampMs;
use vw_api_types::id::{ProjectId, ProjectScope, TenantScope};
use vw_shared::project::{Info as ProjectInfo, TimeInfo as ProjectTimeInfo};

use super::super::super::super::domain::{AgentSchedulerContext, TaskId};
use super::super::super::super::state_machine::{AgentTaskKind, AgentTaskStatus};
use super::super::super::super::task_graph::AgentTaskDependency;
use super::super::super::AgentSchedulerStore;
use super::super::super::tasks::NewAgentTask;
use super::super::PostgresAgentSchedulerStore;
use crate::global_database::GlobalDatabase;
use crate::global_database::test_support::PostgresTestDatabase;
use crate::project::store::ProjectStore;

#[test]
fn postgres_task_graph_matches_sqlite_replacement_contract() -> Result<()> {
    let fixture = crate::global_database::test_support::postgres_test_database!();
    let project = tempfile::tempdir()?;
    let (store, context, isolated_context) =
        scheduler_store_fixture(fixture.database().clone(), project.path())?;

    for task_id in ["task-a", "task-b", "task-c"] {
        store
            .create_task(&context, task(task_id, AgentTaskKind::Finite, AgentTaskStatus::Draft)?)?;
    }
    store.create_task(
        &context,
        task("task-infinite", AgentTaskKind::Infinite, AgentTaskStatus::Draft)?,
    )?;
    for task_id in ["task-a", "task-b", "task-c", "task-other-scope"] {
        store.create_task(
            &isolated_context,
            task(task_id, AgentTaskKind::Finite, AgentTaskStatus::Draft)?,
        )?;
    }

    store.replace_task_dependencies(&context, vec![dependency("task-a", "task-b")])?;
    assert_eq!(store.list_task_dependencies(&context)?, vec![dependency("task-a", "task-b")]);
    assert_eq!(
        store
            .load_task_graph(&context)?
            .tasks()
            .map(|task| task.id.as_str().to_owned())
            .collect::<Vec<_>>(),
        vec!["task-a", "task-b"]
    );
    let initial = vec![dependency("task-a", "task-b")];
    for invalid_graph in [
        vec![dependency("task-a", "task-missing")],
        vec![dependency("task-a", "task-other-scope")],
        vec![
            dependency("task-a", "task-b"),
            dependency("task-b", "task-c"),
            dependency("task-c", "task-a"),
        ],
        vec![dependency("task-infinite", "task-b")],
    ] {
        assert!(store.replace_task_dependencies(&context, invalid_graph).is_err());
        assert_eq!(store.list_task_dependencies(&context)?, initial);
    }
    store.replace_task_dependencies(&isolated_context, vec![dependency("task-b", "task-c")])?;

    store.replace_task_dependencies(&context, vec![dependency("task-a", "task-c")])?;
    assert_eq!(store.list_task_dependencies(&context)?, vec![dependency("task-a", "task-c")]);
    assert_eq!(
        store.list_task_dependencies(&isolated_context)?,
        vec![dependency("task-b", "task-c")]
    );

    fixture.cleanup()
}

fn scheduler_store_fixture(
    database: GlobalDatabase,
    project_path: &std::path::Path,
) -> Result<(AgentSchedulerStore, AgentSchedulerContext, AgentSchedulerContext)> {
    let tenant = TenantScope::new("dependencies-platform", "dependencies-company")?;
    let isolated_tenant =
        TenantScope::new("dependencies-platform-isolated", "dependencies-company-isolated")?;
    let project_id = ProjectId::from("dependencies-store-project");
    let project_store = ProjectStore::new(database.clone())?;
    for tenant in [&tenant, &isolated_tenant] {
        project_store.create(
            tenant,
            &ProjectInfo {
                id: project_id.as_ref().to_owned(),
                worktree: project_path.to_string_lossy().into_owned(),
                vcs: None,
                name: Some("Dependencies store project".to_owned()),
                icon: None,
                commands: None,
                time: ProjectTimeInfo { created: 0, updated: 0, initialized: None },
                sandboxes: Vec::new(),
            },
        )?;
    }
    let context = AgentSchedulerContext::new(
        AgentSchedulerScope::new(ProjectScope::new(tenant, project_id.clone())?),
        project_path,
    )?;
    let isolated_context = AgentSchedulerContext::new(
        AgentSchedulerScope::new(ProjectScope::new(isolated_tenant, project_id)?),
        project_path,
    )?;
    Ok((AgentSchedulerStore::from_database(database), context, isolated_context))
}

fn dependency(predecessor: &str, successor: &str) -> AgentTaskDependency {
    AgentTaskDependency::new(
        TaskId::new(predecessor).expect("valid predecessor task id"),
        TaskId::new(successor).expect("valid successor task id"),
    )
}

fn task(id: &str, kind: AgentTaskKind, status: AgentTaskStatus) -> Result<NewAgentTask> {
    Ok(NewAgentTask {
        id: TaskId::new(id)?,
        title: format!("Task {id}"),
        kind,
        status,
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
        metadata: json!({}),
        created_at_ms: TimestampMs(0),
    })
}
