//! 逻辑工位池：保留容量与租约管理，目录资源由显式工具绑定。

use crate::app::agent::project;
use anyhow::{Context, Result, bail};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};
use vw_api_types::id::{ProjectId, TaskId};
use vw_api_types::task::{
    TaskPoolWorktreeAcquireResponse, TaskPoolWorktreeMaintenanceResponse,
    TaskPoolWorktreeMergeLockDto, TaskPoolWorktreeMergeLockResponse,
    TaskPoolWorktreeRecycleResponse, TaskPoolWorktreeReleaseResponse, TaskPoolWorktreeSlotDto,
    TaskPoolWorktreeSlotRecycleResponse, TaskPoolWorktreeSlotStateDto,
    TaskPoolWorktreeSnapshotResponse,
};

static SLOT_SEQUENCE: AtomicU64 = AtomicU64::new(0);

const MERGE_LOCK_STALE_AFTER_MS: u64 = 30 * 60 * 1000;

#[derive(Debug, Clone)]
struct MergeLockEntry {
    task_id: TaskId,
    locked_at_ms: u64,
}

fn now_epoch_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or(0)
}

#[derive(Clone, Default)]
pub struct TaskWorktreePoolService {
    pools: Arc<Mutex<HashMap<String, ProjectPoolState>>>,
    pool_root_override: Option<PathBuf>,
}

impl TaskWorktreePoolService {
    pub fn new() -> Self {
        Self::default()
    }

    #[cfg(test)]
    pub(crate) fn with_pool_root_for_tests(pool_root: PathBuf) -> Self {
        Self { pool_root_override: Some(pool_root), ..Self::default() }
    }

    fn with_pool<T>(
        &self,
        project: &project::Info,
        operation: impl FnOnce(&mut ProjectPoolState) -> Result<T>,
    ) -> Result<T> {
        if project.id.trim().is_empty() {
            bail!("missing project id");
        }
        let root =
            std::fs::canonicalize(&project.worktree).context("project directory does not exist")?;
        if !root.is_dir() {
            bail!("project path is not a directory");
        }
        let mut pools = self.pools.lock().expect("task pool lock poisoned");
        let pool = pools.entry(project.id.clone()).or_insert_with(|| ProjectPoolState {
            repo_root: root.to_string_lossy().into_owned(),
            pool_root: self.pool_root_override.clone().unwrap_or_default(),
            base_branch: String::new(),
            slots: Vec::new(),
            merge_target_locks: HashMap::new(),
        });
        if pool.repo_root != root.to_string_lossy() {
            bail!("project directory changed while pool is active");
        }
        operation(pool)
    }

    async fn ensure_project_pool(&self, project: &project::Info) -> Result<String> {
        self.with_pool(project, |_| Ok(()))?;
        Ok(project.id.clone())
    }

    pub async fn snapshot(
        &self,
        project: &project::Info,
    ) -> Result<TaskPoolWorktreeSnapshotResponse> {
        self.with_pool(project, |pool| Ok(snapshot_from_pool(&project.id, pool)))
    }

    pub async fn maintain(
        &self,
        project: &project::Info,
        desired_capacity: u32,
    ) -> Result<TaskPoolWorktreeMaintenanceResponse> {
        self.with_pool(project, |pool| {
            let target = desired_capacity.max(1);
            while pool.count(TaskPoolWorktreeSlotStateDto::Idle) < target
                && pool.usable_capacity_count() < target.saturating_mul(2)
            {
                pool.slots.push(SlotRecord::idle());
            }
            Ok(TaskPoolWorktreeMaintenanceResponse {
                idle_count: pool.count(TaskPoolWorktreeSlotStateDto::Idle),
                target_idle_count: target,
                snapshot: Some(snapshot_from_pool(&project.id, pool)),
            })
        })
    }

