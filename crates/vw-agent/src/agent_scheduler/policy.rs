//! 执行任务看板的自动推进、失败重试与超时收敛策略，并返回可审计的处理结果。

use anyhow::{Context, Result, bail};
use serde_json::{Map, Value, json};
use vw_api_types::common::TimestampMs;
use vw_api_types::task::{TaskPoolWorkflowSnapshotDto, TaskPoolWorkflowStatusRoleDto};

use crate::agent_scheduler::domain::{RunId, TaskId};
use crate::agent_scheduler::events::AgentEventKind;
use crate::agent_scheduler::lifecycle_bridge::{LifecycleBridge, LifecycleStatusProjection};
use crate::agent_scheduler::state_machine::{AgentTaskStatus, ensure_transition};
use crate::agent_scheduler::store::events::AppendAgentEventRequest;
use crate::agent_scheduler::store::logs::AppendAgentTaskLogRequest;
use crate::agent_scheduler::store::runs::{AgentRunFinishStatus, StoredAgentRun};
use crate::agent_scheduler::store::tasks::{AgentTaskFilter, StoredAgentTask};
use crate::agent_scheduler::store::{AgentSchedulerStorageContext, AgentSchedulerStore};
use crate::task_lifecycle::DEFAULT_TASK_LIFECYCLE_WORKFLOW_ID;

const DEFAULT_FAILED_RETRY_MINUTES: u32 = 20;
const DEFAULT_FAILED_RETRY_MAX_ATTEMPTS: u32 = 10;
const DEFAULT_AUTO_PROMOTE_DELAY_MS: i64 = 30_000;
const DEFAULT_RUNNING_TIMEOUT_MS: i64 = 20 * 60_000;
const DEFAULT_MERGE_STALL_TIMEOUT_MS: i64 = 300_000;
const RUNNING_TIMEOUT_GRACE_MARKER: &str = "超时保活";
const OUTCOME_SUCCESS: &str = "success";
const OUTCOME_FAILURE: &str = "failure";
const OUTCOME_RETRY: &str = "retry";
const OUTCOME_BLOCKED: &str = "blocked";

#[derive(Debug, Clone, PartialEq, Eq)]
/// 自动推进、失败重试和各阶段超时阈值。
pub struct AgentSchedulerPolicyConfig {
    pub auto_promote_pool_tasks: bool,
    pub auto_promote_delay_ms: i64,
    pub failed_retry_minutes: u32,
    pub failed_retry_max_attempts: u32,
    pub running_timeout_ms: i64,
    pub merge_stall_timeout_ms: i64,
}

impl Default for AgentSchedulerPolicyConfig {
    fn default() -> Self {
        Self {
            auto_promote_pool_tasks: false,
            auto_promote_delay_ms: DEFAULT_AUTO_PROMOTE_DELAY_MS,
            failed_retry_minutes: DEFAULT_FAILED_RETRY_MINUTES,
            failed_retry_max_attempts: DEFAULT_FAILED_RETRY_MAX_ATTEMPTS,
            running_timeout_ms: DEFAULT_RUNNING_TIMEOUT_MS,
            merge_stall_timeout_ms: DEFAULT_MERGE_STALL_TIMEOUT_MS,
        }
    }
}

