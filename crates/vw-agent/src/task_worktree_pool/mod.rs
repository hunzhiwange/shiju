use crate::app::agent::project;
use anyhow::{Context, Result, bail};
use directories::UserDirs;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
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

/// 合并锁陈旧阈值：持有超过该时长仍未释放即视为陈旧，可被新任务在 `try_lock_merge_target`
/// 请求路径内直接抢占。量级对齐合并阶段长耗时（30 分钟），覆盖持有进程崩溃/被杀后 Drop
/// 守卫无法运行导致锁永不回收的情况。
const MERGE_LOCK_STALE_AFTER_MS: u64 = 30 * 60 * 1000;

/// 合并锁持有记录：持有任务与加锁时刻（epoch 毫秒），用于 TTL 过期判定与抢占。
#[derive(Debug, Clone)]
struct MergeLockEntry {
    task_id: TaskId,
    locked_at_ms: u64,
}

/// 当前 epoch 毫秒；在 `SystemTime` 回退等异常时间下取 0，绝不 panic。
fn now_epoch_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or(0)
}

#[derive(Clone, Default)]
pub struct TaskWorktreePoolService {
    pools: Arc<Mutex<HashMap<String, ProjectPoolState>>>,
    /// 按 (project_key, task_id) 串行化 acquire：`acquire_and_prepare` 跨多次加锁与异步
    /// git checkout，整体并非原子。同一任务被重复触发（手动 + 自动调度）时若并发进入，
    /// 会各自租到不同空闲槽位并对同一 `vw/task/<id>` 分支执行 `checkout -B`，git 拒绝第二个
    /// 并报 "branch is already used by worktree"。持有此锁后第二个 acquire 等待第一个完成，
    /// 再走 `assigned_slot` 快路径复用已租槽位，使分配幂等。
    acquire_locks: Arc<Mutex<HashMap<String, Arc<tokio::sync::Mutex<()>>>>>,
    pool_root_override: Option<PathBuf>,
}

impl TaskWorktreePoolService {
    pub fn new() -> Self {
        Self::default()
    }

    #[cfg(test)]
    pub(crate) fn with_pool_root_for_tests(pool_root: PathBuf) -> Self {
        Self {
            pools: Arc::new(Mutex::new(HashMap::new())),
            acquire_locks: Arc::new(Mutex::new(HashMap::new())),
            pool_root_override: Some(pool_root),
        }
    }

    /// 取出某任务的 acquire 串行化锁；不存在时创建。
    fn acquire_lock_for(&self, project_key: &str, task_id: &TaskId) -> Arc<tokio::sync::Mutex<()>> {
        let key = format!("{}::{}", project_key, task_id.as_ref());
        let mut locks = self.acquire_locks.lock().expect("acquire lock map poisoned");
        Arc::clone(locks.entry(key).or_default())
    }

    /// 任务释放槽位时移除其 acquire 锁条目，避免长生命周期守护进程的锁表无界增长。
    /// 仍持有 `Arc` 克隆的在途等待者不受影响。
    fn drop_acquire_lock(&self, project_key: &str, task_id: &TaskId) {
        let key = format!("{}::{}", project_key, task_id.as_ref());
        let mut locks = self.acquire_locks.lock().expect("acquire lock map poisoned");
        locks.remove(&key);
    }

    pub async fn snapshot(
        &self,
        project: &project::Info,
    ) -> Result<TaskPoolWorktreeSnapshotResponse> {
        let project_key = self.sync_project(project).await?;
        let pools = self.pools.lock().expect("task worktree pool lock poisoned");
        let pool = pools
            .get(&project_key)
            .with_context(|| format!("task worktree pool missing after sync: {project_key}"))?;
        Ok(snapshot_from_pool(&project_key, pool))
    }

    pub async fn maintain(
        &self,
        project: &project::Info,
        desired_capacity: u32,
    ) -> Result<TaskPoolWorktreeMaintenanceResponse> {
        let project_key = self.sync_project(project).await?;
        let target_idle = desired_capacity.max(1);
        let max_worktrees = target_idle.saturating_mul(2);

        loop {
            let create = {
                let pools = self.pools.lock().expect("task worktree pool lock poisoned");
                let Some(pool) = pools.get(&project_key) else {
                    break;
                };
                let idle_count = pool.count(TaskPoolWorktreeSlotStateDto::Idle);
                let usable_count = pool.usable_capacity_count();
                if idle_count >= target_idle || usable_count >= max_worktrees {
                    None
                } else {
                    Some((pool.repo_root.clone(), pool.pool_root.clone(), pool.base_branch.clone()))
                }
            };

            let Some((repo_root, pool_root, base_branch)) = create else {
                break;
            };

            let slot = create_managed_worktree(&repo_root, &pool_root, &base_branch).await?;
            let mut pools = self.pools.lock().expect("task worktree pool lock poisoned");
            if let Some(pool) = pools.get_mut(&project_key) {
                pool.slots.push(slot);
                pool.sort_slots();
            }
        }

        let snapshot = self.snapshot(project).await?;
        Ok(TaskPoolWorktreeMaintenanceResponse {
            idle_count: snapshot.idle_count,
            target_idle_count: target_idle,
            snapshot: Some(snapshot),
        })
    }