    /// 将现有智能体运行投射到工位；同一定义可绑定多个独立运行。
    pub async fn bind_run(
        &self,
        project: &project::Info,
        task_id: &TaskId,
        run_id: &str,
        agent_id: Option<String>,
    ) -> Result<TaskPoolWorktreeSlotDto> {
        validate_task_id(task_id)?;
        if run_id.trim().is_empty() {
            bail!("missing run id");
        }
        self.with_pool(project, |pool| {
            if let Some(slot) =
                pool.slots.iter().find(|slot| slot.run_id.as_deref() == Some(run_id))
            {
                if slot.leased_task_id.as_ref() != Some(task_id) || slot.agent_id != agent_id {
                    bail!("run ownership mismatch");
                }
                return Ok(slot_to_dto(slot));
            }
            let index = match pool
                .slots
                .iter()
                .position(|slot| slot.state == TaskPoolWorktreeSlotStateDto::Idle)
            {
                Some(index) => index,
                None => {
                    pool.slots.push(SlotRecord::idle());
                    pool.slots.len() - 1
                }
            };
            let slot = &mut pool.slots[index];
            slot.state = TaskPoolWorktreeSlotStateDto::Busy;
            slot.leased_task_id = Some(task_id.clone());
            slot.run_id = Some(run_id.to_string());
            slot.agent_id = agent_id;
            Ok(slot_to_dto(slot))
        })
    }

    /// 仅在调用者确认运行退出后清理其逻辑归属，不触碰目录。
    pub async fn finish_run(
        &self,
        project: &project::Info,
        task_id: &TaskId,
        run_id: &str,
        reason: Option<String>,
    ) -> Result<bool> {
        self.with_pool(project, |pool| {
            let Some(slot) =
                pool.slots.iter_mut().find(|slot| slot.run_id.as_deref() == Some(run_id))
            else {
                return Ok(false);
            };
            if slot.leased_task_id.as_ref() != Some(task_id) {
                bail!("run ownership mismatch");
            }
            slot.reset(reason);
            Ok(true)
        })
    }

    pub async fn acquire_and_prepare(
        &self,
        project: &project::Info,
        task_id: &TaskId,
    ) -> Result<TaskPoolWorktreeAcquireResponse> {
        validate_task_id(task_id)?;
        self.with_pool(project, |pool| {
            if let Some(slot) =
                pool.slots.iter().find(|slot| slot.leased_task_id.as_ref() == Some(task_id))
            {
                return Ok(TaskPoolWorktreeAcquireResponse {
                    leased: true,
                    slot: Some(slot_to_dto(slot)),
                });
            }
            let index = match pool
                .slots
                .iter()
                .position(|slot| slot.state == TaskPoolWorktreeSlotStateDto::Idle)
            {
                Some(index) => index,
                None => {
                    pool.slots.push(SlotRecord::idle());
                    pool.slots.len() - 1
                }
            };
            let slot = &mut pool.slots[index];
            slot.state = TaskPoolWorktreeSlotStateDto::Busy;
            slot.leased_task_id = Some(task_id.clone());
            Ok(TaskPoolWorktreeAcquireResponse { leased: true, slot: Some(slot_to_dto(slot)) })
        })
    }

    pub async fn release_task_slot(
        &self,
        project: &project::Info,
        task_id: &TaskId,
    ) -> Result<TaskPoolWorktreeReleaseResponse> {
        validate_task_id(task_id)?;
        self.with_pool(project, |pool| {
            let Some(slot) =
                pool.slots.iter_mut().find(|slot| slot.leased_task_id.as_ref() == Some(task_id))
            else {
                return Ok(TaskPoolWorktreeReleaseResponse { released: false });
            };
            if slot.run_id.is_some() {
                bail!("active run must be released by its owner");
            }
            slot.reset(None);
            Ok(TaskPoolWorktreeReleaseResponse { released: true })
        })
    }

    pub async fn recycle_task_slot(
        &self,
        project: &project::Info,
        task_id: &TaskId,
        reason: Option<String>,
    ) -> Result<TaskPoolWorktreeRecycleResponse> {
        validate_task_id(task_id)?;
        self.with_pool(project, |pool| {
            let Some(slot) =
                pool.slots.iter_mut().find(|slot| slot.leased_task_id.as_ref() == Some(task_id))
            else {
                return Ok(TaskPoolWorktreeRecycleResponse { recycled: false });
            };
            if slot.run_id.is_some() {
                bail!("active run must be recycled by its owner");
            }
            slot.reset(reason);
            Ok(TaskPoolWorktreeRecycleResponse { recycled: true })
        })
    }

