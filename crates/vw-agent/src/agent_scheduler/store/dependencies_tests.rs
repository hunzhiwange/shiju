//! 测试任务依赖存储的拓扑约束、查询与作用域隔离。
//! 自依赖、重复边和跨作用域前驱明确拒绝，合法依赖按稳定顺序返回。

use anyhow::Result;
use serde_json::json;
use vw_api_types::agent_scheduler::AgentSchedulerScope;
use vw_api_types::common::TimestampMs;
use vw_api_types::id::{ProjectId, ProjectScope, TenantScope};

use super::super::domain::{AgentSchedulerContext, TaskId};
use super::super::state_machine::{AgentTaskKind, AgentTaskStatus};
use super::super::task_graph::AgentTaskDependency;
use super::AgentSchedulerStore;
use super::tasks::NewAgentTask;

#[test]
fn agent_scheduler_store_replaces_scoped_task_graph_atomically() -> Result<()> {
    let store = AgentSchedulerStore::new_for_tests()?;
    let temp_dir = tempfile::tempdir()?;
    let scope_a = scheduler_context(temp_dir.path(), "platform-a", "company-a", "project")?;
    let scope_b = scheduler_context(temp_dir.path(), "platform-b", "company-b", "project")?;
    for context in [&scope_a, &scope_b] {
        for task_id in ["task-a", "task-b", "task-c"] {
            store.create_task(
                context,
                task(task_id, AgentTaskKind::Finite, AgentTaskStatus::Draft)?,
            )?;
        }
    }

    store.replace_task_dependencies(&scope_a, vec![dependency("task-a", "task-b")])?;
    assert_eq!(store.list_task_dependencies(&scope_a)?, vec![dependency("task-a", "task-b")]);
    assert_eq!(
        store
            .load_task_graph(&scope_a)?
            .tasks()
            .map(|task| task.id.as_str().to_owned())
            .collect::<Vec<_>>(),
        vec!["task-a", "task-b"]
    );
    store.replace_task_dependencies(&scope_b, vec![dependency("task-b", "task-c")])?;

    store.replace_task_dependencies(&scope_a, vec![dependency("task-a", "task-c")])?;
    assert_eq!(store.list_task_dependencies(&scope_a)?, vec![dependency("task-a", "task-c")]);
    assert_eq!(store.list_task_dependencies(&scope_b)?, vec![dependency("task-b", "task-c")]);

    Ok(())
}

#[test]
fn agent_scheduler_store_rejects_invalid_or_active_graph_mutation() -> Result<()> {
    let store = AgentSchedulerStore::new_for_tests()?;
    let temp_dir = tempfile::tempdir()?;
    let scope_a = scheduler_context(temp_dir.path(), "platform-a", "company-a", "project")?;
    let scope_b = scheduler_context(temp_dir.path(), "platform-b", "company-b", "project")?;
    for task_id in ["task-a", "task-b", "task-c"] {
        store
            .create_task(&scope_a, task(task_id, AgentTaskKind::Finite, AgentTaskStatus::Draft)?)?;
    }
    store.create_task(
        &scope_a,
        task("task-infinite", AgentTaskKind::Infinite, AgentTaskStatus::Draft)?,
    )?;
    store.create_task(
        &scope_b,
        task("task-other-scope", AgentTaskKind::Finite, AgentTaskStatus::Draft)?,
    )?;

    let initial = vec![dependency("task-a", "task-b")];
    store.replace_task_dependencies(&scope_a, initial.clone())?;

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
        assert!(store.replace_task_dependencies(&scope_a, invalid_graph).is_err());
        assert_eq!(store.list_task_dependencies(&scope_a)?, initial);
    }

    store.transition_task_status(
        &scope_a,
        &TaskId::new("task-b")?,
        AgentTaskStatus::Assigned,
        TimestampMs(1),
    )?;
    store.replace_task_dependencies(&scope_a, initial.clone())?;
    assert!(
        store.replace_task_dependencies(&scope_a, vec![dependency("task-c", "task-b")]).is_err()
    );
    assert_eq!(store.list_task_dependencies(&scope_a)?, initial);

    Ok(())
}

fn dependency(predecessor: &str, successor: &str) -> AgentTaskDependency {
    AgentTaskDependency::new(
        TaskId::new(predecessor).expect("valid predecessor task id"),
        TaskId::new(successor).expect("valid successor task id"),
    )
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
