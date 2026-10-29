use super::tests::wait_for;
use super::*;
use std::os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle};
use windows_sys::Win32::Foundation::{WAIT_OBJECT_0, WAIT_TIMEOUT};
use windows_sys::Win32::System::Threading::{
    OpenProcess, PROCESS_SYNCHRONIZE, WaitForSingleObject,
};

const FIXTURE: &str =
    "tools::shell::processes::termination_windows_tests::shell_process_windows_fixture";
const ROLE: &str = "VW_SHELL_TREE_TEST_ROLE";

#[test]
fn shell_process_windows_fixture() {
    let Ok(role) = std::env::var(ROLE) else { return };
    if role == "leaf" {
        std::thread::sleep(Duration::from_secs(60));
        return;
    }
    let mut child = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", FIXTURE, "--nocapture"])
        .env(ROLE, "leaf")
        .spawn()
        .unwrap();
    let ready = PathBuf::from(std::env::var_os("VW_SHELL_TREE_TEST_READY").unwrap());
    std::fs::write(&ready, child.id().to_string()).unwrap();
    if role == "exit" {
        // Intentionally leave a descendant holding the inherited output pipes.
        let release = ready.with_extension("release");
        wait_for(|| release.exists());
        std::process::exit(7);
    }
    child.wait().unwrap();
}

fn fixture(
    ready: PathBuf,
    role: &'static str,
) -> impl FnOnce() -> anyhow::Result<tokio::process::Command> + Send {
    move || {
        let mut command = tokio::process::Command::new(std::env::current_exe()?);
        command
            .args(["--exact", FIXTURE, "--nocapture"])
            .env(ROLE, role)
            .env("VW_SHELL_TREE_TEST_READY", ready);
        Ok(command)
    }
}

fn descendant(ready: &Path) -> OwnedHandle {
    wait_for(|| std::fs::read_to_string(ready).is_ok_and(|pid| pid.parse::<u32>().is_ok()));
    let pid = std::fs::read_to_string(ready).unwrap().parse().unwrap();
    // SAFETY: request only synchronization access; never signal a numeric PID.
    let raw = unsafe { OpenProcess(PROCESS_SYNCHRONIZE, 0, pid) };
    assert!(!raw.is_null(), "{}", std::io::Error::last_os_error());
    // SAFETY: OpenProcess returned a fresh handle owned by this value.
    let handle = unsafe { OwnedHandle::from_raw_handle(raw) };
    // SAFETY: the owned process handle remains valid during the nonblocking probe.
    assert_eq!(unsafe { WaitForSingleObject(handle.as_raw_handle(), 0) }, WAIT_TIMEOUT);
    handle
}

#[test]
fn shell_process_windows_cleanup_stops_descendants() {
    for action in ["cancel", "timeout", "shutdown", "exit"] {
        let root = tempfile::tempdir().unwrap();
        let owner = ShellProcessScope::new("tenant".into(), root.path(), action.into()).unwrap();
        let host = ShellProcesses::new().unwrap();
        let rt = tokio::runtime::Runtime::new().unwrap();
        let ready = root.path().join("ready");
        let limit = (action == "timeout").then_some(Duration::from_secs(3));
        let started = rt
            .block_on(host.start_with_timeout(
                owner.clone(),
                limit,
                fixture(ready.clone(), if action == "exit" { "exit" } else { "root" }),
            ))
            .unwrap();
        let child = descendant(&ready);
        let expected = match action {
            "cancel" => {
                let result = rt.block_on(host.cancel(&owner, &started.process_id)).unwrap();
                assert_eq!(result.state, ShellProcessState::Cancelled);
                // Repeated cancellation must remain safe after the Job handle closes.
                assert_eq!(
                    rt.block_on(host.cancel(&owner, &started.process_id)).unwrap().state,
                    result.state
                );
                ShellProcessState::Cancelled
            }
            "shutdown" => {
                host.shutdown().unwrap();
                ShellProcessState::Cancelled
            }
            "exit" => {
                std::fs::write(ready.with_extension("release"), "release").unwrap();
                ShellProcessState::Exited
            }
            _ => ShellProcessState::TimedOut,
        };
        wait_for(|| {
            host.snapshot(&owner, &started.process_id).unwrap().state != ShellProcessState::Running
        });
        let result = host.snapshot(&owner, &started.process_id).unwrap();
        assert_eq!(result.state, expected, "{action}");
        if action == "exit" {
            assert_eq!(result.exit_code, Some(7));
        }
        // SAFETY: this retained handle identifies the original descendant even after PID reuse.
        assert_eq!(
            unsafe { WaitForSingleObject(child.as_raw_handle(), 5000) },
            WAIT_OBJECT_0,
            "{action}"
        );
    }
}

#[test]
fn shell_process_windows_rejects_remote_runtime_before_spawn() {
    let root = tempfile::tempdir().unwrap();
    let owner = ShellProcessScope::new("tenant".into(), root.path(), "remote".into()).unwrap();
    let host = ShellProcesses::new().unwrap();
    let rt = tokio::runtime::Runtime::new().unwrap();
    for program in ["docker.exe", "PODMAN.EXE", "ssh.exe"] {
        let error = rt
            .block_on(host.start(owner.clone(), move || Ok(tokio::process::Command::new(program))))
            .unwrap_err();
        assert!(error.to_string().contains("cannot guarantee"));
    }
    assert!(host.records.lock().is_empty());
}
