//! 项目领域重导出及宿主快照初始化组合。
pub use agent_project::*;
use crate::snapshot;
use std::collections::HashSet;
use std::path::Path;
use std::sync::{LazyLock, Mutex};

static BOOTSTRAPPED_WORKTREES: LazyLock<Mutex<HashSet<String>>> =
    LazyLock::new(|| Mutex::new(HashSet::new()));

/// 引导项目实例
pub async fn instance_bootstrap(worktree: impl AsRef<Path>) {
    let worktree = worktree.as_ref().to_path_buf();
    let key = worktree.to_string_lossy().to_string();
    let should_init = {
        let mut bootstrapped = BOOTSTRAPPED_WORKTREES.lock().unwrap_or_else(|e| e.into_inner());
        bootstrapped.insert(key)
    };
    if should_init {
        snapshot::init(&worktree);
    }
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;

#[cfg(all(test, not(target_arch = "wasm32")))]
#[path = "store_tests.rs"]
mod store_tests;
