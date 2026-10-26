use super::tests::{command, wait_for};
use super::*;

fn scope(root: &Path) -> ShellProcessScope {
    ShellProcessScope::new("tenant".into(), root, "session".into()).unwrap()
}

fn pid(path: &Path) -> i32 {
    wait_for(|| std::fs::read_to_string(path).is_ok_and(|s| s.trim().parse::<i32>().is_ok()));
    std::fs::read_to_string(path).unwrap().trim().parse().unwrap()
}

fn assert_gone(pid: i32) {
    wait_for(|| unsafe { libc::kill(pid, 0) } == -1);
    assert_eq!(std::io::Error::last_os_error().raw_os_error(), Some(libc::ESRCH));
}

#[test]
fn shell_cancel_stops_descendants() {
    let root = tempfile::tempdir().unwrap();
    let owner = scope(root.path());
    let host = ShellProcesses::new().unwrap();
    let rt = tokio::runtime::Runtime::new().unwrap();
    let child_pid = root.path().join("child");
    let script = format!(
        "trap 'wait; printf tail; exit 9' TERM; sleep 60 & echo $! > '{}'; printf ready; wait",
        child_pid.display()
    );
    let started = rt.block_on(host.start(owner.clone(), command(script))).unwrap();
    let descendant = pid(&child_pid);
    let result = rt.block_on(host.cancel(&owner, &started.process_id)).unwrap();
    assert_eq!(result.state, ShellProcessState::Cancelled);
    assert!(result.output.contains("ready"));
    assert!(result.output.contains("tail"));
    assert_eq!(result.exit_code, Some(9));
    assert_gone(descendant);
    eprintln!("cancel: descendant reaped, TERM tail retained, exit=9");
}

#[test]
fn shell_cancel_is_idempotent() {
    let root = tempfile::tempdir().unwrap();
    let owner = scope(root.path());
    let host = ShellProcesses::new().unwrap();
    let rt = tokio::runtime::Runtime::new().unwrap();
    let started = rt.block_on(host.start(owner.clone(), command("exec sleep 60".into()))).unwrap();
    let first = rt.block_on(host.cancel(&owner, &started.process_id)).unwrap();
    let second = rt.block_on(host.cancel(&owner, &started.process_id)).unwrap();
    assert_eq!(first.state, ShellProcessState::Cancelled);
    assert_eq!(first.state, second.state);
    assert_eq!(first.exit_code, second.exit_code);
    let done = rt.block_on(host.start(owner.clone(), command("exit 7".into()))).unwrap();
    wait_for(|| {
        host.snapshot(&owner, &done.process_id).unwrap().state == ShellProcessState::Exited
    });
    assert_eq!(rt.block_on(host.cancel(&owner, &done.process_id)).unwrap().exit_code, Some(7));
}

#[test]
fn shell_cancel_rejects_other_scope() {
    let root = tempfile::tempdir().unwrap();
    let owner = scope(root.path());
    let host = ShellProcesses::new().unwrap();
    let rt = tokio::runtime::Runtime::new().unwrap();
    let started = rt.block_on(host.start(owner.clone(), command("exec sleep 60".into()))).unwrap();
    let unknown = rt.block_on(host.cancel(&owner, "unknown")).unwrap_err().to_string();
    for (tenant, session) in [("other", "session"), ("tenant", "other")] {
        let foreign = ShellProcessScope::new(tenant.into(), root.path(), session.into()).unwrap();
        assert_eq!(
            rt.block_on(host.cancel(&foreign, &started.process_id)).unwrap_err().to_string(),
            unknown
        );
    }
    assert_eq!(
        host.snapshot(&owner, &started.process_id).unwrap().state,
        ShellProcessState::Running
    );
}

#[test]
fn shell_deadline_is_distinct_from_yield() {
    let root = tempfile::tempdir().unwrap();
    let owner = scope(root.path());
    let host = ShellProcesses::new().unwrap();
    let rt = tokio::runtime::Runtime::new().unwrap();
    let started = rt
        .block_on(host.start_with_timeout(
            owner.clone(),
            Some(Duration::from_millis(300)),
            command("exec sleep 60".into()),
        ))
        .unwrap();
    assert_eq!(
        rt.block_on(host.wait(&owner, &started.process_id, 0, Duration::from_millis(10)))
            .unwrap()
            .state,
        ShellProcessState::Running
    );
    let result =
        rt.block_on(host.wait(&owner, &started.process_id, 0, Duration::from_secs(10))).unwrap();
    assert_eq!(result.state, ShellProcessState::TimedOut);
    assert_ne!(result.exit_code, Some(0));
}

#[test]
fn shell_shutdown_reaps_children() {
    let root = tempfile::tempdir().unwrap();
    let owner = scope(root.path());
    let path = root.path().join("pid");
    let host = ShellProcesses::new().unwrap();
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(host.start(
        owner,
        command(format!("echo $$ > '{}'; trap '' TERM; exec sleep 60", path.display())),
    ))
    .unwrap();
    let child = pid(&path);
    host.shutdown().unwrap();
    assert_gone(child);
    eprintln!("shutdown: TERM-resistant child killed and reaped");
}

#[test]
fn shell_cancel_rejects_remote_runtime_before_spawn() {
    let root = tempfile::tempdir().unwrap();
    let owner = scope(root.path());
    let host = ShellProcesses::new().unwrap();
    let rt = tokio::runtime::Runtime::new().unwrap();
    for program in ["docker", "podman", "ssh"] {
        let error = rt
            .block_on(host.start(owner.clone(), move || Ok(tokio::process::Command::new(program))))
            .unwrap_err();
        assert!(error.to_string().contains("cannot guarantee"));
    }
    assert!(host.records.lock().is_empty());
}

#[test]
fn shell_cancel_wakes_waiters() {
    let root = tempfile::tempdir().unwrap();
    let owner = scope(root.path());
    let host = ShellProcesses::new().unwrap();
    let rt = tokio::runtime::Runtime::new().unwrap();
    let started = rt.block_on(host.start(owner.clone(), command("exec sleep 60".into()))).unwrap();
    let (waiting, cancelled) = rt.block_on(async {
        tokio::join!(
            host.wait(&owner, &started.process_id, 0, Duration::from_secs(30)),
            host.cancel(&owner, &started.process_id)
        )
    });
    assert_eq!(waiting.unwrap().state, ShellProcessState::Cancelled);
    assert_eq!(cancelled.unwrap().state, ShellProcessState::Cancelled);
}
