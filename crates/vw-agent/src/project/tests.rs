//! 宿主快照初始化组合测试。
use super::*;

#[tokio::test]
async fn instance_bootstrap_accepts_existing_directory() {
    let dir = tempfile::TempDir::new().expect("tempdir should create");
    instance_bootstrap(dir.path()).await;
}

#[tokio::test]
async fn instance_bootstrap_records_worktree_once() {
    let dir = tempfile::TempDir::new().expect("tempdir should create");
    let key = dir.path().to_string_lossy().to_string();
    {
        let mut bootstrapped = BOOTSTRAPPED_WORKTREES.lock().unwrap_or_else(|e| e.into_inner());
        bootstrapped.remove(&key);
    }

    instance_bootstrap(dir.path()).await;
    instance_bootstrap(dir.path()).await;

    let bootstrapped = BOOTSTRAPPED_WORKTREES.lock().unwrap_or_else(|e| e.into_inner());
    assert!(bootstrapped.contains(&key));
    assert_eq!(bootstrapped.iter().filter(|entry| *entry == &key).count(), 1);
}

#[tokio::test]
// 全局注册表锁中毒后仍要恢复，避免一次测试或线程崩溃永久阻断项目启动。
async fn instance_bootstrap_recovers_from_poisoned_worktree_registry() {
    let poisoned = std::panic::catch_unwind(|| {
        let _guard = BOOTSTRAPPED_WORKTREES.lock().unwrap_or_else(|error| error.into_inner());
        panic!("poison worktree registry for recovery coverage");
    });
    assert!(poisoned.is_err());

    let dir = tempfile::TempDir::new().expect("tempdir should create");
    let key = dir.path().to_string_lossy().to_string();
    instance_bootstrap(dir.path()).await;

    let bootstrapped = BOOTSTRAPPED_WORKTREES.lock().unwrap_or_else(|error| error.into_inner());
    assert!(bootstrapped.contains(&key));
}