    pub async fn recycle_slot(
        &self,
        project: &project::Info,
        slot_id: &str,
    ) -> Result<TaskPoolWorktreeSlotRecycleResponse> {
        self.with_pool(project, |pool| {
            let slot = pool
                .slots
                .iter_mut()
                .find(|slot| slot.id == slot_id)
                .context("task worktree slot missing")?;
            if slot.leased_task_id.is_some()
                || matches!(
                    slot.state,
                    TaskPoolWorktreeSlotStateDto::Busy | TaskPoolWorktreeSlotStateDto::Recycling
                )
            {
                bail!("cannot recycle an active task slot");
            }
            slot.reset(None);
            Ok(TaskPoolWorktreeSlotRecycleResponse { recycled: true })
        })
    }
    pub async fn try_lock_merge_target(
        &self,
        project: &project::Info,
        task_id: &TaskId,
        target_branch: &str,
    ) -> Result<TaskPoolWorktreeMergeLockResponse> {
        validate_task_id(task_id)?;
        let target_branch = normalize_merge_target(target_branch);
        if target_branch.is_empty() {
            return Ok(TaskPoolWorktreeMergeLockResponse { locked: true, holder_task_id: None });
        }

        let project_key = self.ensure_project_pool(project).await?;
        let mut pools = self.pools.lock().expect("task worktree pool lock poisoned");
        let pool = pools
            .get_mut(&project_key)
            .with_context(|| format!("task worktree pool missing: {project_key}"))?;

        let now_ms = now_epoch_ms();
        if let Some(holder) = pool.merge_target_locks.get(&target_branch) {
            let is_self = holder.task_id == *task_id;
            let is_stale = now_ms.saturating_sub(holder.locked_at_ms) >= MERGE_LOCK_STALE_AFTER_MS;
            if !is_self && !is_stale {
                return Ok(TaskPoolWorktreeMergeLockResponse {
                    locked: false,
                    holder_task_id: Some(holder.task_id.clone()),
                });
            }
            // 续锁（同任务）或抢占（陈旧锁）：覆盖为当前任务并刷新加锁时刻。
        }

        pool.merge_target_locks.insert(
            target_branch,
            MergeLockEntry { task_id: task_id.clone(), locked_at_ms: now_ms },
        );
        Ok(TaskPoolWorktreeMergeLockResponse {
            locked: true,
            holder_task_id: Some(task_id.clone()),
        })
    }

    pub async fn unlock_merge_target(
        &self,
        project: &project::Info,
        task_id: &TaskId,
        target_branch: &str,
    ) -> Result<TaskPoolWorktreeMergeLockResponse> {
        validate_task_id(task_id)?;
        let target_branch = normalize_merge_target(target_branch);
        if target_branch.is_empty() {
            return Ok(TaskPoolWorktreeMergeLockResponse { locked: false, holder_task_id: None });
        }

        let project_key = self.ensure_project_pool(project).await?;
        let mut pools = self.pools.lock().expect("task worktree pool lock poisoned");
        let pool = pools
            .get_mut(&project_key)
            .with_context(|| format!("task worktree pool missing: {project_key}"))?;
        if pool
            .merge_target_locks
            .get(&target_branch)
            .is_some_and(|holder| holder.task_id == *task_id)
        {
            pool.merge_target_locks.remove(&target_branch);
        }
        let holder =
            pool.merge_target_locks.get(&target_branch).map(|holder| holder.task_id.clone());
        Ok(TaskPoolWorktreeMergeLockResponse { locked: holder.is_some(), holder_task_id: holder })
    }

    pub async fn merge_lock_holder(
        &self,
        project: &project::Info,
        target_branch: &str,
    ) -> Result<Option<TaskId>> {
        let target_branch = normalize_merge_target(target_branch);
        if target_branch.is_empty() {
            return Ok(None);
        }

        let project_key = self.ensure_project_pool(project).await?;
        let pools = self.pools.lock().expect("task worktree pool lock poisoned");
        Ok(pools
            .get(&project_key)
            .and_then(|pool| pool.merge_target_locks.get(&target_branch))
            .map(|holder| holder.task_id.clone()))
    }
}