    pub async fn acquire_and_prepare(
        &self,
        project: &project::Info,
        task_id: &TaskId,
    ) -> Result<TaskPoolWorktreeAcquireResponse> {
        validate_task_id(task_id)?;
        let project_key = self.sync_project(project).await?;

        // 串行化同一任务的并发 acquire，避免对同一分支重复 `checkout -B`。
        let acquire_lock = self.acquire_lock_for(&project_key, task_id);
        let _acquire_guard = acquire_lock.lock().await;

        let mut last_prepare_error = None;
        for _ in 0..128 {
            if let Some(slot) = self.assigned_slot(&project_key, task_id)? {
                if let Some(reason) = slot_health_failure(Path::new(&slot.path)) {
                    self.mark_slot_dead(&project_key, &slot.id, reason)?;
                    continue;
                }
                let prepared = match prepare_slot_for_task(&slot, task_id).await {
                    Ok(prepared) => prepared,
                    Err(error) => {
                        let reason = error.to_string();
                        self.mark_slot_tainted(&project_key, &slot.id, reason.clone())?;
                        last_prepare_error = Some(reason);
                        continue;
                    }
                };
                self.finish_acquired_slot(&project_key, prepared.clone(), task_id)?;
                return Ok(TaskPoolWorktreeAcquireResponse {
                    leased: true,
                    slot: Some(slot_to_dto(&prepared)),
                });
            }

            self.maintain(project, 1).await?;
            let slot = match self.lease_next_idle_slot(&project_key, task_id)? {
                Some(slot) => slot,
                // 没有空闲槽位时通常是 maintain 的 max_worktrees 上限挡住了预热，
                // 但本次 acquire 是按需分配，必须保证拿到槽位；否则桌面端会报
                // “gateway 未分配任务 worktree 槽位”。这里按需新建一个托管 worktree。
                None => match self.create_and_lease_slot(&project_key, task_id).await? {
                    Some(slot) => slot,
                    None => {
                        return Ok(TaskPoolWorktreeAcquireResponse { leased: false, slot: None });
                    }
                },
            };
            if let Some(reason) = slot_health_failure(Path::new(&slot.path)) {
                self.mark_slot_dead(&project_key, &slot.id, reason)?;
                continue;
            }
            let prepared = match prepare_slot_for_task(&slot, task_id).await {
                Ok(prepared) => prepared,
                Err(error) => {
                    let reason = error.to_string();
                    self.mark_slot_tainted(&project_key, &slot.id, reason.clone())?;
                    last_prepare_error = Some(reason);
                    continue;
                }
            };
            self.finish_acquired_slot(&project_key, prepared.clone(), task_id)?;
            return Ok(TaskPoolWorktreeAcquireResponse {
                leased: true,
                slot: Some(slot_to_dto(&prepared)),
            });
        }

        if let Some(reason) = last_prepare_error {
            bail!(
                "failed to acquire a healthy task worktree slot after replacing failed slots: {reason}"
            );
        }
        bail!("failed to acquire a healthy task worktree slot");
    }

    pub async fn release_task_slot(
        &self,
        project: &project::Info,
        task_id: &TaskId,
    ) -> Result<TaskPoolWorktreeReleaseResponse> {
        validate_task_id(task_id)?;
        let project_key = self.ensure_project_pool(project).await?;
        self.drop_acquire_lock(&project_key, task_id);
        let mut pools = self.pools.lock().expect("task worktree pool lock poisoned");
        let Some(pool) = pools.get_mut(&project_key) else {
            return Ok(TaskPoolWorktreeReleaseResponse { released: false });
        };
        let Some(slot) = pool.slot_for_task_mut(task_id) else {
            return Ok(TaskPoolWorktreeReleaseResponse { released: false });
        };

        if matches!(
            slot.state,
            TaskPoolWorktreeSlotStateDto::Busy | TaskPoolWorktreeSlotStateDto::Recycling
        ) {
            slot.state = TaskPoolWorktreeSlotStateDto::Idle;
            slot.taint_reason = None;
        }
        slot.leased_task_id = None;
        Ok(TaskPoolWorktreeReleaseResponse { released: true })
    }

