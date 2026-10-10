//! 回收过期运行与资源租约，并按策略暂停或失败任务。

use anyhow::{Context, Result};
use serde_json::{Value, json};
use std::collections::HashSet;
use vw_api_types::common::TimestampMs;

use crate::agent_scheduler::domain::{ResourceLeaseId, RunId};
use crate::agent_scheduler::events::AgentEventKind;
use crate::agent_scheduler::state_machine::AgentTaskStatus;
use crate::agent_scheduler::store::events::AppendAgentEventRequest;
use crate::agent_scheduler::store::outbox::EnqueueOutboxMessageRequest;
use crate::agent_scheduler::store::resources::StoredResourceLease;
use crate::agent_scheduler::store::runs::{AgentRunFinishStatus, StoredAgentRun};
use crate::agent_scheduler::store::tasks::StoredAgentTask;
use crate::agent_scheduler::store::{AgentSchedulerStorageContext, AgentSchedulerStore};

pub const DEFAULT_RECOVERY_GRACE_MS: i64 = 5 * 60 * 1_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// 过期运行对应任务的收敛方式。
pub enum StaleRunResolution {
    FailTask,
    PauseTask,
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// 恢复宽限期、资源污染和空闲节点休眠策略。
pub struct RecoveryPolicy {
    pub stale_run_grace_ms: i64,
    pub stale_run_resolution: StaleRunResolution,
    pub taint_released_resources: bool,
    pub admin_notification_target: Option<String>,
    pub admin_recipient_id: Option<String>,
}

impl Default for RecoveryPolicy {
    fn default() -> Self {
        Self {
            stale_run_grace_ms: DEFAULT_RECOVERY_GRACE_MS,
            stale_run_resolution: StaleRunResolution::FailTask,
            taint_released_resources: true,
            admin_notification_target: None,
            admin_recipient_id: None,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
/// 单轮恢复处理的运行、资源、节点和通知统计。
pub struct RecoveryOutcome {
    pub graced_run_ids: Vec<RunId>,
    pub recovered_run_ids: Vec<RunId>,
    pub released_resource_lease_ids: Vec<ResourceLeaseId>,
    pub tainted_resource_lease_ids: Vec<ResourceLeaseId>,
    pub outbox_message_ids: Vec<String>,
}

/// 对过期运行、遗留租约和空闲节点执行补偿动作的恢复引擎。
pub struct RecoveryEngine<'a> {
    store: &'a AgentSchedulerStore,
    policy: RecoveryPolicy,
}

impl<'a> RecoveryEngine<'a> {
    pub fn new(store: &'a AgentSchedulerStore) -> Self {
        Self::with_policy(store, RecoveryPolicy::default())
    }

    pub fn with_policy(
        store: &'a AgentSchedulerStore,
        policy: RecoveryPolicy,
    ) -> Self {
        Self { store, policy }
    }

    /// 以给定时间为一致快照执行一轮恢复，并返回所有补偿计数。
    pub async fn reconcile_once(
        &self,
        project_path: &(impl AgentSchedulerStorageContext + ?Sized),
        now_ms: TimestampMs,
    ) -> Result<RecoveryOutcome> {
        let mut outcome = RecoveryOutcome::default();
        let stale_runs = self.store.list_stale_runs(project_path, now_ms)?;
        let mut graced_run_ids = HashSet::new();

        for run in stale_runs {
            if self.run_has_recovery_grace(project_path, &run)? {
                self.recover_stale_run(project_path, &run, now_ms, &mut outcome)?;
            } else {
                self.record_recovery_grace(project_path, &run, now_ms, &mut outcome)?;
                graced_run_ids.insert(run.id.as_str().to_string());
            }
        }

        self.release_stale_resources(project_path, now_ms, &graced_run_ids, &mut outcome)?;
        Ok(outcome)
    }

    fn run_has_recovery_grace(
        &self,
        project_path: &(impl AgentSchedulerStorageContext + ?Sized),
        run: &StoredAgentRun,
    ) -> Result<bool> {
        let events = self.store.list_events_after(project_path, None, 10_000)?;
        Ok(events.iter().any(|event| {
            event.kind == AgentEventKind::RunRecovered
                && event.run_id.as_ref() == Some(&run.id)
                && event.payload["phase"] == "grace"
        }))
    }

    fn record_recovery_grace(
        &self,
        project_path: &(impl AgentSchedulerStorageContext + ?Sized),
        run: &StoredAgentRun,
        now_ms: TimestampMs,
        outcome: &mut RecoveryOutcome,
    ) -> Result<()> {
        let lease_expires_at_ms = grace_expires_at(now_ms, self.policy.stale_run_grace_ms)?;
        let refreshed = self
            .store
            .heartbeat_run(project_path, &run.id, now_ms, lease_expires_at_ms)?
            .with_context(|| format!("stale run disappeared before grace: {}", run.id.as_str()))?;
        self.append_recovery_event(
            project_path,
            &refreshed,
            None,
            "grace",
            AgentTaskStatus::Running,
            format!(
                "stale run {} granted recovery grace until {}",
                refreshed.id.as_str(),
                lease_expires_at_ms.0
            ),
            json!({
                "phase": "grace",
                "run_id": refreshed.id.as_str(),
                "task_id": refreshed.task_id.as_str(),
                "lease_expires_at_ms": lease_expires_at_ms.0,
            }),
            now_ms,
        )?;
        self.enqueue_admin_notification(
            project_path,
            "run-grace",
            refreshed.id.as_str(),
            format!(
                "[Recovery] Run {} is stale; waiting one grace cycle before failing the task.",
                refreshed.id.as_str()
            ),
            json!({
                "kind": "recovery",
                "phase": "grace",
                "run_id": refreshed.id.as_str(),
                "task_id": refreshed.task_id.as_str(),
            }),
            now_ms,
            outcome,
        )?;
        outcome.graced_run_ids.push(refreshed.id);
        Ok(())
    }

    fn recover_stale_run(
        &self,
        project_path: &(impl AgentSchedulerStorageContext + ?Sized),
        run: &StoredAgentRun,
        now_ms: TimestampMs,
        outcome: &mut RecoveryOutcome,
    ) -> Result<()> {
        let task = self
            .store
            .load_task(project_path, &run.task_id)?
            .with_context(|| format!("stale run task missing: {}", run.task_id.as_str()))?;
        let to_status = self.policy.stale_run_resolution.task_status();
        let reason = format!(
            "run {} stale for two recovery cycles; recovery marked task {}",
            run.id.as_str(),
            to_status.as_str()
        );
        self.store
            .transition_task_status(project_path, &task.id, to_status, now_ms)?
            .with_context(|| format!("stale run task disappeared: {}", task.id.as_str()))?;
        let finished = self
            .store
            .finish_run(
                project_path,
                &run.id,
                AgentRunFinishStatus::Failed,
                now_ms,
                Some(reason.clone()),
            )?
            .with_context(|| format!("stale run disappeared before finish: {}", run.id.as_str()))?;
        self.append_recovery_event(
            project_path,
            &finished,
            Some(&task),
            "resolved",
            to_status,
            reason.clone(),
            json!({
                "phase": "resolved",
                "run_id": finished.id.as_str(),
                "task_id": finished.task_id.as_str(),
                "resolution": self.policy.stale_run_resolution.as_str(),
                "resource_recovery_requested": true,
            }),
            now_ms,
        )?;
        self.enqueue_admin_notification(
            project_path,
            "run-resolved",
            finished.id.as_str(),
            format!("[Recovery] {reason}"),
            json!({
                "kind": "recovery",
                "phase": "resolved",
                "run_id": finished.id.as_str(),
                "task_id": finished.task_id.as_str(),
                "resolution": self.policy.stale_run_resolution.as_str(),
            }),
            now_ms,
            outcome,
        )?;
        outcome.recovered_run_ids.push(finished.id);
        Ok(())
    }

    fn release_stale_resources(
        &self,
        project_path: &(impl AgentSchedulerStorageContext + ?Sized),
        now_ms: TimestampMs,
        graced_run_ids: &HashSet<String>,
        outcome: &mut RecoveryOutcome,
    ) -> Result<()> {
        let stale_leases = self.store.list_stale_resource_leases(project_path, now_ms)?;
        for lease in stale_leases {
            if graced_run_ids.contains(lease.holder_id.as_str()) {
                continue;
            }
            self.release_stale_resource(project_path, lease, now_ms, outcome)?;
        }
        Ok(())
    }

    fn release_stale_resource(
        &self,
        project_path: &(impl AgentSchedulerStorageContext + ?Sized),
        lease: StoredResourceLease,
        now_ms: TimestampMs,
        outcome: &mut RecoveryOutcome,
    ) -> Result<()> {
        if self.policy.taint_released_resources {
            let taint_reason =
                format!("lease {} stale at recovery time {}", lease.id.as_str(), now_ms.0);
            if let Some(tainted) = self.store.mark_resource_tainted(
                project_path,
                lease.resource_kind,
                &lease.resource_id,
                taint_reason,
            )? {
                outcome.tainted_resource_lease_ids.push(tainted.id);
            }
        }

        let released =
            self.store.release_resource_lease(project_path, &lease.id, &lease.holder_id, now_ms)?;
        if let Some(released) = released {
            self.store.append_event(
                project_path,
                AppendAgentEventRequest {
                    task_id: None,
                    run_id: RunId::new(released.holder_id.as_str().to_string()).ok(),
                    worker_id: None,
                    team_id: None,
                    kind: AgentEventKind::ResourceReleased,
                    from_status: None,
                    to_status: None,
                    from_status_key: None,
                    to_status_key: None,
                    message: format!("released stale resource lease {}", released.id.as_str()),
                    payload: json!({
                        "lease_id": released.id.as_str(),
                        "resource_kind": released.resource_kind.as_str(),
                        "resource_id": released.resource_id.as_str(),
                        "holder_id": released.holder_id.as_str(),
                        "tainted": released.taint_reason.is_some(),
                    }),
                    created_at_ms: now_ms,
                },
            )?;
            self.enqueue_admin_notification(
                project_path,
                "resource-released",
                released.id.as_str(),
                format!(
                    "[Recovery] Released stale resource lease {} for {}.",
                    released.id.as_str(),
                    released.resource_id.as_str()
                ),
                json!({
                    "kind": "recovery",
                    "phase": "resource_released",
                    "lease_id": released.id.as_str(),
                    "resource_id": released.resource_id.as_str(),
                    "holder_id": released.holder_id.as_str(),
                }),
                now_ms,
                outcome,
            )?;
            outcome.released_resource_lease_ids.push(released.id);
        }
        Ok(())
    }




    #[allow(clippy::too_many_arguments)]
    fn append_recovery_event(
        &self,
        project_path: &(impl AgentSchedulerStorageContext + ?Sized),
        run: &StoredAgentRun,
        task: Option<&StoredAgentTask>,
        phase: &str,
        to_status: AgentTaskStatus,
        message: String,
        payload: Value,
        now_ms: TimestampMs,
    ) -> Result<()> {
        self.store.append_event(
            project_path,
            AppendAgentEventRequest {
                task_id: Some(run.task_id.clone()),
                run_id: Some(run.id.clone()),
                worker_id: None,
                team_id: task.and_then(|task| task.team_id.clone()),
                kind: AgentEventKind::RunRecovered,
                from_status: task.map(|task| task.status),
                to_status: Some(to_status),
                from_status_key: None,
                to_status_key: None,
                message,
                payload: with_phase(payload, phase),
                created_at_ms: now_ms,
            },
        )?;
        Ok(())
    }

    fn enqueue_admin_notification(
        &self,
        project_path: &(impl AgentSchedulerStorageContext + ?Sized),
        phase: &str,
        subject_id: &str,
        body: String,
        payload: Value,
        now_ms: TimestampMs,
        outcome: &mut RecoveryOutcome,
    ) -> Result<()> {
        let Some(target) = &self.policy.admin_notification_target else {
            return Ok(());
        };
        let message_id = format!("outbox-recovery-{phase}-{subject_id}");
        let message = self.store.enqueue_outbox_message(
            project_path,
            EnqueueOutboxMessageRequest {
                message_id,
                channel: "local".to_string(),
                target: target.clone(),
                recipient_id: self.policy.admin_recipient_id.clone(),
                body,
                idempotency_key: format!("recovery:{phase}:{subject_id}"),
                payload,
                created_at_ms: now_ms,
                next_attempt_at_ms: None,
            },
        )?;
        outcome.outbox_message_ids.push(message.id);
        Ok(())
    }
}

impl StaleRunResolution {
    fn task_status(self) -> AgentTaskStatus {
        match self {
            Self::FailTask => AgentTaskStatus::Failed,
            Self::PauseTask => AgentTaskStatus::Paused,
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::FailTask => "fail_task",
            Self::PauseTask => "pause_task",
        }
    }
}

fn grace_expires_at(now_ms: TimestampMs, grace_ms: i64) -> Result<TimestampMs> {
    let grace_ms = grace_ms.max(1);
    Ok(TimestampMs(now_ms.0.checked_add(grace_ms).context("recovery grace timestamp overflow")?))
}

fn with_phase(mut payload: Value, phase: &str) -> Value {
    payload["phase"] = Value::String(phase.to_string());
    payload
}