#[derive(Debug, Clone)]
struct ProjectPoolState {
    repo_root: String,
    pool_root: PathBuf,
    base_branch: String,
    slots: Vec<SlotRecord>,
    merge_target_locks: HashMap<String, MergeLockEntry>,
}
impl ProjectPoolState {
    fn count(&self, state: TaskPoolWorktreeSlotStateDto) -> u32 {
        self.slots.iter().filter(|slot| slot.state == state).count() as u32
    }
    fn usable_capacity_count(&self) -> u32 {
        self.slots
            .iter()
            .filter(|slot| {
                matches!(
                    slot.state,
                    TaskPoolWorktreeSlotStateDto::Idle | TaskPoolWorktreeSlotStateDto::Busy
                )
            })
            .count() as u32
    }
}
#[derive(Debug, Clone)]
struct SlotRecord {
    id: String,
    path: String,
    branch: String,
    state: TaskPoolWorktreeSlotStateDto,
    run_id: Option<String>,
    agent_id: Option<String>,
    leased_task_id: Option<TaskId>,
    taint_reason: Option<String>,
}
impl SlotRecord {
    fn idle() -> Self {
        Self {
            run_id: None,
            agent_id: None,
            id: next_slot_id(),
            path: String::new(),
            branch: String::new(),
            state: TaskPoolWorktreeSlotStateDto::Idle,
            leased_task_id: None,
            taint_reason: None,
        }
    }
    fn reset(&mut self, reason: Option<String>) {
        self.state = if reason.is_some() {
            TaskPoolWorktreeSlotStateDto::Tainted
        } else {
            TaskPoolWorktreeSlotStateDto::Idle
        };
        self.leased_task_id = None;
        self.run_id = None;
        self.agent_id = None;
        self.taint_reason = reason;
    }
}
fn validate_task_id(task_id: &TaskId) -> Result<()> {
    if task_id.as_ref().trim().is_empty() {
        bail!("missing task id");
    }
    Ok(())
}
fn normalize_merge_target(target: &str) -> String {
    target.trim().to_string()
}
fn next_slot_id() -> String {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or_default();
    let sequence = SLOT_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    format!("slot-{millis}-{sequence}")
}

fn snapshot_from_pool(
    project_id: &str,
    pool: &ProjectPoolState,
) -> TaskPoolWorktreeSnapshotResponse {
    let now_ms = now_epoch_ms();
    let mut merge_target_locks = pool
        .merge_target_locks
        .iter()
        .map(|(target_branch, entry)| TaskPoolWorktreeMergeLockDto {
            target_branch: target_branch.clone(),
            task_id: entry.task_id.clone(),
            age_ms: now_ms.saturating_sub(entry.locked_at_ms),
        })
        .collect::<Vec<_>>();
    merge_target_locks.sort_by(|a, b| {
        a.target_branch
            .cmp(&b.target_branch)
            .then_with(|| a.task_id.as_ref().cmp(b.task_id.as_ref()))
    });

    let mut slots = pool.slots.iter().map(slot_to_dto).collect::<Vec<_>>();
    slots.sort_by(|a, b| a.id.cmp(&b.id));

    TaskPoolWorktreeSnapshotResponse {
        project_id: ProjectId(project_id.to_string()),
        repo_root: pool.repo_root.clone(),
        pool_root: pool.pool_root.to_string_lossy().to_string(),
        base_branch: pool.base_branch.clone(),
        idle_count: pool.count(TaskPoolWorktreeSlotStateDto::Idle),
        busy_count: pool.count(TaskPoolWorktreeSlotStateDto::Busy),
        tainted_count: pool.count(TaskPoolWorktreeSlotStateDto::Tainted),
        recycling_count: pool.count(TaskPoolWorktreeSlotStateDto::Recycling),
        dead_count: pool.count(TaskPoolWorktreeSlotStateDto::Dead),
        merge_target_locks,
        slots,
    }
}

fn slot_to_dto(slot: &SlotRecord) -> TaskPoolWorktreeSlotDto {
    TaskPoolWorktreeSlotDto {
        run_id: slot.run_id.clone(),
        agent_id: slot.agent_id.clone(),
        id: slot.id.clone(),
        path: slot.path.clone(),
        branch: slot.branch.clone(),
        state: slot.state,
        leased_task_id: slot.leased_task_id.clone(),
        taint_reason: slot.taint_reason.clone(),
    }
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
