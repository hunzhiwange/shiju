//! Process-group lifecycle shared by cancellation, deadlines and host shutdown.
use std::time::Duration;
use tokio::process::{Child, Command};

pub(super) fn prepare(command: &mut Command) -> anyhow::Result<()> {
    #[cfg(unix)]
    {
        let program = std::path::Path::new(command.as_std().get_program())
            .file_name()
            .and_then(|name| name.to_str());
        if matches!(program, Some("docker" | "podman" | "ssh")) {
            anyhow::bail!("Shell process runtime cannot guarantee remote process-tree termination");
        }
        command.process_group(0);
        Ok(())
    }
    #[cfg(not(unix))]
    {
        let _ = command;
        anyhow::bail!("Managed shell process-tree termination is unsupported on this platform");
    }
}

pub(super) fn signal(pid: Option<u32>, force: bool) {
    #[cfg(unix)]
    if let Some(pid) = pid.and_then(|pid| i32::try_from(pid).ok()).filter(|pid| *pid > 0) {
        // SAFETY: prepare places the child in a private group. Negative PID signals
        // only that group, never the application's own group.
        unsafe {
            libc::kill(-pid, if force { libc::SIGKILL } else { libc::SIGTERM });
        }
    }
    #[cfg(not(unix))]
    let _ = (pid, force);
}

pub(super) async fn terminate(
    child: &mut Child,
    pid: Option<u32>,
) -> std::io::Result<std::process::ExitStatus> {
    signal(pid, false);
    // Keep the group alive through the grace interval so the shell can reap its
    // children and flush TERM handlers before escalation.
    tokio::time::sleep(Duration::from_millis(250)).await;
    signal(pid, true);
    child.wait().await
}
