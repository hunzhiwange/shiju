//! 快照清理任务的宿主调度适配。

use agent_snapshot::cleanup;
use agent_scheduler_core as scheduler;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

/// 初始化快照清理任务。
///
/// 注册一个定时任务，定期清理过期的快照数据。任务每小时执行一次。
pub fn init(worktree: impl AsRef<Path>) {
    let worktree = worktree.as_ref().to_path_buf();
    let instance = worktree.to_string_lossy().to_string();

    let run: scheduler::RunFn = Arc::new(move || {
        let worktree = worktree.clone();
        Box::pin(async move {
            tokio::task::spawn_blocking(move || cleanup(&worktree))
                .await
                .map_err(|e| e.to_string())?
                .map_err(|e| e.to_string())?;
            Ok(())
        })
    });

    scheduler::register(scheduler::Task {
        id: "snapshot.cleanup".to_string(),
        interval: Duration::from_secs(60 * 60),
        run,
        scope: scheduler::Scope::Instance(instance),
    });
}
