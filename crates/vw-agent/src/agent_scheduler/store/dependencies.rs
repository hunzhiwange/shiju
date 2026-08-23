//! 持久化 SQLite 任务依赖、构建就绪图，并阻止修改已有执行约束的后继关系。
//! 依赖替换与相关任务读取在即时事务中完成，避免并发派发观察到半更新图。

use std::collections::{BTreeMap, BTreeSet};

use anyhow::{Context, Result, bail};
use rusqlite::{Connection, OptionalExtension, Transaction, TransactionBehavior, params};

use crate::agent_scheduler::domain::TaskId;
use crate::agent_scheduler::state_machine::{AgentTaskKind, AgentTaskStatus};
use crate::agent_scheduler::task_graph::{AgentTaskDependency, TaskGraph, TaskGraphTaskSnapshot};

use super::{
    AgentSchedulerStorageContext, AgentSchedulerStore, open_initialized_connection,
    open_locked_initialized_connection, postgres_provider, scope_params,
};

impl AgentSchedulerStore {
    pub fn list_task_dependencies(
        &self,
        context: &(impl AgentSchedulerStorageContext + ?Sized),
    ) -> Result<Vec<AgentTaskDependency>> {
        if let Some(store) = postgres_provider(self, context)? {
            return store.list_task_dependencies(context);
        }

        let scope = context.storage_scope()?;
        let connection = open_initialized_connection(self, context)?;
        list_task_dependencies_with_connection(&connection, &scope)
    }

    pub fn replace_task_dependencies(
        &self,
        context: &(impl AgentSchedulerStorageContext + ?Sized),
        dependencies: Vec<AgentTaskDependency>,
    ) -> Result<()> {
        if let Some(store) = postgres_provider(self, context)? {
            return store.replace_task_dependencies(context, dependencies);
        }

        let scope = context.storage_scope()?;
        let (_guard, mut connection) = open_locked_initialized_connection(self, context)?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let existing_dependencies = list_task_dependencies_with_connection(&transaction, &scope)?;
        let task_ids = dependency_task_ids(&existing_dependencies, &dependencies);
        let tasks = load_task_snapshots(&transaction, &scope, &task_ids)?;
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
        TaskGraph::new(graph_tasks, dependencies.clone())?;

        ensure_mutable_successors(&existing_dependencies, &dependencies, &tasks)?;
        replace_task_dependencies_with_connection(&transaction, &scope, &dependencies)?;
        transaction.commit()?;
        Ok(())
    }

    pub fn load_task_graph(
        &self,
        context: &(impl AgentSchedulerStorageContext + ?Sized),
    ) -> Result<TaskGraph> {
        if let Some(store) = postgres_provider(self, context)? {
            return store.load_task_graph(context);
        }

        let scope = context.storage_scope()?;
        let connection = open_initialized_connection(self, context)?;
        let dependencies = list_task_dependencies_with_connection(&connection, &scope)?;
        let task_ids = dependency_task_ids(&[], &dependencies);
        let tasks = load_task_snapshots(&connection, &scope, &task_ids)?;
        build_task_graph(dependencies, tasks)
    }
}

/// 从任务快照与依赖边构建已验证图，所有结构错误原样返回调用者。
pub(crate) fn build_task_graph(
    dependencies: Vec<AgentTaskDependency>,
    tasks: BTreeMap<String, TaskGraphTaskSnapshot>,
) -> Result<TaskGraph> {
    let graph_tasks = dependency_task_ids(&[], &dependencies)
        .iter()
        .map(|task_id| {
            tasks
                .get(task_id)
                .cloned()
                .with_context(|| format!("task graph task does not exist: {task_id}"))
        })
        .collect::<Result<Vec<_>>>()?;
    TaskGraph::new(graph_tasks, dependencies)
}

