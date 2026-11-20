use ::postgres::{GenericClient, Transaction};
use anyhow::{Context, Result};
use vw_api_agent_scheduler::{AgentSchedulerScope, WorkPlanPublication};
use vw_api_transport::common::TimestampMs;

use super::events::insert_event_with_client;
use crate::engine::dispatch::effective_dispatch_workflow;

use super::PostgresAgentSchedulerStore;
use super::tasks::{load_task_for_update, load_task_with_client, load_workflow_with_client};
use crate::domain::{RunId, TaskId};
use crate::state_machine::AgentTaskStatus;
use serde_json::Value;
use crate::store::scope_params;
use crate::store::task_plan::{
    DELETE_PLAN, PlacementWrite, PlanMember, SELECT_MEMBERS, SELECT_PLAN, TaskPlanEntry, TaskPlanPlacement,
    PlanDispatchSnapshot, SELECT_DISPATCH_MEMBERS, UPSERT_PLAN, decode_entry, validate_placement,
    FINISH_PLANNING_RUN, INSERT_PLAN_CHILD, INSERT_PUBLICATION, INSERT_RECEIPT,
    PublicationReceipt, SELECT_PLANNING_RUNS, SELECT_PUBLICATION, SELECT_RECEIPT, UPDATE_PLAN_TASK,
    conflict, generated_child, manual_child_metadata, missing_parent, placement_event,
    publication_attempt, publication_event, publication_key, queued_status_key, validate_publication,
    waiting_parent_metadata, CLAIM_SUMMARY, SELECT_CLAIMED_PARENT, changes_completion,
    prepare_summary, reject_claimed_child_update, summary_key,
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
        task_id: Option<&TaskId>,
    ) -> Result<Vec<TaskPlanEntry>> {
        agent_database::run_sync_database(|| {
            let (platform, company, project) = scope_params(scope);
            self.database.connect_postgres()?.query(
                SELECT_PLAN,
                &[&platform, &company, &project, &parent_task_id.map(TaskId::as_str), &task_id.map(TaskId::as_str)],
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
            row.try_get(7)?,
            row.try_get(8)?,
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
    if let Some(TaskPlanPlacement::Child { parent_task_id, .. }) = replacement
        && postgres_publication_exists(tx, scope, parent_task_id)?
    {
        return Err(conflict("published child placements are frozen"));
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

impl PostgresAgentSchedulerStore {
    pub(in crate::store) fn queue_completed_work_plans(
        &self, scope: &AgentSchedulerScope, now: TimestampMs,
    ) -> Result<usize> {
        agent_database::run_sync_database(|| {
            let mut conn = self.database.connect_postgres()?;
            let mut tx = conn.transaction()?;
            lock_task_plan_project(&mut tx, scope)?;
            let workflow = effective_dispatch_workflow(load_workflow_with_client(&mut tx, scope)?)?;
            let snapshot = load_plan_dispatch_snapshot(&mut tx, scope, None)?;
            let (platform, company, project) = scope_params(scope);
            let mut queued = 0;
            for (id, attempt) in snapshot.summaries_to_queue(&workflow) {
                let parent = load_task_for_update(&mut tx, scope, id)?.context("work plan parent disappeared")?;
                let Some((metadata, event)) = prepare_summary(&parent, &workflow, attempt, now)? else {
                    continue;
                };
                tx.execute(INSERT_RECEIPT, &[
                    &platform, &company, &project, &summary_key(id), &format!("pending:{attempt}"),
                ])?;
                tx.execute(UPDATE_PLAN_TASK, &[
                    &platform, &company, &project, &id.as_str(), &"queued", &metadata, &now.0,
                ])?;
                insert_event_with_client(&mut tx, scope, event)?;
                queued += 1;
            }
            tx.commit()?;
            Ok(queued)
        })
    }

    pub(in crate::store) fn summary_receipt(
        &self, scope: &AgentSchedulerScope, parent: &TaskId,
    ) -> Result<Option<String>> {
        agent_database::run_sync_database(|| {
            let (platform, company, project) = scope_params(scope);
            self.database.connect_postgres()?.query_opt(
                SELECT_RECEIPT, &[&platform, &company, &project, &summary_key(parent)],
            )?.map(|row| row.try_get(0)).transpose().map_err(Into::into)
        })
    }

    pub(in crate::store) fn work_plan_is_published(
        &self,
        scope: &AgentSchedulerScope,
        parent: &TaskId,
    ) -> Result<bool> {
        agent_database::run_sync_database(|| {
            postgres_publication_exists(&mut *self.database.connect_postgres()?, scope, parent)
        })
    }

    pub(in crate::store) fn publish_work_plan(
        &self,
        scope: &AgentSchedulerScope,
        parent_id: &TaskId,
        request: &WorkPlanPublication,
        now: TimestampMs,
    ) -> Result<Vec<String>> {
        agent_database::run_sync_database(|| {
            let mut conn = self.database.connect_postgres()?;
            let mut tx = conn.transaction()?;
            lock_task_plan_project(&mut tx, scope)?;
            let (platform, company, project) = scope_params(scope);
            let key = publication_key(parent_id);
            if postgres_publication_exists(&mut tx, scope, parent_id)? {
                let receipt = tx.query_opt(SELECT_RECEIPT, &[&platform, &company, &project, &key])?
                    .map(|row| row.try_get(0)).transpose()?;
                return PublicationReceipt::replay(receipt, request);
            }

            let parent = load_task_for_update(&mut tx, scope, parent_id)?.ok_or_else(missing_parent)?;
            let members = load_plan_members(&mut tx, scope, parent_id)?;
            let active_runs = tx.query(SELECT_PLANNING_RUNS, &[&platform, &company, &project, &parent_id.as_str()])?
                .into_iter().map(|row| row.try_get(0)).collect::<std::result::Result<Vec<String>, _>>()?;
            validate_publication(&parent, &members, request, &active_runs)?;
            let workflow = effective_dispatch_workflow(load_workflow_with_client(&mut tx, scope)?)?;
            let queued_key = queued_status_key(&workflow)?;
            let mut child_ids = Vec::new();
            match request {
                WorkPlanPublication::Generated { planning_attempt_id, subtasks } => {
                    for (index, subtask) in subtasks.iter().enumerate() {
                        let (id, metadata) = generated_child(&parent, subtask, index, &workflow, queued_key)?;
                        tx.execute(INSERT_PLAN_CHILD, &[
                            &platform, &company, &project, &id.as_str(), &subtask.title, &metadata, &now.0,
                            &parent.assigned_to.as_ref().map(|id| id.as_str()),
                            &parent.team_id.as_ref().map(|id| id.as_str()),
                            &parent.room_id, &parent.project_room_id,
                            &parent.delegated_to_team.as_ref().map(|id| id.as_str()),
                            &format!("task-board/{}/spec.md", id.as_str()),
                            &format!("task-board/{}/meta.json", id.as_str()),
                        ])?;
                        let placement = TaskPlanPlacement::Child { parent_task_id: parent_id.clone(), batch: subtask.batch };
                        tx.execute(UPSERT_PLAN, &[&platform, &company, &project, &id.as_str(),
                            &Some(parent_id.as_str()), &Option::<i64>::None, &i64::from(subtask.batch)])?;
                        let child = load_task_with_client(&mut tx, scope, &id)?.context("published child disappeared")?;
                        insert_event_with_client(&mut tx, scope, placement_event(&child, Some(&placement), true))?;
                        child_ids.push(id.as_str().to_owned());
                    }
                    if tx.execute(FINISH_PLANNING_RUN, &[
                        &platform, &company, &project, &parent_id.as_str(), planning_attempt_id, &now.0
                    ])? != 1 {
                        return Err(conflict("planning run ended before publication"));
                    }
                }
                WorkPlanPublication::Manual => {
                    for child in members.iter().filter(|member| member.is_child_of(parent_id)) {
                        let metadata = manual_child_metadata(&parent, child, &workflow, queued_key)?;
                        tx.execute(UPDATE_PLAN_TASK, &[
                            &platform, &company, &project, &child.id.as_str(), &"queued", &metadata, &now.0
                        ])?;
                        child_ids.push(child.id.as_str().to_owned());
                    }
                }
            }
            tx.execute(UPDATE_PLAN_TASK, &[
                &platform, &company, &project, &parent_id.as_str(), &"blocked",
                &waiting_parent_metadata(&parent)?, &now.0
            ])?;
            tx.execute(INSERT_PUBLICATION, &[&platform, &company, &project, &parent_id.as_str(), &publication_attempt(request)])?;
            insert_event_with_client(&mut tx, scope, publication_event(&parent, request, &child_ids, now)?)?;
            let receipt = PublicationReceipt::new(request, child_ids)?;
            tx.execute(INSERT_RECEIPT, &[&platform, &company, &project, &key, &serde_json::to_string(&receipt)?])?;
            tx.commit()?;
            Ok(receipt.into_child_ids())
        })
    }
}

pub(super) fn postgres_publication_exists(
    client: &mut impl GenericClient,
    scope: &AgentSchedulerScope,
    parent: &TaskId,
) -> Result<bool> {
    let (platform, company, project) = scope_params(scope);
    Ok(client.query_opt(SELECT_PUBLICATION, &[&platform, &company, &project, &parent.as_str()])?.is_some())
}

pub(super) fn claim_summary(
    tx: &mut Transaction<'_>, scope: &AgentSchedulerScope, task: &TaskId, run: &RunId,
) -> Result<()> {
    let (platform, company, project) = scope_params(scope);
    tx.execute(CLAIM_SUMMARY, &[
        &platform, &company, &project, &summary_key(task), &format!("run:{}", run.as_str()),
    ])?;
    Ok(())
}

pub(super) fn validate_summary_child_update(
    tx: &mut Transaction<'_>, scope: &AgentSchedulerScope, current: &StoredAgentTask,
    status: AgentTaskStatus, metadata: &Value,
) -> Result<()> {
    if changes_completion(current, status, metadata) {
        let (platform, company, project) = scope_params(scope);
        let parent = tx.query_opt(
            SELECT_CLAIMED_PARENT, &[&platform, &company, &project, &current.id.as_str()],
        )?.map(|row| row.try_get(0)).transpose()?;
        reject_claimed_child_update(parent)?;
    }
    Ok(())
}
