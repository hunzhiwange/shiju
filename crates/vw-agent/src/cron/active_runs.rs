//! 当前进程内的定时任务执行快照，不包含命令、提示词或输出。
//!
//! 快照在重启后自然清空；持久化历史与任务存在性由汇总查询端处理。
use super::CronJob;
use crate::app::agent::config::Config;
use chrono::{DateTime, Utc};
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard, OnceLock};

/// 一次实际执行的元数据；不使用持久化行 ID 或尚未发生的结束信息。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ActiveRun {
    pub execution_id: String,
    pub job_id: String,
    pub started_at: DateTime<Utc>,
    pub name: Option<String>,
    pub workspace: PathBuf,
}

static ACTIVE_RUNS: OnceLock<Mutex<BTreeMap<String, ActiveRun>>> = OnceLock::new();

fn runs() -> MutexGuard<'static, BTreeMap<String, ActiveRun>> {
    ACTIVE_RUNS
        .get_or_init(|| Mutex::new(BTreeMap::new()))
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

// 与工作区运行数据的路径身份规则一致：绝对化后解析真实路径；
// 不存在的路径保留绝对路径，不能退回其它工作区。
fn workspace(config: &Config) -> PathBuf {
    let path = if config.workspace_dir.is_absolute() {
        config.workspace_dir.clone()
    } else {
        std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")).join(&config.workspace_dir)
    };
    path.canonicalize().unwrap_or(path)
}

/// 跨越执行、投递和持久化的作用域；取消或 unwind 也会移除本次执行。
#[must_use]
pub(crate) struct ActiveRunGuard {
    execution_id: String,
}

pub(crate) fn register(
    config: &Config,
    job: &CronJob,
    started_at: DateTime<Utc>,
) -> ActiveRunGuard {
    let execution_id = uuid::Uuid::new_v4().to_string();
    let run = ActiveRun {
        execution_id: execution_id.clone(),
        job_id: job.id.clone(),
        started_at,
        name: job.name.clone(),
        workspace: workspace(config),
    };
    runs().insert(execution_id.clone(), run);
    ActiveRunGuard { execution_id }
}

impl Drop for ActiveRunGuard {
    fn drop(&mut self) {
        runs().remove(&self.execution_id);
    }
}

/// 复制当前工作区执行项，锁不会跨越调用者的 await 或数据库操作。
///
/// 汇总查询须过滤已删除任务，并以工作区、job_id、started_at 与结束记录去重。
pub fn snapshot(config: &Config) -> Vec<ActiveRun> {
    let workspace = workspace(config);
    runs().values().filter(|run| run.workspace == workspace).cloned().collect()
}
