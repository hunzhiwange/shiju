use super::{operations::*, process};
use std::{process::Command, time::Duration};
use vw_api_types::{agent_apps::AppId, task::TaskStatus};

#[cfg(unix)]
#[tokio::test]
async fn cc_apps_operation_cancel_reaps_process_tree_and_recovers() {
    let temp = tempfile::tempdir().unwrap();
    let pid = temp.path().join("pid");
    let marker = temp.path().join("escaped");
    let mut cmd = Command::new("/bin/sh");
    cmd.args(["-c", "echo $$ > \"$1\"; (sleep 0.4; touch \"$2\") & wait", "fixture"])
        .arg(&pid)
        .arg(&marker);
    let op = Operation::new();
    op.start(vec![AppId::Gemini], move || process::run_command(cmd).map(|_| ())).unwrap();
    tokio::time::timeout(Duration::from_secs(5), async {
        while !pid.exists() {
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    })
    .await
    .unwrap();
    assert!(op.start(vec![AppId::Gemini], || Ok(())).is_err());
    assert_eq!(
        super::coordinator::ToolLifecycleCoordinator
            .run(vec![AppId::Gemini], || Ok(()))
            .await
            .unwrap_err(),
        "TOOL_ACTION_IN_PROGRESS"
    );
    // No subscriber owns or can abort the worker.
    op.cancel();
    let mut events = op.subscribe();
    tokio::time::timeout(Duration::from_secs(5), async {
        while events.borrow().clone().phase != Phase::Terminal {
            events.changed().await.unwrap();
        }
    })
    .await
    .unwrap();
    assert_eq!(op.snapshot().status, TaskStatus::Cancelled);
    let child: i32 = std::fs::read_to_string(pid).unwrap().trim().parse().unwrap();
    assert_eq!(unsafe { libc::kill(child, 0) }, -1);
    super::coordinator::ToolLifecycleCoordinator.run(vec![AppId::Gemini], || Ok(())).await.unwrap();
    tokio::time::sleep(Duration::from_millis(500)).await;
    assert!(!marker.exists(), "descendant escaped cancellation");
}

#[cfg(unix)]
#[tokio::test]
async fn cc_apps_operation_cancel_reaps_process_success_failure_and_preflight() {
    for (script, expected) in
        [("printf 'token=secret'", TaskStatus::Completed), ("exit 17", TaskStatus::Failed)]
    {
        let op = Operation::new();
        let observed = op.clone();
        op.start(vec![AppId::Codex], move || {
            assert_eq!(observed.snapshot().phase, Phase::Preflight);
            let mut cmd = Command::new("/bin/sh");
            cmd.args(["-c", script]);
            let result = process::run_command(cmd);
            assert_eq!(observed.snapshot().phase, Phase::Postflight);
            if let Ok((_, output)) = &result {
                assert!(!output.contains("secret"));
            }
            result.map(|_| ())
        })
        .unwrap();
        assert_eq!(op.wait().await.status, expected);
        assert_eq!(op.subscribe().borrow().status, expected);
    }
    let op = Operation::new();
    op.cancel();
    op.start(vec![AppId::Codex], || panic!("cancelled before preflight")).unwrap();
    assert_eq!(op.cancel_and_wait().await.status, TaskStatus::Cancelled);
    let op = Operation::new();
    op.start(vec![AppId::Codex], || panic!("fixture panic")).unwrap();
    assert_eq!(op.wait().await.status, TaskStatus::Failed);
}

#[tokio::test]
async fn cc_apps_operation_cancel_reaps_process_existing_install_context() {
    let op = Operation::new();
    op.cancel();
    op.start_action(async {
        process::install(
            AppId::Dsh,
            super::native_primary::InstallEnvironment::Posix,
            || panic!("cancelled before Node gate"),
            || panic!("cancelled before probe"),
        )
        .await
        .map(|_| ())
    })
    .unwrap();
    assert_eq!(op.wait().await.status, TaskStatus::Cancelled);
}