impl AgentSchedulerPolicyConfig {
    fn sanitized(&self) -> Self {
        Self {
            auto_promote_pool_tasks: self.auto_promote_pool_tasks,
            auto_promote_delay_ms: self.auto_promote_delay_ms.max(0),
            failed_retry_minutes: self.failed_retry_minutes.clamp(1, 1440),
            failed_retry_max_attempts: self.failed_retry_max_attempts.clamp(0, 10),
            running_timeout_ms: self.running_timeout_ms.max(1),
            merge_stall_timeout_ms: self.merge_stall_timeout_ms.max(1),
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
/// 单轮策略执行的扫描、转换、重试和告警统计。
pub struct AgentSchedulerPolicyOutcome {
    pub promoted_task_ids: Vec<TaskId>,
    pub retry_task_ids: Vec<TaskId>,
    pub timeout_grace_task_ids: Vec<TaskId>,
    pub timed_out_task_ids: Vec<TaskId>,
    pub review_task_ids: Vec<TaskId>,
    pub skipped_review_task_ids: Vec<TaskId>,
    pub merge_task_ids: Vec<TaskId>,
    pub blocked_task_ids: Vec<TaskId>,
}

/// 对项目任务快照执行确定性自动化策略的服务。
pub struct AgentSchedulerPolicyService<'a> {
    store: &'a AgentSchedulerStore,
}

impl<'a> AgentSchedulerPolicyService<'a> {
    pub fn new(store: &'a AgentSchedulerStore) -> Self {
        Self { store }
    }

    /// 应用一次项目策略；每项状态变化仍通过存储层状态机校验。
    pub fn apply_project_policy(
        &self,
        project_path: &(impl AgentSchedulerStorageContext + ?Sized),
        now_ms: TimestampMs,
        config: &AgentSchedulerPolicyConfig,
    ) -> Result<AgentSchedulerPolicyOutcome> {
        let config = config.sanitized();
        let workflow = self.store.load_task_board_workflow(project_path)?;
        let mut outcome = AgentSchedulerPolicyOutcome::default();
        let tasks = self.store.list_active_tasks(project_path, AgentTaskFilter::default())?;

        for task in tasks {
            if bool_metadata_field(&task.metadata, "task_board.deleted")
                || bool_metadata_field(&task.metadata, "task_board.archived")
            {
                continue;
            }

            match task.status {
                AgentTaskStatus::Draft => {
                    self.apply_pool_policy(project_path, &task, now_ms, &config, &mut outcome)?;
                }
                AgentTaskStatus::Assigned
                | AgentTaskStatus::Dispatched
                | AgentTaskStatus::Acknowledged
                | AgentTaskStatus::Running => {
                    self.apply_timeout_policy(
                        project_path,
                        &task,
                        now_ms,
                        &config,
                        workflow.as_ref(),
                        &mut outcome,
                    )?;
                }
                AgentTaskStatus::Failed => {
                    self.apply_retry_policy(
                        project_path,
                        &task,
                        now_ms,
                        &config,
                        workflow.as_ref(),
                        &mut outcome,
                    )?;
                }
                AgentTaskStatus::Submitted => {
                    self.apply_submitted_policy(
                        project_path,
                        &task,
                        now_ms,
                        &config,
                        workflow.as_ref(),
                        &mut outcome,
                    )?;
                }
                AgentTaskStatus::Queued
                | AgentTaskStatus::Active
                | AgentTaskStatus::Revision
                | AgentTaskStatus::Blocked
                | AgentTaskStatus::Paused
                | AgentTaskStatus::Completed
                | AgentTaskStatus::Cancelled
                | AgentTaskStatus::Archived => {}
            }
        }

        Ok(outcome)
    }

    fn apply_pool_policy(
        &self,
        project_path: &(impl AgentSchedulerStorageContext + ?Sized),
        task: &StoredAgentTask,
        now_ms: TimestampMs,
        config: &AgentSchedulerPolicyConfig,
        outcome: &mut AgentSchedulerPolicyOutcome,
    ) -> Result<()> {
        if !config.auto_promote_pool_tasks || !is_backlog_task(task) {
            return Ok(());
        }
        let delay_ms = integer_metadata_field(&task.metadata, "auto_promote_delay_ms")
            .unwrap_or(config.auto_promote_delay_ms)
            .max(0);
        if elapsed_since(now_ms, task.created_at_ms) < delay_ms {
            return Ok(());
        }

        let from_status_key = crate::agent_scheduler::task_board::task_board_status_key(task);
        let updated = self
            .store
            .transition_task_status(project_path, &task.id, AgentTaskStatus::Queued, now_ms)?
            .with_context(|| {
                format!("task disappeared during pool promotion: {}", task.id.as_str())
            })?;
        let to_status_key = crate::agent_scheduler::task_board::task_board_status_key(&updated);
        let message = "agent scheduler policy auto-promoted pool task to queued";
        self.append_log(
            project_path,
            &task.id,
            None,
            now_ms,
            Some(task.status),
            Some(AgentTaskStatus::Queued),
            Some(from_status_key.clone()),
            Some(to_status_key.clone()),
            message,
        )?;
        self.append_event(
            project_path,
            &updated,
            None,
            AgentEventKind::TaskResumed,
            Some(task.status),
            Some(AgentTaskStatus::Queued),
            Some(from_status_key),
            Some(to_status_key),
            message,
            json!({"policy_action": "auto_promote_pool"}),
            now_ms,
        )?;
        outcome.promoted_task_ids.push(task.id.clone());
        Ok(())
    }

    fn apply_retry_policy(
        &self,
        project_path: &(impl AgentSchedulerStorageContext + ?Sized),
        task: &StoredAgentTask,
        now_ms: TimestampMs,
        config: &AgentSchedulerPolicyConfig,
        workflow: Option<&TaskPoolWorkflowSnapshotDto>,
        outcome: &mut AgentSchedulerPolicyOutcome,
    ) -> Result<()> {
        if config.failed_retry_max_attempts == 0 {
            return Ok(());
        }

        let latest_run = self.store.load_latest_run_for_task(project_path, &task.id)?;
        let retry_count = latest_run
            .as_ref()
            .map(|run| run.attempt.saturating_sub(1))
            .or_else(|| u32_metadata_field(&task.metadata, "retry_count"))
            .unwrap_or_default();
        if retry_count >= config.failed_retry_max_attempts {
            return Ok(());
        }

        let error = latest_run
            .as_ref()
            .and_then(|run| run.error_message.as_deref())
            .and_then(non_empty_str)
            .map(str::to_string)
            .or_else(|| string_metadata_field(&task.metadata, "last_error"));
        let wait_ms = if error.as_deref().is_some_and(is_model_response_timeout_error) {
            minutes_to_ms(retry_backoff_minutes(config.failed_retry_minutes, retry_count))
        } else {
            0
        };
        let anchor =
            retry_anchor(&task.metadata, latest_run.as_ref()).unwrap_or(task.updated_at_ms);
        if elapsed_since(now_ms, anchor) < wait_ms {
            return Ok(());
        }

        let message = match error {
            Some(error) => format!(
                "agent scheduler policy queued failed task for retry after attempt {}: {}",
                retry_count + 1,
                error
            ),
            None => format!(
                "agent scheduler policy queued failed task for retry after attempt {}",
                retry_count + 1
            ),
        };
        if self
            .transition_task_with_lifecycle_outcome(
                project_path,
                task,
                workflow,
                OUTCOME_RETRY,
                now_ms,
                message,
                AgentEventKind::TaskResumed,
                json!({
                    "policy_action": "retry_failed_task",
                    "retry_count": retry_count,
                    "max_attempts": config.failed_retry_max_attempts,
                }),
            )?
            .is_some()
        {
            outcome.retry_task_ids.push(task.id.clone());
        }
        Ok(())
    }

    fn apply_timeout_policy(
        &self,
        project_path: &(impl AgentSchedulerStorageContext + ?Sized),
        task: &StoredAgentTask,
        now_ms: TimestampMs,
        config: &AgentSchedulerPolicyConfig,
        workflow: Option<&TaskPoolWorkflowSnapshotDto>,
        outcome: &mut AgentSchedulerPolicyOutcome,
    ) -> Result<()> {
        let latest_run = self.store.load_latest_run_for_task(project_path, &task.id)?;
        let anchor = running_timeout_anchor(&task.metadata, latest_run.as_ref(), task);
        if elapsed_since(now_ms, anchor) < config.running_timeout_ms {
            return Ok(());
        }

        let logs = self.store.list_task_logs(project_path, &task.id)?;
        if !logs.iter().rev().any(|log| {
            log.timestamp_ms >= anchor && log.message.contains(RUNNING_TIMEOUT_GRACE_MARKER)
        }) {
            let message = format!(
                "{RUNNING_TIMEOUT_GRACE_MARKER}: {} exceeded {} ms, keeping one policy grace window",
                task.status.as_str(),
                config.running_timeout_ms
            );
            self.append_log(
                project_path,
                &task.id,
                latest_run.as_ref().map(|run| &run.id),
                now_ms,
                None,
                None,
                None,
                None,
                message.clone(),
            )?;
            self.append_event(
                project_path,
                task,
                latest_run.as_ref(),
                AgentEventKind::HumanIntervened,
                Some(task.status),
                Some(task.status),
                None,
                None,
                message,
                json!({"policy_action": "running_timeout_grace"}),
                now_ms,
            )?;
            outcome.timeout_grace_task_ids.push(task.id.clone());
            return Ok(());
        }

        let message = format!(
            "{} exceeded {} ms for two policy windows, marking failed",
            task.status.as_str(),
            config.running_timeout_ms
        );
        if let Some(run) = latest_run.as_ref().filter(|run| run.finished_at_ms.is_none()) {
            self.store.finish_run(
                project_path,
                &run.id,
                AgentRunFinishStatus::TimedOut,
                now_ms,
                Some(message.clone()),
            )?;
        }
        if self
            .transition_task_with_lifecycle_outcome(
                project_path,
                task,
                workflow,
                OUTCOME_FAILURE,
                now_ms,
                message,
                AgentEventKind::RunFailed,
                json!({"policy_action": "running_timeout"}),
            )?
            .is_some()
        {
            outcome.timed_out_task_ids.push(task.id.clone());
        }
        Ok(())
    }

    fn apply_submitted_policy(
        &self,
        project_path: &(impl AgentSchedulerStorageContext + ?Sized),
        task: &StoredAgentTask,
        now_ms: TimestampMs,
        config: &AgentSchedulerPolicyConfig,
        workflow: Option<&TaskPoolWorkflowSnapshotDto>,
        outcome: &mut AgentSchedulerPolicyOutcome,
    ) -> Result<()> {
        let role = status_role_from_metadata(&task.metadata);
        let status_key = crate::agent_scheduler::task_board::task_board_status_key(task);
        let allows_git_merge = allows_git_merge_policy(task, workflow);
        if is_merging_projection(role, &status_key) {
            if !allows_git_merge {
                self.transition_task_with_lifecycle_outcome(
                    project_path,
                    task,
                    workflow,
                    OUTCOME_SUCCESS,
                    now_ms,
                    "agent scheduler policy completed non-development merge stage without git merge",
                    AgentEventKind::RunCompleted,
                    json!({
                        "policy_action": "complete_non_development_merge_stage",
                        "workflow_id": workflow_id_for_policy(task, workflow),
                    }),
                )?;
                return Ok(());
            }
            if self.apply_merge_stall_policy(
                project_path,
                task,
                now_ms,
                config,
                workflow,
                outcome,
            )? {
                return Ok(());
            }
            self.append_log(
                project_path,
                &task.id,
                None,
                now_ms,
                Some(task.status),
                Some(task.status),
                Some(status_key.clone()),
                Some(status_key.clone()),
                "agent scheduler policy requested merge for submitted task",
            )?;
            outcome.merge_task_ids.push(task.id.clone());
            return Ok(());
        }

        if !is_review_ready_projection(role, &status_key) {
            return Ok(());
        }

        if !allows_git_merge {
            self.transition_task_with_lifecycle_outcome(
                project_path,
                task,
                workflow,
                OUTCOME_SUCCESS,
                now_ms,
                "agent scheduler policy completed non-development submitted task without git merge",
                AgentEventKind::RunCompleted,
                json!({
                    "policy_action": "complete_non_development_submitted_task",
                    "workflow_id": workflow_id_for_policy(task, workflow),
                }),
            )?;
            return Ok(());
        }

        Ok(())
    }

    fn apply_merge_stall_policy(
        &self,
        project_path: &(impl AgentSchedulerStorageContext + ?Sized),
        task: &StoredAgentTask,
        now_ms: TimestampMs,
        config: &AgentSchedulerPolicyConfig,
        workflow: Option<&TaskPoolWorkflowSnapshotDto>,
        outcome: &mut AgentSchedulerPolicyOutcome,
    ) -> Result<bool> {
        let anchor = timestamp_metadata_field(&task.metadata, "last_active_at_ms")
            .unwrap_or(task.updated_at_ms);
        if elapsed_since(now_ms, anchor) < config.merge_stall_timeout_ms {
            return Ok(false);
        }
        let target_branch = string_metadata_field(&task.metadata, "merge_target_branch")
            .unwrap_or_else(|| "unknown".to_string());
        let selected_worktree = string_metadata_field(&task.metadata, "selected_worktree_path")
            .unwrap_or_else(|| "unknown".to_string());
        let message = format!(
            "agent scheduler policy merge stall detected: inactive_ms={} worktree={} target={}",
            elapsed_since(now_ms, anchor),
            selected_worktree,
            target_branch
        );
        if self
            .transition_task_with_lifecycle_outcome(
                project_path,
                task,
                workflow,
                OUTCOME_BLOCKED,
                now_ms,
                message,
                AgentEventKind::TaskBlocked,
                json!({
                    "policy_action": "merge_stall",
                    "target_branch": target_branch,
                    "selected_worktree": selected_worktree,
                }),
            )?
            .is_some()
        {
            outcome.blocked_task_ids.push(task.id.clone());
            return Ok(true);
        }
        Ok(false)
    }

    fn transition_task_with_lifecycle_outcome(
        &self,
        project_path: &(impl AgentSchedulerStorageContext + ?Sized),
        task: &StoredAgentTask,
        workflow: Option<&TaskPoolWorkflowSnapshotDto>,
        lifecycle_outcome: &str,
        now_ms: TimestampMs,
        message: impl Into<String>,
        event_kind: AgentEventKind,
        payload: Value,
    ) -> Result<Option<StoredAgentTask>> {
        let Some((from_status_key, target)) = self.resolve_lifecycle_outcome_target(
            project_path,
            task,
            workflow,
            lifecycle_outcome,
            now_ms,
        )?
        else {
            return Ok(None);
        };
        let to_status = target.runtime_status;
        ensure_transition(task.kind, task.status, to_status)?;
        let to_status_key = target.status_key.clone();
        let mut metadata = object_metadata(task.metadata.clone())?;
        set_read_model_status(&mut metadata, &target);
        if to_status == AgentTaskStatus::Failed {
            metadata.insert("last_active_at_ms".to_string(), json!(now_ms.0));
        }

        let updated = self
            .store
            .transition_task_status_with_metadata(
                project_path,
                &task.id,
                to_status,
                Value::Object(metadata),
                now_ms,
            )?
            .with_context(|| {
                format!("task disappeared during policy transition: {}", task.id.as_str())
            })?;
        let message = message.into();
        self.append_log(
            project_path,
            &task.id,
            None,
            now_ms,
            Some(task.status),
            Some(to_status),
            Some(from_status_key.clone()),
            Some(to_status_key.clone()),
            message.clone(),
        )?;
        self.append_event(
            project_path,
            &updated,
            None,
            event_kind,
            Some(task.status),
            Some(to_status),
            Some(from_status_key),
            Some(to_status_key),
            message,
            payload,
            now_ms,
        )?;
        Ok(Some(updated))
    }

    fn resolve_lifecycle_outcome_target(
        &self,
        project_path: &(impl AgentSchedulerStorageContext + ?Sized),
        task: &StoredAgentTask,
        workflow: Option<&TaskPoolWorkflowSnapshotDto>,
        lifecycle_outcome: &str,
        now_ms: TimestampMs,
    ) -> Result<Option<(String, LifecycleStatusProjection)>> {
        let from_status_key = crate::agent_scheduler::task_board::task_board_status_key(task);
        let Some(workflow) = workflow else {
            self.append_lifecycle_outcome_skip_log(
                project_path,
                &task.id,
                task.status,
                from_status_key.clone(),
                lifecycle_outcome,
                "task board workflow is required",
                now_ms,
            )?;
            return Ok(None);
        };
        match LifecycleBridge::new(workflow).resolve_outcome(&from_status_key, lifecycle_outcome) {
            Ok(target) => Ok(Some((from_status_key, target))),
            Err(error) => {
                self.append_lifecycle_outcome_skip_log(
                    project_path,
                    &task.id,
                    task.status,
                    from_status_key,
                    lifecycle_outcome,
                    error.to_string(),
                    now_ms,
                )?;
                Ok(None)
            }
        }
    }

    fn append_lifecycle_outcome_skip_log(
        &self,
        project_path: &(impl AgentSchedulerStorageContext + ?Sized),
        task_id: &TaskId,
        status: AgentTaskStatus,
        status_key: String,
        lifecycle_outcome: &str,
        reason: impl Into<String>,
        now_ms: TimestampMs,
    ) -> Result<()> {
        self.append_log(
            project_path,
            task_id,
            None,
            now_ms,
            Some(status),
            Some(status),
            Some(status_key.clone()),
            Some(status_key),
            format!(
                "agent scheduler policy skipped lifecycle outcome `{}`: {}",
                lifecycle_outcome,
                reason.into()
            ),
        )
    }

    fn append_log(
        &self,
        project_path: &(impl AgentSchedulerStorageContext + ?Sized),
        task_id: &TaskId,
        run_id: Option<&RunId>,
        timestamp_ms: TimestampMs,
        from_status: Option<AgentTaskStatus>,
        to_status: Option<AgentTaskStatus>,
        from_status_key: Option<String>,
        to_status_key: Option<String>,
        message: impl Into<String>,
    ) -> Result<()> {
        self.store.append_task_log(
            project_path,
            AppendAgentTaskLogRequest {
                task_id: task_id.clone(),
                run_id: run_id.cloned(),
                timestamp_ms,
                from_status,
                to_status,
                from_status_key,
                to_status_key,
                message: message.into(),
            },
        )?;
        Ok(())
    }

    fn append_event(
        &self,
        project_path: &(impl AgentSchedulerStorageContext + ?Sized),
        task: &StoredAgentTask,
        run: Option<&StoredAgentRun>,
        kind: AgentEventKind,
        from_status: Option<AgentTaskStatus>,
        to_status: Option<AgentTaskStatus>,
        from_status_key: Option<String>,
        to_status_key: Option<String>,
        message: impl Into<String>,
        payload: Value,
        created_at_ms: TimestampMs,
    ) -> Result<()> {
        self.store.append_event(
            project_path,
            AppendAgentEventRequest {
                task_id: Some(task.id.clone()),
                run_id: run.map(|run| run.id.clone()),
                worker_id: task.assigned_to.clone(),
                team_id: task.team_id.clone(),
                kind,
                from_status,
                to_status,
                from_status_key,
                to_status_key,
                message: message.into(),
                payload,
                created_at_ms,
            },
        )?;
        Ok(())
    }
}

fn is_backlog_task(task: &StoredAgentTask) -> bool {
    let status_key = crate::agent_scheduler::task_board::task_board_status_key(task);
    task.status == AgentTaskStatus::Draft
        && (matches!(
            status_role_from_metadata(&task.metadata),
            Some(TaskPoolWorkflowStatusRoleDto::Backlog)
        ) || matches!(status_key.as_str(), "pool" | "backlog" | "draft"))
}

fn is_review_ready_projection(
    role: Option<TaskPoolWorkflowStatusRoleDto>,
    status_key: &str,
) -> bool {
    matches!(role, Some(TaskPoolWorkflowStatusRoleDto::ReviewReady))
        || matches!(status_key, "code_complete" | "review_ready")
}

fn is_merging_projection(role: Option<TaskPoolWorkflowStatusRoleDto>, status_key: &str) -> bool {
    matches!(role, Some(TaskPoolWorkflowStatusRoleDto::Merging))
        || matches!(status_key, "pr_submitted" | "merging")
}

fn allows_git_merge_policy(
    task: &StoredAgentTask,
    workflow: Option<&TaskPoolWorkflowSnapshotDto>,
) -> bool {
    workflow_id_for_policy(task, workflow)
        .as_deref()
        .map_or(true, |workflow_id| workflow_id == DEFAULT_TASK_LIFECYCLE_WORKFLOW_ID)
}

fn workflow_id_for_policy(
    task: &StoredAgentTask,
    workflow: Option<&TaskPoolWorkflowSnapshotDto>,
) -> Option<String> {
    string_metadata_field(&task.metadata, "task_board.workflow_id")
        .or_else(|| workflow.and_then(|workflow| non_empty_str(&workflow.id).map(str::to_string)))
}

fn retry_anchor(metadata: &Value, latest_run: Option<&StoredAgentRun>) -> Option<TimestampMs> {
    latest_run
        .and_then(|run| run.finished_at_ms)
        .or_else(|| timestamp_metadata_field(metadata, "last_active_at_ms"))
}

fn running_timeout_anchor(
    metadata: &Value,
    latest_run: Option<&StoredAgentRun>,
    task: &StoredAgentTask,
) -> TimestampMs {
    latest_run
        .and_then(|run| run.last_heartbeat_at_ms.or(run.started_at_ms).or(run.lease_acquired_at_ms))
        .or_else(|| timestamp_metadata_field(metadata, "last_active_at_ms"))
        .unwrap_or(task.updated_at_ms)
}

fn elapsed_since(now_ms: TimestampMs, anchor: TimestampMs) -> i64 {
    now_ms.0.saturating_sub(anchor.0)
}

fn minutes_to_ms(minutes: u32) -> i64 {
    i64::from(minutes.max(1)).saturating_mul(60_000)
}

fn retry_backoff_minutes(base_minutes: u32, retry_count: u32) -> u32 {
    let base = u64::from(base_minutes.clamp(1, 1440));
    let factor = 1_u64.checked_shl(retry_count.min(31)).unwrap_or(u64::MAX);
    base.saturating_mul(factor).min(1440) as u32
}

fn is_model_response_timeout_error(error: &str) -> bool {
    let normalized = error.to_ascii_lowercase();
    normalized.contains("模型响应超时")
        || normalized.contains("llm response timed out")
        || normalized.contains("llm response timeout")
        || normalized.contains("model response timed out")
        || normalized.contains("model response timeout")
        || normalized.contains("request timed out while waiting for the model")
        || normalized.contains("gateway stream idle timeout")
        || normalized.contains("gateway 响应超时")
}

fn set_read_model_status(metadata: &mut Map<String, Value>, target: &LifecycleStatusProjection) {
    metadata.insert("task_board.status_key".to_string(), json!(target.status_key.clone()));
    metadata
        .insert("task_board.status_role".to_string(), json!(status_role_key(target.status_role)));
}

fn status_role_key(role: TaskPoolWorkflowStatusRoleDto) -> &'static str {
    match role {
        TaskPoolWorkflowStatusRoleDto::Backlog => "backlog",
        TaskPoolWorkflowStatusRoleDto::Queued => "queued",
        TaskPoolWorkflowStatusRoleDto::Planning => "planning",
        TaskPoolWorkflowStatusRoleDto::Executing => "executing",
        TaskPoolWorkflowStatusRoleDto::Failed => "failed",
        TaskPoolWorkflowStatusRoleDto::Paused => "paused",
        TaskPoolWorkflowStatusRoleDto::ReviewReady => "review_ready",
        TaskPoolWorkflowStatusRoleDto::Review => "review",
        TaskPoolWorkflowStatusRoleDto::Merging => "merging",
        TaskPoolWorkflowStatusRoleDto::Done => "done",
        TaskPoolWorkflowStatusRoleDto::Archived => "archived",
        TaskPoolWorkflowStatusRoleDto::Custom => "custom",
    }
}

fn status_role_from_metadata(metadata: &Value) -> Option<TaskPoolWorkflowStatusRoleDto> {
    match string_metadata_field(metadata, "task_board.status_role")?.as_str() {
        "backlog" => Some(TaskPoolWorkflowStatusRoleDto::Backlog),
        "queued" => Some(TaskPoolWorkflowStatusRoleDto::Queued),
        "planning" => Some(TaskPoolWorkflowStatusRoleDto::Planning),
        "executing" => Some(TaskPoolWorkflowStatusRoleDto::Executing),
        "failed" => Some(TaskPoolWorkflowStatusRoleDto::Failed),
        "paused" => Some(TaskPoolWorkflowStatusRoleDto::Paused),
        "review_ready" => Some(TaskPoolWorkflowStatusRoleDto::ReviewReady),
        "review" => Some(TaskPoolWorkflowStatusRoleDto::Review),
        "merging" => Some(TaskPoolWorkflowStatusRoleDto::Merging),
        "done" => Some(TaskPoolWorkflowStatusRoleDto::Done),
        "archived" => Some(TaskPoolWorkflowStatusRoleDto::Archived),
        "custom" => Some(TaskPoolWorkflowStatusRoleDto::Custom),
        _ => None,
    }
}

fn object_metadata(value: Value) -> Result<Map<String, Value>> {
    match value {
        Value::Object(map) => Ok(map),
        Value::Null => Ok(Map::new()),
        _ => bail!("task metadata must be a JSON object"),
    }
}

fn bool_metadata_field(metadata: &Value, key: &str) -> bool {
    metadata.get(key).and_then(Value::as_bool).unwrap_or(false)
}

fn string_metadata_field(metadata: &Value, key: &str) -> Option<String> {
    metadata.get(key).and_then(Value::as_str).and_then(non_empty_str).map(str::to_string)
}

fn integer_metadata_field(metadata: &Value, key: &str) -> Option<i64> {
    metadata.get(key).and_then(Value::as_i64)
}

fn u32_metadata_field(metadata: &Value, key: &str) -> Option<u32> {
    metadata.get(key).and_then(Value::as_u64).and_then(|value| u32::try_from(value).ok())
}

fn timestamp_metadata_field(metadata: &Value, key: &str) -> Option<TimestampMs> {
    integer_metadata_field(metadata, key).map(TimestampMs)
}

fn non_empty_str(value: &str) -> Option<&str> {
    let trimmed = value.trim();
    (!trimmed.is_empty()).then_some(trimmed)
}