    pub async fn recycle_task_slot(
        &self,
        project: &project::Info,
        task_id: &TaskId,
        taint_reason: Option<String>,
    ) -> Result<TaskPoolWorktreeRecycleResponse> {
        validate_task_id(task_id)?;
        let project_key = self.ensure_project_pool(project).await?;
        let Some(slot_id) = self.slot_id_for_task(&project_key, task_id)? else {
            return Ok(TaskPoolWorktreeRecycleResponse { recycled: false });
        };
        self.recycle_slot_by_id(&project_key, &slot_id, taint_reason).await?;
        Ok(TaskPoolWorktreeRecycleResponse { recycled: true })
    }

    pub async fn recycle_slot(
        &self,
        project: &project::Info,
        slot_id: &str,
    ) -> Result<TaskPoolWorktreeSlotRecycleResponse> {
        let project_key = self.ensure_project_pool(project).await?;
        self.recycle_slot_by_id(&project_key, slot_id, None).await?;
        Ok(TaskPoolWorktreeSlotRecycleResponse { recycled: true })
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

    async fn sync_project(&self, project: &project::Info) -> Result<String> {
        let project_key = validate_project(project)?;
        let repo_root = git_repo_root(Path::new(&project.worktree)).await?;
        let base_branch = git_base_branch(&repo_root).await?;
        let pool_root = self.pool_root_for_project(project);
        let entries = git_worktree_entries(&repo_root).await?;

        let previous = {
            let pools = self.pools.lock().expect("task worktree pool lock poisoned");
            pools.get(&project_key).cloned()
        };
        let previous_slots = previous
            .as_ref()
            .map(|pool| {
                pool.slots
                    .iter()
                    .cloned()
                    .map(|slot| (slot.id.clone(), slot))
                    .collect::<HashMap<_, _>>()
            })
            .unwrap_or_default();

        let mut slots = Vec::new();
        for entry in entries {
            if !is_managed_slot_path(&entry.path, &pool_root) {
                continue;
            }

            let slot_id = slot_id_from_path(&entry.path)?;
            let branch = entry.branch.unwrap_or_else(|| "HEAD".to_string());
            let mut slot = slot_from_previous(
                &slot_id,
                entry.path,
                &base_branch,
                branch,
                previous_slots.get(&slot_id),
            );
            if let Some(reason) = slot_health_failure(Path::new(&slot.path)) {
                slot.state = TaskPoolWorktreeSlotStateDto::Dead;
                slot.leased_task_id = None;
                slot.taint_reason = Some(reason);
            }
            slots.push(slot);
        }

        let mut pool = ProjectPoolState {
            repo_root: repo_root.to_string_lossy().to_string(),
            pool_root,
            base_branch,
            slots,
            merge_target_locks: previous
                .as_ref()
                .map(|pool| pool.merge_target_locks.clone())
                .unwrap_or_default(),
        };
        pool.retain_known_merge_locks();
        pool.sort_slots();

        let mut pools = self.pools.lock().expect("task worktree pool lock poisoned");
        pools.insert(project_key.clone(), pool);
        Ok(project_key)
    }

    fn pool_root_for_project(&self, project: &project::Info) -> PathBuf {
        self.pool_root_override.clone().unwrap_or_else(|| default_pool_root(project))
    }

    async fn ensure_project_pool(&self, project: &project::Info) -> Result<String> {
        let project_key = validate_project(project)?;
        if self.pools.lock().expect("task worktree pool lock poisoned").contains_key(&project_key) {
            return Ok(project_key);
        }
        self.sync_project(project).await
    }

    fn assigned_slot(&self, project_key: &str, task_id: &TaskId) -> Result<Option<SlotRecord>> {
        let pools = self.pools.lock().expect("task worktree pool lock poisoned");
        let Some(pool) = pools.get(project_key) else {
            return Ok(None);
        };
        Ok(pool.slot_for_task(task_id).cloned())
    }

    /// 在没有空闲槽位时按需新建一个托管 worktree 并直接租给该任务。
    ///
    /// 与 `maintain` 不同，这里不受 `max_worktrees` 预热上限约束：上层调度已经按
    /// `max_concurrent` 控制并发，能走到这一步说明确有任务需要槽位，必须满足。
    async fn create_and_lease_slot(
        &self,
        project_key: &str,
        task_id: &TaskId,
    ) -> Result<Option<SlotRecord>> {
        let create = {
            let pools = self.pools.lock().expect("task worktree pool lock poisoned");
            pools.get(project_key).map(|pool| {
                (pool.repo_root.clone(), pool.pool_root.clone(), pool.base_branch.clone())
            })
        };
        let Some((repo_root, pool_root, base_branch)) = create else {
            return Ok(None);
        };

        let mut slot = create_managed_worktree(&repo_root, &pool_root, &base_branch).await?;
        slot.state = TaskPoolWorktreeSlotStateDto::Busy;
        slot.leased_task_id = Some(task_id.clone());
        slot.taint_reason = None;

        let mut pools = self.pools.lock().expect("task worktree pool lock poisoned");
        let Some(pool) = pools.get_mut(project_key) else {
            return Ok(None);
        };
        pool.slots.push(slot.clone());
        pool.sort_slots();
        Ok(Some(slot))
    }

    fn lease_next_idle_slot(
        &self,
        project_key: &str,
        task_id: &TaskId,
    ) -> Result<Option<SlotRecord>> {
        let mut pools = self.pools.lock().expect("task worktree pool lock poisoned");
        let Some(pool) = pools.get_mut(project_key) else {
            return Ok(None);
        };
        let Some(slot) =
            pool.slots.iter_mut().find(|slot| slot.state == TaskPoolWorktreeSlotStateDto::Idle)
        else {
            return Ok(None);
        };
        slot.state = TaskPoolWorktreeSlotStateDto::Busy;
        slot.leased_task_id = Some(task_id.clone());
        slot.taint_reason = None;
        Ok(Some(slot.clone()))
    }

    fn finish_acquired_slot(
        &self,
        project_key: &str,
        prepared: SlotRecord,
        task_id: &TaskId,
    ) -> Result<()> {
        let mut pools = self.pools.lock().expect("task worktree pool lock poisoned");
        let pool = pools
            .get_mut(project_key)
            .with_context(|| format!("task worktree pool missing: {project_key}"))?;
        let slot = pool
            .slots
            .iter_mut()
            .find(|slot| slot.id == prepared.id)
            .with_context(|| format!("task worktree slot missing: {}", prepared.id))?;
        slot.branch = prepared.branch;
        slot.state = TaskPoolWorktreeSlotStateDto::Busy;
        slot.leased_task_id = Some(task_id.clone());
        slot.taint_reason = None;
        Ok(())
    }

    fn mark_slot_dead(&self, project_key: &str, slot_id: &str, reason: String) -> Result<()> {
        self.update_failed_slot(project_key, slot_id, TaskPoolWorktreeSlotStateDto::Dead, reason)
    }

    fn mark_slot_tainted(&self, project_key: &str, slot_id: &str, reason: String) -> Result<()> {
        self.update_failed_slot(project_key, slot_id, TaskPoolWorktreeSlotStateDto::Tainted, reason)
    }

    fn update_failed_slot(
        &self,
        project_key: &str,
        slot_id: &str,
        state: TaskPoolWorktreeSlotStateDto,
        reason: String,
    ) -> Result<()> {
        let mut pools = self.pools.lock().expect("task worktree pool lock poisoned");
        let pool = pools
            .get_mut(project_key)
            .with_context(|| format!("task worktree pool missing: {project_key}"))?;
        pool.clear_task_for_slot(slot_id);
        let slot = pool
            .slots
            .iter_mut()
            .find(|slot| slot.id == slot_id)
            .with_context(|| format!("task worktree slot missing: {slot_id}"))?;
        slot.state = state;
        slot.leased_task_id = None;
        slot.taint_reason = Some(reason);
        Ok(())
    }

    fn slot_id_for_task(&self, project_key: &str, task_id: &TaskId) -> Result<Option<String>> {
        let pools = self.pools.lock().expect("task worktree pool lock poisoned");
        let Some(pool) = pools.get(project_key) else {
            return Ok(None);
        };
        Ok(pool.slot_for_task(task_id).map(|slot| slot.id.clone()))
    }

    async fn recycle_slot_by_id(
        &self,
        project_key: &str,
        slot_id: &str,
        taint_reason: Option<String>,
    ) -> Result<()> {
        let slot = self.begin_recycle_slot(project_key, slot_id, taint_reason.clone())?;
        match reset_and_clean_slot(&slot).await {
            Ok(recycled) => self.finish_recycle_slot(project_key, recycled, true, taint_reason),
            Err(error) => {
                let reason = error.to_string();
                self.finish_recycle_slot(project_key, slot, false, Some(reason.clone()))?;
                Err(error)
            }
        }
    }

    fn begin_recycle_slot(
        &self,
        project_key: &str,
        slot_id: &str,
        taint_reason: Option<String>,
    ) -> Result<SlotRecord> {
        let mut pools = self.pools.lock().expect("task worktree pool lock poisoned");
        let pool = pools
            .get_mut(project_key)
            .with_context(|| format!("task worktree pool missing: {project_key}"))?;
        pool.clear_task_for_slot(slot_id);
        let slot = pool
            .slots
            .iter_mut()
            .find(|slot| slot.id == slot_id)
            .with_context(|| format!("task worktree slot missing: {slot_id}"))?;
        if slot.state == TaskPoolWorktreeSlotStateDto::Recycling {
            bail!("task worktree slot is already recycling: {slot_id}");
        }
        slot.state = TaskPoolWorktreeSlotStateDto::Recycling;
        slot.leased_task_id = None;
        slot.taint_reason = taint_reason;
        Ok(slot.clone())
    }

    fn finish_recycle_slot(
        &self,
        project_key: &str,
        recycled: SlotRecord,
        clean: bool,
        reason: Option<String>,
    ) -> Result<()> {
        let mut pools = self.pools.lock().expect("task worktree pool lock poisoned");
        let pool = pools
            .get_mut(project_key)
            .with_context(|| format!("task worktree pool missing: {project_key}"))?;
        pool.clear_task_for_slot(&recycled.id);
        let slot = pool
            .slots
            .iter_mut()
            .find(|slot| slot.id == recycled.id)
            .with_context(|| format!("task worktree slot missing: {}", recycled.id))?;
        slot.branch = recycled.branch;
        slot.state = if clean && reason.is_some() {
            TaskPoolWorktreeSlotStateDto::Tainted
        } else if clean {
            TaskPoolWorktreeSlotStateDto::Idle
        } else {
            TaskPoolWorktreeSlotStateDto::Dead
        };
        slot.leased_task_id = None;
        slot.taint_reason = reason;
        Ok(())
    }

    #[cfg(test)]
    fn set_slot_state_for_tests(
        &self,
        project_id: &str,
        slot_id: &str,
        state: TaskPoolWorktreeSlotStateDto,
        leased_task_id: Option<&str>,
        taint_reason: Option<&str>,
    ) {
        let mut pools = self.pools.lock().expect("task worktree pool lock poisoned");
        let pool = pools.get_mut(project_id).expect("project pool");
        let slot = pool.slots.iter_mut().find(|slot| slot.id == slot_id).expect("slot");
        slot.state = state;
        slot.leased_task_id = leased_task_id.map(|value| TaskId(value.to_string()));
        slot.taint_reason = taint_reason.map(str::to_string);
    }

    #[cfg(test)]
    fn set_merge_lock_for_tests(
        &self,
        project_id: &str,
        target_branch: &str,
        task_id: &str,
        locked_at_ms: u64,
    ) {
        let mut pools = self.pools.lock().expect("task worktree pool lock poisoned");
        let pool = pools.get_mut(project_id).expect("project pool");
        pool.merge_target_locks.insert(
            target_branch.to_string(),
            MergeLockEntry { task_id: TaskId(task_id.to_string()), locked_at_ms },
        );
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

    fn sort_slots(&mut self) {
        self.slots.sort_by(|a, b| a.id.cmp(&b.id));
    }

    fn retain_known_merge_locks(&mut self) {
        self.merge_target_locks.retain(|target_branch, entry| {
            !target_branch.trim().is_empty() && !entry.task_id.as_ref().trim().is_empty()
        });
    }

    fn slot_for_task(&self, task_id: &TaskId) -> Option<&SlotRecord> {
        self.slots.iter().find(|slot| slot.leased_task_id.as_ref() == Some(task_id))
    }

    fn slot_for_task_mut(&mut self, task_id: &TaskId) -> Option<&mut SlotRecord> {
        self.slots.iter_mut().find(|slot| slot.leased_task_id.as_ref() == Some(task_id))
    }

    fn clear_task_for_slot(&mut self, slot_id: &str) {
        for slot in &mut self.slots {
            if slot.id == slot_id {
                slot.leased_task_id = None;
            }
        }
    }
}

#[derive(Debug, Clone)]
struct SlotRecord {
    id: String,
    path: String,
    base_branch: String,
    branch: String,
    state: TaskPoolWorktreeSlotStateDto,
    leased_task_id: Option<TaskId>,
    taint_reason: Option<String>,
}

#[derive(Debug)]
struct WorktreeEntry {
    path: PathBuf,
    branch: Option<String>,
}

#[derive(Debug)]
struct GitOutput {
    success: bool,
    stdout: String,
    stderr: String,
}

fn validate_project(project: &project::Info) -> Result<String> {
    if project.id.trim().is_empty() {
        bail!("missing project id");
    }
    if project.worktree.trim().is_empty() {
        bail!("missing project worktree");
    }
    if project.vcs != Some(project::Vcs::Git) {
        bail!("task worktree pool requires a git project");
    }
    Ok(project.id.clone())
}

fn validate_task_id(task_id: &TaskId) -> Result<()> {
    if task_id.as_ref().trim().is_empty() {
        bail!("missing task id");
    }
    Ok(())
}

fn normalize_merge_target(target_branch: &str) -> String {
    target_branch.trim().to_string()
}

fn default_pool_root(project: &project::Info) -> PathBuf {
    let base = UserDirs::new()
        .map_or_else(|| PathBuf::from(&project.worktree), |dirs| dirs.home_dir().to_path_buf());
    vw_config_types::paths::task_worktrees_dir(base)
}

async fn git_repo_root(worktree: &Path) -> Result<PathBuf> {
    let output = run_git(worktree, ["rev-parse", "--show-toplevel"]).await?;
    if !output.success {
        bail!(git_error_text(&output, "failed to resolve git repo root"));
    }
    let root = output.stdout.trim();
    if root.is_empty() {
        bail!("git repo root is empty");
    }
    Ok(PathBuf::from(root))
}

async fn git_base_branch(repo_root: &Path) -> Result<String> {
    let output = run_git(repo_root, ["rev-parse", "--abbrev-ref", "HEAD"]).await?;
    if !output.success {
        return Ok("main".to_string());
    }
    let branch = output.stdout.trim();
    if branch.is_empty() || branch == "HEAD" {
        Ok("main".to_string())
    } else {
        Ok(branch.to_string())
    }
}

async fn git_worktree_entries(repo_root: &Path) -> Result<Vec<WorktreeEntry>> {
    let output = run_git(repo_root, ["worktree", "list", "--porcelain"]).await?;
    if !output.success {
        bail!(git_error_text(&output, "failed to read git worktrees"));
    }
    Ok(parse_worktree_list(&output.stdout))
}

async fn run_git<const N: usize>(cwd: &Path, args: [&str; N]) -> Result<GitOutput> {
    let cwd = cwd.to_path_buf();
    let args = args.map(str::to_string);
    let output = tokio::task::spawn_blocking(move || {
        let mut command = crate::app::agent::shell::git_std_command();
        command.current_dir(cwd);
        for arg in args {
            command.arg(arg);
        }
        command.output()
    })
    .await??;

    Ok(GitOutput {
        success: output.status.success(),
        stdout: normalize_output(&output.stdout),
        stderr: normalize_output(&output.stderr),
    })
}

async fn run_git_dynamic(cwd: &Path, args: Vec<String>) -> Result<GitOutput> {
    let cwd = cwd.to_path_buf();
    let output = tokio::task::spawn_blocking(move || {
        let mut command = crate::app::agent::shell::git_std_command();
        command.current_dir(cwd);
        for arg in args {
            command.arg(arg);
        }
        command.output()
    })
    .await??;

    Ok(GitOutput {
        success: output.status.success(),
        stdout: normalize_output(&output.stdout),
        stderr: normalize_output(&output.stderr),
    })
}

fn normalize_output(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).replace("\r\n", "\n").replace('\r', "\n")
}

fn git_error_text(output: &GitOutput, fallback: &str) -> String {
    let message = [output.stderr.trim(), output.stdout.trim()]
        .into_iter()
        .filter(|value| !value.is_empty())
        .collect::<Vec<_>>()
        .join("\n");
    if message.is_empty() { fallback.to_string() } else { message }
}

fn parse_worktree_list(input: &str) -> Vec<WorktreeEntry> {
    let mut entries = Vec::new();
    let mut current_path: Option<PathBuf> = None;
    let mut current_branch: Option<String> = None;

    for line in input.lines() {
        if let Some(path) = line.strip_prefix("worktree ") {
            if let Some(path) = current_path.take() {
                entries.push(WorktreeEntry { path, branch: current_branch.take() });
            }
            current_path = Some(PathBuf::from(path.trim()));
            current_branch = None;
            continue;
        }
        if let Some(branch) = line.strip_prefix("branch ") {
            let branch = branch
                .trim()
                .strip_prefix("refs/heads/")
                .unwrap_or_else(|| branch.trim())
                .to_string();
            current_branch = Some(branch);
            continue;
        }
        if line.trim().is_empty() {
            if let Some(path) = current_path.take() {
                entries.push(WorktreeEntry { path, branch: current_branch.take() });
            }
            current_branch = None;
        }
    }

    if let Some(path) = current_path.take() {
        entries.push(WorktreeEntry { path, branch: current_branch.take() });
    }
    entries
}

fn is_managed_slot_path(path: &Path, pool_root: &Path) -> bool {
    path.file_name().and_then(|value| value.to_str()).is_some_and(|name| name.starts_with("slot-"))
        && comparable_path(path).starts_with(comparable_path(pool_root))
}

fn comparable_path(path: &Path) -> PathBuf {
    std::fs::canonicalize(path).unwrap_or_else(|_| {
        if path.is_absolute() {
            path.to_path_buf()
        } else {
            std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")).join(path)
        }
    })
}

fn slot_id_from_path(path: &Path) -> Result<String> {
    let Some(slot_id) = path.file_name().and_then(|value| value.to_str()) else {
        bail!("managed worktree path has no slot id: {}", path.display());
    };
    Ok(slot_id.to_string())
}

fn slot_from_previous(
    slot_id: &str,
    path: PathBuf,
    base_branch: &str,
    branch: String,
    previous: Option<&SlotRecord>,
) -> SlotRecord {
    let (state, leased_task_id, taint_reason) =
        previous.map_or((TaskPoolWorktreeSlotStateDto::Idle, None, None), |slot| {
            match slot.state {
                TaskPoolWorktreeSlotStateDto::Busy | TaskPoolWorktreeSlotStateDto::Recycling => {
                    (slot.state, slot.leased_task_id.clone(), slot.taint_reason.clone())
                }
                TaskPoolWorktreeSlotStateDto::Tainted | TaskPoolWorktreeSlotStateDto::Dead => {
                    (slot.state, None, slot.taint_reason.clone())
                }
                TaskPoolWorktreeSlotStateDto::Idle => {
                    (TaskPoolWorktreeSlotStateDto::Idle, None, None)
                }
            }
        });

    SlotRecord {
        id: slot_id.to_string(),
        path: path.to_string_lossy().to_string(),
        base_branch: base_branch.to_string(),
        branch: if branch.trim().is_empty() { base_branch.to_string() } else { branch },
        state,
        leased_task_id,
        taint_reason,
    }
}

fn slot_health_failure(path: &Path) -> Option<String> {
    if !path.exists() {
        return Some("worktree directory does not exist".to_string());
    }

    let git_dir = match worktree_git_dir(path) {
        Ok(git_dir) => git_dir,
        Err(error) => return Some(error),
    };
    if !git_dir.exists() {
        return Some("worktree gitdir does not exist".to_string());
    }
    if git_dir.join("index.lock").exists() {
        return Some("worktree git index.lock exists".to_string());
    }
    None
}

async fn prepare_slot_for_task(slot: &SlotRecord, task_id: &TaskId) -> Result<SlotRecord> {
    let task_branch = format!("vw/task/{}", sanitize_branch_token(task_id.as_ref()));
    let reset_output =
        run_git(Path::new(&slot.path), ["reset", "--hard", slot.branch_base_target()]).await?;
    if !reset_output.success {
        bail!(
            "failed to reset task worktree slot: {}",
            git_error_text(&reset_output, "git reset failed")
        );
    }

    let clean_output = run_git(Path::new(&slot.path), ["clean", "-fd"]).await?;
    if !clean_output.success {
        bail!(
            "failed to clean task worktree slot: {}",
            git_error_text(&clean_output, "git clean failed")
        );
    }

    let checkout_output = run_git_dynamic(
        Path::new(&slot.path),
        vec![
            "checkout".to_string(),
            "-B".to_string(),
            task_branch.clone(),
            slot.branch_base_target().to_string(),
        ],
    )
    .await?;
    if !checkout_output.success {
        bail!(
            "failed to checkout task worktree branch: {}",
            git_error_text(&checkout_output, "git checkout failed")
        );
    }

    let mut prepared = slot.clone();
    prepared.branch = task_branch;
    prepared.state = TaskPoolWorktreeSlotStateDto::Busy;
    prepared.leased_task_id = Some(task_id.clone());
    prepared.taint_reason = None;
    Ok(prepared)
}

async fn reset_and_clean_slot(slot: &SlotRecord) -> Result<SlotRecord> {
    let reset_output =
        run_git(Path::new(&slot.path), ["reset", "--hard", slot.branch_base_target()]).await?;
    if !reset_output.success {
        bail!(
            "failed to reset task worktree slot: {}",
            git_error_text(&reset_output, "git reset failed")
        );
    }

    let clean_output = run_git(Path::new(&slot.path), ["clean", "-fd"]).await?;
    if !clean_output.success {
        bail!(
            "failed to clean task worktree slot: {}",
            git_error_text(&clean_output, "git clean failed")
        );
    }

    if !git_is_clean(Path::new(&slot.path)).await? {
        bail!("task worktree slot is dirty after recycle: {}", slot.path);
    }

    let mut recycled = slot.clone();
    recycled.branch =
        current_branch(Path::new(&slot.path)).await.unwrap_or_else(|| slot.branch.clone());
    recycled.state = TaskPoolWorktreeSlotStateDto::Idle;
    recycled.leased_task_id = None;
    recycled.taint_reason = None;
    Ok(recycled)
}

impl SlotRecord {
    fn branch_base_target(&self) -> &str {
        if self.base_branch.trim().is_empty() { "HEAD" } else { &self.base_branch }
    }
}

fn sanitize_branch_token(value: &str) -> String {
    let token = value
        .trim()
        .chars()
        .map(
            |ch| {
                if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.') { ch } else { '-' }
            },
        )
        .collect::<String>()
        .trim_matches(['-', '.', '_'])
        .to_string();
    if token.is_empty() { "task".to_string() } else { token }
}

async fn git_is_clean(path: &Path) -> Result<bool> {
    let output = run_git(path, ["status", "--short"]).await?;
    if !output.success {
        bail!(git_error_text(&output, "failed to read task worktree status"));
    }
    Ok(output.stdout.trim().is_empty())
}

async fn current_branch(path: &Path) -> Option<String> {
    let output = run_git(path, ["rev-parse", "--abbrev-ref", "HEAD"]).await.ok()?;
    if !output.success {
        return None;
    }
    let branch = output.stdout.trim();
    if branch.is_empty() { None } else { Some(branch.to_string()) }
}

fn worktree_git_dir(path: &Path) -> std::result::Result<PathBuf, String> {
    let dot_git = path.join(".git");
    if !dot_git.exists() {
        return Err("missing .git metadata".to_string());
    }
    if dot_git.is_dir() {
        return Ok(dot_git);
    }

    let content =
        std::fs::read_to_string(&dot_git).map_err(|err| format!("failed to read .git: {err}"))?;
    let Some(gitdir) = content
        .lines()
        .find_map(|line| line.trim().strip_prefix("gitdir:").map(str::trim))
        .filter(|value| !value.is_empty())
    else {
        return Err("failed to parse .git metadata".to_string());
    };

    let gitdir = PathBuf::from(gitdir);
    if gitdir.is_absolute() { Ok(gitdir) } else { Ok(path.join(gitdir)) }
}

async fn create_managed_worktree(
    repo_root: &str,
    pool_root: &Path,
    base_branch: &str,
) -> Result<SlotRecord> {
    tokio::fs::create_dir_all(pool_root)
        .await
        .with_context(|| format!("failed to create task worktree pool: {}", pool_root.display()))?;

    for _ in 0..128 {
        let slot_id = next_slot_id();
        let branch = format!("vw/task/{slot_id}");
        let path = pool_root.join(&slot_id);
        if path.exists() {
            continue;
        }

        let output = run_git_dynamic(
            Path::new(repo_root),
            vec![
                "worktree".to_string(),
                "add".to_string(),
                "-b".to_string(),
                branch.clone(),
                path.to_string_lossy().to_string(),
                base_branch.to_string(),
            ],
        )
        .await?;
        if !output.success {
            bail!(git_error_text(&output, "failed to create task worktree slot"));
        }

        return Ok(SlotRecord {
            id: slot_id,
            path: path.to_string_lossy().to_string(),
            base_branch: base_branch.to_string(),
            branch,
            state: TaskPoolWorktreeSlotStateDto::Idle,
            leased_task_id: None,
            taint_reason: None,
        });
    }

    bail!("failed to generate a unique task worktree slot id");
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
