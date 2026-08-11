use std::collections::{BTreeMap, BTreeSet};

use ::postgres::GenericClient;
use anyhow::{Context, Result};
use vw_api_types::agent_scheduler::AgentSchedulerScope;

use super::PostgresAgentSchedulerStore;
use crate::agent_scheduler::domain::TaskId;
use crate::agent_scheduler::state_machine::{AgentTaskKind, AgentTaskStatus};
use crate::agent_scheduler::store::AgentSchedulerStorageContext;
use crate::agent_scheduler::store::dependencies::ensure_mutable_successors;
use crate::agent_scheduler::task_graph::{AgentTaskDependency, TaskGraph, TaskGraphTaskSnapshot};

impl PostgresAgentSchedulerStore {
    pub(in crate::agent_scheduler::store) fn list_task_dependencies(
        &self,
        context: &(impl AgentSchedulerStorageContext + ?Sized),
    ) -> Result<Vec<AgentTaskDependency>> {
        crate::global_database::run_sync_database(|| {
            let scope = context.storage_scope()?;
            let mut connection = self.database.connect_postgres()?;
            list_task_dependencies_with_client(&mut *connection, &scope)
        })
    }

    pub(in crate::agent_scheduler::store) fn replace_task_dependencies(
        &self,
        context: &(impl AgentSchedulerStorageContext + ?Sized),
        dependencies: Vec<AgentTaskDependency>,
    ) -> Result<()> {
        crate::global_database::run_sync_database(|| {
            let scope = context.storage_scope()?;
            let mut connection = self.database.connect_postgres()?;
            let mut transaction =
                connection.transaction().context("begin PostgreSQL task dependency replacement")?;
            let existing_dependencies =
                list_task_dependencies_with_client(&mut transaction, &scope)?;
            let task_ids = dependency_task_ids(&existing_dependencies, &dependencies);
            let tasks = lock_task_snapshots(&mut transaction, &scope, &task_ids)?;
            let graph_task_ids = dependencies
                .iter()
                .flat_map(|dependency| [&dependency.predecessor, &dependency.successor])
                .map(|task_id| task_id.as_str().to_owned())
                .collect::<BTreeSet<_>>();
            let graph_tasks = graph_task_ids
                .iter()
                .map(|task_id| {
                    tasks
                        .get(task_id)
                        .cloned()
                        .with_context(|| format!("task graph task does not exist: {task_id}"))
                })
                .collect::<Result<Vec<_>>>()?;
            let dependencies = normalize_dependencies(dependencies);
            TaskGraph::new(graph_tasks, dependencies.clone())?;

            ensure_mutable_successors(&existing_dependencies, &dependencies, &tasks)?;
            replace_task_dependencies_with_client(&mut transaction, &scope, &dependencies)?;
            transaction.commit().context("commit PostgreSQL task dependency replacement")?;
            Ok(())
        })
    }
}

fn list_task_dependencies_with_client(
    client: &mut impl GenericClient,
    scope: &AgentSchedulerScope,
) -> Result<Vec<AgentTaskDependency>> {
    let project = scope.project_scope();
    client
        .query(
            "SELECT predecessor_task_id, successor_task_id
             FROM scheduler_task_dependencies
             WHERE platform_id = $1 AND company_id = $2 AND project_id = $3
             ORDER BY predecessor_task_id, successor_task_id",
            &[
                &project.platform_id().as_ref(),
                &project.company_id().as_ref(),
                &project.project_id().as_ref(),
            ],
        )
        .context("list scoped PostgreSQL task dependencies")?
        .into_iter()
        .map(|row| {
            let predecessor: String = row.try_get(0)?;
            let successor: String = row.try_get(1)?;
            Ok(AgentTaskDependency::new(
                TaskId::new(predecessor).context("parse dependency predecessor task id")?,
                TaskId::new(successor).context("parse dependency successor task id")?,
            ))
        })
        .collect()
}

fn dependency_task_ids(
    existing_dependencies: &[AgentTaskDependency],
    dependencies: &[AgentTaskDependency],
) -> BTreeSet<String> {
    existing_dependencies
        .iter()
        .chain(dependencies)
        .flat_map(|dependency| [&dependency.predecessor, &dependency.successor])
        .map(|task_id| task_id.as_str().to_owned())
        .collect()
}

fn lock_task_snapshots(
    client: &mut impl GenericClient,
    scope: &AgentSchedulerScope,
    task_ids: &BTreeSet<String>,
) -> Result<BTreeMap<String, TaskGraphTaskSnapshot>> {
    let project = scope.project_scope();
    let mut tasks = BTreeMap::new();
    for task_id in task_ids {
        let task = client
            .query_opt(
                "SELECT id, kind, status
                 FROM scheduler_tasks
                 WHERE platform_id = $1 AND company_id = $2 AND project_id = $3 AND id = $4
                 FOR UPDATE",
                &[
                    &project.platform_id().as_ref(),
                    &project.company_id().as_ref(),
                    &project.project_id().as_ref(),
                    &task_id,
                ],
            )
            .with_context(|| format!("lock scoped PostgreSQL task graph task: {task_id}"))?;
        let Some(task) = task else {
            continue;
        };
        let id: String = task.try_get(0).context("read PostgreSQL task graph task id")?;
        let kind: String = task.try_get(1).context("read PostgreSQL task graph task kind")?;
        let status: String = task.try_get(2).context("read PostgreSQL task graph task status")?;
        let snapshot = TaskGraphTaskSnapshot::new(
            TaskId::new(id).context("parse PostgreSQL task graph task id")?,
            AgentTaskKind::parse(&kind).context("parse PostgreSQL task graph task kind")?,
            AgentTaskStatus::parse(&status).context("parse PostgreSQL task graph task status")?,
        );
        tasks.insert(snapshot.id.as_str().to_owned(), snapshot);
    }
    Ok(tasks)
}

fn normalize_dependencies(mut dependencies: Vec<AgentTaskDependency>) -> Vec<AgentTaskDependency> {
    dependencies.sort_by(|left, right| {
        (left.predecessor.as_str(), left.successor.as_str())
            .cmp(&(right.predecessor.as_str(), right.successor.as_str()))
    });
    dependencies
}

fn replace_task_dependencies_with_client(
    client: &mut impl GenericClient,
    scope: &AgentSchedulerScope,
    dependencies: &[AgentTaskDependency],
) -> Result<()> {
    let project = scope.project_scope();
    client
        .execute(
            "DELETE FROM scheduler_task_dependencies
             WHERE platform_id = $1 AND company_id = $2 AND project_id = $3",
            &[
                &project.platform_id().as_ref(),
                &project.company_id().as_ref(),
                &project.project_id().as_ref(),
            ],
        )
        .context("delete scoped PostgreSQL task dependencies")?;
    for dependency in dependencies {
        client
            .execute(
                "INSERT INTO scheduler_task_dependencies (
                     platform_id, company_id, project_id, predecessor_task_id, successor_task_id
                 ) VALUES ($1, $2, $3, $4, $5)",
                &[
                    &project.platform_id().as_ref(),
                    &project.company_id().as_ref(),
                    &project.project_id().as_ref(),
                    &dependency.predecessor.as_str(),
                    &dependency.successor.as_str(),
                ],
            )
            .with_context(|| {
                format!(
                    "insert scoped PostgreSQL task dependency: {} -> {}",
                    dependency.predecessor.as_str(),
                    dependency.successor.as_str()
                )
            })?;
    }
    Ok(())
}

#[cfg(test)]
#[path = "dependencies_tests.rs"]
mod dependencies_tests;
