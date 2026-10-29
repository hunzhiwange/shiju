//! Process-tree lifecycle shared by cancellation, deadlines and host shutdown.
use anyhow::Context;
#[cfg(unix)]
use std::time::Duration;
use tokio::process::{Child, Command};

#[cfg(windows)]
#[path = "termination_windows.rs"]
mod windows;

pub(super) struct ProcessTree {
    #[cfg(unix)]
    pid: Option<u32>,
    #[cfg(windows)]
    job: windows::JobObject,
}

pub(super) fn spawn(command: &mut Command) -> anyhow::Result<(Child, ProcessTree)> {
    let program_path = std::path::Path::new(command.as_std().get_program());
    #[cfg(windows)]
    let program = program_path.file_stem().and_then(|name| name.to_str());
    #[cfg(not(windows))]
    let program = program_path.file_name().and_then(|name| name.to_str());
    if program.is_some_and(|name| {
        ["docker", "podman", "ssh"].iter().any(|remote| {
            #[cfg(windows)]
            {
                name.eq_ignore_ascii_case(remote)
            }
            #[cfg(not(windows))]
            {
                name == *remote
            }
        })
    }) {
        anyhow::bail!("Shell process runtime cannot guarantee remote process-tree termination");
    }
    #[cfg(unix)]
    {
        command.process_group(0);
        let child = command.spawn().context("Failed to spawn shell process")?;
        let tree = ProcessTree { pid: child.id() };
        Ok((child, tree))
    }
    #[cfg(windows)]
    {
        let job = windows::JobObject::new().context("Failed to create shell process job")?;
        let child = job.spawn(command).context("Failed to spawn contained shell process")?;
        Ok((child, ProcessTree { job }))
    }
    #[cfg(not(any(unix, windows)))]
    {
        let _ = command;
        anyhow::bail!("Managed shell process-tree termination is unsupported on this platform");
    }
}

#[cfg(unix)]
fn signal(pid: Option<u32>, force: bool) {
    if let Some(pid) = pid.and_then(|pid| i32::try_from(pid).ok()).filter(|pid| *pid > 0) {
        // SAFETY: spawn places the child in a private group. Negative PID signals
        // only that group, never the application's own group.
        unsafe {
            libc::kill(-pid, if force { libc::SIGKILL } else { libc::SIGTERM });
        }
    }
}

impl ProcessTree {
    pub(super) async fn terminate(
        &self,
        child: &mut Child,
    ) -> std::io::Result<std::process::ExitStatus> {
        #[cfg(unix)]
        {
            signal(self.pid, false);
            // Let the shell reap its children and flush TERM handlers before escalation.
            tokio::time::sleep(Duration::from_millis(250)).await;
            signal(self.pid, true);
        }
        #[cfg(windows)]
        self.job.terminate()?;
        child.wait().await
    }
}

impl Drop for ProcessTree {
    fn drop(&mut self) {
        #[cfg(unix)]
        signal(self.pid, true);
        // On Windows the owned job kills all remaining members when its handle closes.
    }
}
