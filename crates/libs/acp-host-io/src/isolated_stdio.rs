//! ACP host entry points for isolated fixed executables.

pub use process_sandbox::bubblewrap::{ISOLATED_UNIX_RELAY_SOCKET, ISOLATED_UNIX_RELAY_WORKSPACE};
use process_sandbox::bubblewrap::{isolated_stdio_command, isolated_unix_relay_command};
use std::ffi::OsString;
use std::io;
use std::path::Path;
use std::process::ExitStatus;
use tokio::process::{Child, ChildStderr, ChildStdin, ChildStdout, Command};

/// The sandbox supervisor owns all namespace members. Dropping it requests
/// termination. Call `cancel` to await supervisor reaping.
pub struct IsolatedStdio(Child);

/// Spawn the isolated supervisor. A successful spawn is not a readiness signal:
/// Bubblewrap setup failures produce stderr and a failed exit and never launch
/// the entry outside the sandbox. Use the child's protocol to establish readiness.
pub fn spawn_isolated_stdio(executable: &Path, args: &[OsString]) -> io::Result<IsolatedStdio> {
    spawn(isolated_stdio_command(executable, args)?)
}

/// Spawn a caller-reviewed executable with one request-owned Unix relay.
/// Give it the sandbox workspace in its protocol requests. The socket must be
/// owned by the host user with mode 0600 inside a request directory of mode 0700.
/// Establish readiness through the child protocol and keep the listener
/// private and alive for the request. The host remains responsible for grants.
pub fn spawn_isolated_stdio_with_unix_relay(
    executable: &Path,
    args: &[OsString],
    relay_socket: &Path,
) -> io::Result<IsolatedStdio> {
    spawn(isolated_unix_relay_command(executable, args, relay_socket)?)
}

fn spawn(command: std::process::Command) -> io::Result<IsolatedStdio> {
    let mut command = Command::from(command);
    command.kill_on_drop(true);
    command.spawn().map(IsolatedStdio)
}

impl IsolatedStdio {
    pub fn take_stdin(&mut self) -> Option<ChildStdin> {
        self.0.stdin.take()
    }

    pub fn take_stdout(&mut self) -> Option<ChildStdout> {
        self.0.stdout.take()
    }

    pub fn take_stderr(&mut self) -> Option<ChildStderr> {
        self.0.stderr.take()
    }

    pub async fn wait(&mut self) -> io::Result<ExitStatus> {
        self.0.wait().await
    }

    pub async fn cancel(&mut self) -> io::Result<()> {
        self.kill_group();
        self.0.kill().await
    }

    fn kill_group(&self) {
        if let Some(pid) = self.0.id() {
            // The launcher creates its session before executing Bubblewrap.
            // Killing this group also covers cancellation during setup.
            unsafe { libc::kill(-(pid as libc::pid_t), libc::SIGKILL) };
        }
    }
}

impl Drop for IsolatedStdio {
    fn drop(&mut self) {
        self.kill_group();
    }
}