fn list_task_dependencies_with_connection(
    connection: &Connection,
    scope: &vw_api_types::agent_scheduler::AgentSchedulerScope,
) -> Result<Vec<AgentTaskDependency>> {
    let (platform_id, company_id, project_id) = scope_params(scope);
    let mut statement = connection.prepare(
        "SELECT predecessor_task_id, successor_task_id
         FROM scheduler_task_dependencies
         WHERE platform_id = ?1 AND company_id = ?2 AND project_id = ?3
         ORDER BY predecessor_task_id, successor_task_id",
    )?;
    statement
        .query_map(params![platform_id, company_id, project_id], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?
        .map(|row| {
            let (predecessor, successor) = row?;
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

fn load_task_snapshots(
    connection: &Connection,
    scope: &vw_api_types::agent_scheduler::AgentSchedulerScope,
    task_ids: &BTreeSet<String>,
) -> Result<BTreeMap<String, TaskGraphTaskSnapshot>> {
    let (platform_id, company_id, project_id) = scope_params(scope);
    let mut statement = connection.prepare(
        "SELECT id, kind, status
         FROM scheduler_tasks
         WHERE platform_id = ?1 AND company_id = ?2 AND project_id = ?3 AND id = ?4",
    )?;
    let mut tasks = BTreeMap::new();
    for task_id in task_ids {
        let task = statement
            .query_row(
                params![platform_id, company_id, project_id, task_id],
                |row: &rusqlite::Row<'_>| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                    ))
                },
            )
            .optional()?
            .map(|(id, kind, status)| {
                Ok::<TaskGraphTaskSnapshot, anyhow::Error>(TaskGraphTaskSnapshot::new(
                    TaskId::new(id).context("parse task graph task id")?,
                    parse_task_kind(&kind)?,
                    AgentTaskStatus::parse(&status)?,
                ))
            })
            .transpose()?;
        if let Some(task) = task {
            tasks.insert(task.id.as_str().to_owned(), task);
        }
    }
    Ok(tasks)
}

fn parse_task_kind(value: &str) -> Result<AgentTaskKind> {
    match value {
        "finite" => Ok(AgentTaskKind::Finite),
        "infinite" => Ok(AgentTaskKind::Infinite),
        "team_dag" => Ok(AgentTaskKind::TeamDag),
        other => bail!("unsupported agent task kind: {other}"),
    }
}

/// 拒绝修改已进入执行阶段任务的后继边，保护既有调度因果关系。
pub(crate) fn ensure_mutable_successors(
    existing_dependencies: &[AgentTaskDependency],
    dependencies: &[AgentTaskDependency],
    tasks: &BTreeMap<String, TaskGraphTaskSnapshot>,
) -> Result<()> {
    let existing_predecessors = predecessors_by_successor(existing_dependencies);
    let replacement_predecessors = predecessors_by_successor(dependencies);
    for successor in existing_predecessors.keys().chain(replacement_predecessors.keys()) {
        if existing_predecessors.get(successor) == replacement_predecessors.get(successor) {
            continue;
        }
        let task = tasks
            .get(successor)
            .with_context(|| format!("task graph successor does not exist: {successor}"))?;
        if !matches!(task.status, AgentTaskStatus::Draft | AgentTaskStatus::Queued) {
            bail!(
                "task graph successor {successor} must be draft or queued to change dependencies"
            );
        }
    }
    Ok(())
}

fn predecessors_by_successor(
    dependencies: &[AgentTaskDependency],
) -> BTreeMap<String, BTreeSet<String>> {
    let mut predecessors = BTreeMap::new();
    for dependency in dependencies {
        predecessors
            .entry(dependency.successor.as_str().to_owned())
            .or_insert_with(BTreeSet::new)
            .insert(dependency.predecessor.as_str().to_owned());
    }
    predecessors
}

fn replace_task_dependencies_with_connection(
    transaction: &Transaction<'_>,
    scope: &vw_api_types::agent_scheduler::AgentSchedulerScope,
    dependencies: &[AgentTaskDependency],
) -> Result<()> {
    let (platform_id, company_id, project_id) = scope_params(scope);
    transaction.execute(
        "DELETE FROM scheduler_task_dependencies
         WHERE platform_id = ?1 AND company_id = ?2 AND project_id = ?3",
        params![platform_id, company_id, project_id],
    )?;
    let mut statement = transaction.prepare(
        "INSERT INTO scheduler_task_dependencies (
             platform_id, company_id, project_id, predecessor_task_id, successor_task_id
         ) VALUES (?1, ?2, ?3, ?4, ?5)",
    )?;
    for dependency in dependencies {
        statement.execute(params![
            platform_id,
            company_id,
            project_id,
            dependency.predecessor.as_str(),
            dependency.successor.as_str(),
        ])?;
    }
    Ok(())
}
