use ::postgres::{GenericClient, Transaction};
use anyhow::Result;
use vw_api_agent_scheduler::AgentSchedulerScope;

use super::PostgresAgentSchedulerStore;
use super::tasks::{load_task_with_client, load_workflow_with_client};
use crate::domain::TaskId;
use crate::store::scope_params;
use crate::store::task_plan::{
    DELETE_PLAN, PlacementWrite, PlanMember, SELECT_MEMBERS, SELECT_PLAN, TaskPlanEntry, TaskPlanPlacement,
    PlanDispatchSnapshot, SELECT_DISPATCH_MEMBERS, UPSERT_PLAN, decode_entry, validate_placement,
};
use crate::store::tasks::StoredAgentTask;

impl PostgresAgentSchedulerStore {
    pub(in crate::store) fn load_plan_dispatch_snapshot(
        &self,
        scope: &AgentSchedulerScope,
    ) -> Result<PlanDispatchSnapshot> {
        agent_database::run_sync_database(|| {
            load_plan_dispatch_snapshot(&mut *self.database.connect_postgres()?, scope, None)
        })
    }

    pub(in crate::store) fn list_task_plan(
        &self,
        scope: &AgentSchedulerScope,
        parent_task_id: Option<&TaskId>,
    ) -> Result<Vec<TaskPlanEntry>> {
        agent_database::run_sync_database(|| {
            let (platform, company, project) = scope_params(scope);
            self.database.connect_postgres()?.query(
                SELECT_PLAN, &[&platform, &company, &project, &parent_task_id.map(TaskId::as_str)],
            )?.into_iter().map(|row| {
                decode_entry(row.try_get(0)?, row.try_get(1)?, row.try_get(2)?, row.try_get(3)?)
            }).collect()
        })
    }
}

// Acquire before task or run row locks. NO KEY UPDATE serializes project writers while
// remaining compatible with the key-share locks taken by existing project foreign keys.
pub(super) fn lock_task_plan_project(tx: &mut Transaction<'_>, scope: &AgentSchedulerScope) -> Result<()> {
    let (platform, company, project) = scope_params(scope);
    tx.query_one(
        "SELECT project_id FROM projects
         WHERE platform_id = $1 AND company_id = $2 AND project_id = $3 FOR NO KEY UPDATE",
        &[&platform, &company, &project],
    )?;
    Ok(())
}

pub(super) fn load_plan_members(
    client: &mut impl GenericClient, scope: &AgentSchedulerScope, task_id: &TaskId,
) -> Result<Vec<PlanMember>> {
    let (platform, company, project) = scope_params(scope);
    client.query(SELECT_MEMBERS, &[&platform, &company, &project, &task_id.as_str()])?
        .into_iter().map(|row| {
            PlanMember::decode(
                row.try_get(0)?, row.try_get(1)?, row.try_get(2)?, row.try_get(3)?,
                row.try_get(4)?, row.try_get(5)?, row.try_get(6)?,
            )
        }).collect()
}

pub(super) fn load_plan_dispatch_snapshot(
    client: &mut impl GenericClient,
    scope: &AgentSchedulerScope,
    task_id: Option<&TaskId>,
) -> Result<PlanDispatchSnapshot> {
    let (platform, company, project) = scope_params(scope);
    let rows = client.query(
        SELECT_DISPATCH_MEMBERS,
        &[&platform, &company, &project, &task_id.map(TaskId::as_str)],
    )?;
    PlanDispatchSnapshot::from_rows(rows.into_iter().map(|row| {
        Ok((
            row.try_get(0)?,
            row.try_get(1)?,
            row.try_get(2)?,
            row.try_get(3)?,
            row.try_get(4)?,
            row.try_get(5)?,
            row.try_get(6)?,
        ))
    }))
}

pub(super) fn write_placement(
    tx: &mut Transaction<'_>,
    scope: &AgentSchedulerScope,
    task: &StoredAgentTask,
    replacement: Option<&TaskPlanPlacement>,
    mode: PlacementWrite,
) -> Result<bool> {
    let members = load_plan_members(tx, scope, &task.id)?;
    let parent = match replacement {
        Some(TaskPlanPlacement::Child { parent_task_id, .. }) =>
            load_task_with_client(tx, scope, parent_task_id)?,
        _ => None,
    };
    if !validate_placement(
        &members, load_workflow_with_client(tx, scope)?, task, replacement, parent.as_ref(), mode,
    )? {
        return Ok(false);
    }
    let (platform, company, project) = scope_params(scope);
    if let Some(placement) = replacement {
        let (parent, outer, batch) = placement.columns();
        tx.execute(UPSERT_PLAN, &[&platform, &company, &project, &task.id.as_str(), &parent, &outer, &batch])?;
    } else {
        tx.execute(DELETE_PLAN, &[&platform, &company, &project, &task.id.as_str()])?;
    }
    Ok(true)
}
