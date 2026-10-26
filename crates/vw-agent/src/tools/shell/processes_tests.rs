use super::*;
use std::time::{Duration, Instant};

fn command(script: String) -> impl FnOnce() -> anyhow::Result<tokio::process::Command> + Send {
    move || {
        let mut cmd = tokio::process::Command::new("/bin/sh");
        cmd.arg("-c").arg(script).env_clear();
        Ok(cmd)
    }
}
fn wait_for(mut check: impl FnMut() -> bool) {
    let deadline = Instant::now() + Duration::from_secs(10);
    while !check() {
        assert!(Instant::now() < deadline, "handshake timed out");
        std::thread::sleep(Duration::from_millis(10));
    }
}
#[test]
fn shell_process_survives_caller_runtime_drop() {
    let root = tempfile::tempdir().unwrap();
    let owner = ShellProcessScope::new("tenant".into(), root.path(), "session".into()).unwrap();
    let host = ShellProcesses::new().unwrap();
    let ready = root.path().join("ready");
    let release = root.path().join("release");
    let script = format!(
        "touch '{}'; while [ ! -f '{}' ]; do sleep 0.05; done; exit 7",
        ready.display(),
        release.display()
    );
    let caller = tokio::runtime::Runtime::new().unwrap();
    let snapshot = caller.block_on(host.start(owner.clone(), command(script))).unwrap();
    drop(caller);
    wait_for(|| ready.exists());
    assert_eq!(
        host.snapshot(&owner, &snapshot.process_id).unwrap().state,
        ShellProcessState::Running
    );
    std::fs::write(release, "release").unwrap();
    wait_for(|| {
        host.snapshot(&owner, &snapshot.process_id).unwrap().state == ShellProcessState::Exited
    });
    assert_eq!(host.snapshot(&owner, &snapshot.process_id).unwrap().exit_code, Some(7));
    eprintln!("caller runtime dropped; real process exited with code 7");
    host.shutdown().unwrap();
}
#[test]
fn shell_process_scope_rejects_other_owner() {
    let root = tempfile::tempdir().unwrap();
    let other_root = tempfile::tempdir().unwrap();
    let owner = ShellProcessScope::new("tenant".into(), root.path(), "session".into()).unwrap();
    let host = ShellProcesses::new().unwrap();
    let caller = tokio::runtime::Runtime::new().unwrap();
    let snapshot =
        caller.block_on(host.start(owner.clone(), command("exec sleep 60".into()))).unwrap();
    let unknown = host.snapshot(&owner, "unknown").unwrap_err().to_string();
    for (tenant, workspace, session) in [
        ("other", root.path(), "session"),
        ("tenant", root.path(), "other"),
        ("tenant", other_root.path(), "session"),
    ] {
        let foreign = ShellProcessScope::new(tenant.into(), workspace, session.into()).unwrap();
        assert_eq!(host.snapshot(&foreign, &snapshot.process_id).unwrap_err().to_string(), unknown);
    }
    host.shutdown().unwrap();
    assert_eq!(
        host.snapshot(&owner, &snapshot.process_id).unwrap().state,
        ShellProcessState::Cancelled
    );
}
#[test]
fn shell_process_capacity_rejects_without_eviction() {
    let root = tempfile::tempdir().unwrap();
    let host = ShellProcesses::new().unwrap();
    let caller = tokio::runtime::Runtime::new().unwrap();
    let mut processes = Vec::new();
    for session in 0..4 {
        let owner =
            ShellProcessScope::new("tenant".into(), root.path(), format!("session-{session}"))
                .unwrap();
        for _ in 0..32 {
            let snapshot = caller
                .block_on(host.start(owner.clone(), command("exec sleep 60".into())))
                .unwrap();
            processes.push((owner.clone(), snapshot));
        }
        assert!(caller.block_on(host.start(owner, command("exit 0".into()))).is_err());
    }
    let extra = ShellProcessScope::new("tenant".into(), root.path(), "extra".into()).unwrap();
    assert!(caller.block_on(host.start(extra, command("exit 0".into()))).is_err());
    for (owner, snapshot) in &processes {
        assert_eq!(
            host.snapshot(owner, &snapshot.process_id).unwrap().state,
            ShellProcessState::Running
        );
    }
    host.shutdown().unwrap();
    for (owner, snapshot) in &processes {
        assert_eq!(
            host.snapshot(owner, &snapshot.process_id).unwrap().state,
            ShellProcessState::Cancelled
        );
    }
    eprintln!("capacity: 128 running preserved; shutdown reaped 128 children");
}

#[test]
fn shell_process_spawn_failure_preserves_io_error_and_capacity() {
    let root = tempfile::tempdir().unwrap();
    let owner = ShellProcessScope::new("tenant".into(), root.path(), "session".into()).unwrap();
    let host = ShellProcesses::new().unwrap();
    let caller = tokio::runtime::Runtime::new().unwrap();
    let missing = root.path().join("missing-executable");
    let error = caller
        .block_on(host.start(owner.clone(), move || Ok(tokio::process::Command::new(missing))))
        .unwrap_err();
    assert_eq!(
        error.downcast_ref::<std::io::Error>().unwrap().kind(),
        std::io::ErrorKind::NotFound
    );
    assert!(host.records.lock().is_empty());
    let result = caller.block_on(host.start(owner.clone(), command("exit 3".into()))).unwrap();
    wait_for(|| {
        host.snapshot(&owner, &result.process_id).unwrap().state == ShellProcessState::Exited
    });
    assert_eq!(host.snapshot(&owner, &result.process_id).unwrap().exit_code, Some(3));
    // Advance only the retention timestamp after observing a real process exit.
    host.records.lock().get_mut(&result.process_id).unwrap().finished =
        Some(Instant::now() - RETENTION);
    assert!(host.snapshot(&owner, &result.process_id).is_err());
    let next = caller.block_on(host.start(owner, command("exit 0".into()))).unwrap();
    assert_ne!(result.process_id, next.process_id);
    host.shutdown().unwrap();
}

#[cfg(unix)]
#[test]
fn shell_process_shutdown_reaps_real_child() {
    let root = tempfile::tempdir().unwrap();
    let owner = ShellProcessScope::new("tenant".into(), root.path(), "session".into()).unwrap();
    let pid_path = root.path().join("pid");
    let script = format!("echo $$ > '{}'; exec sleep 60", pid_path.display());
    let host = ShellProcesses::new().unwrap();
    let caller = tokio::runtime::Runtime::new().unwrap();
    caller.block_on(host.start(owner, command(script))).unwrap();
    wait_for(|| {
        std::fs::read_to_string(&pid_path).is_ok_and(|pid| pid.trim().parse::<i32>().is_ok())
    });
    let pid: i32 = std::fs::read_to_string(pid_path).unwrap().trim().parse().unwrap();
    // SAFETY: signal 0 probes existence without changing the process.
    assert_eq!(unsafe { libc::kill(pid, 0) }, 0);
    host.shutdown().unwrap();
    // wait() has reaped the direct child before shutdown returns.
    assert_eq!(unsafe { libc::kill(pid, 0) }, -1);
    assert_eq!(std::io::Error::last_os_error().raw_os_error(), Some(libc::ESRCH));
    eprintln!("shutdown: OS confirms direct child reaped (ESRCH)");
}
