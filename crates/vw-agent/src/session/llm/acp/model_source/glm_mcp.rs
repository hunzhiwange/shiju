//! Host-owned peers for the managed GLM HTTP projection.
use agent_client_protocol::McpServer;
use std::{path::Path, time::Duration};
use tokio::process::Command;
use tokio_util::sync::CancellationToken;

pub(super) async fn open_selected_peer(
    definition: &McpServer,
    timeout: Duration,
    closed: CancellationToken,
) -> anyhow::Result<mcp_client::RunningMcpPeer> {
    tokio::select! {
        biased;
        () = closed.cancelled() => anyhow::bail!("GLM MCP selection was revoked"),
        result = async {
            match definition {
                McpServer::Stdio(server) => mcp_client::client::open_isolated_stdio_peer(
                    isolated_stdio_command(server)?, timeout, closed.clone(),
                ).await,
                McpServer::Http(_) => crate::tools::mcp_common::open_connection_check_peer(
                    definition.clone(), timeout, closed.clone(), None, None,
                ).await,
                _ => anyhow::bail!("GLM MCP projection requires HTTP or isolated stdio"),
            }
        } => result,
    }
}

/// Only system executables and libraries are visible. Session services receive
/// an empty, private scratch filesystem and no host or relay environment.
fn isolated_stdio_command(
    server: &agent_client_protocol::McpServerStdio,
) -> anyhow::Result<Command> {
    anyhow::ensure!(
        cfg!(all(
            target_os = "linux",
            target_env = "gnu",
            any(target_arch = "x86_64", target_arch = "aarch64")
        )),
        "GLM stdio isolation requires Linux GNU"
    );
    anyhow::ensure!(server.env.is_empty(), "GLM stdio environment overrides are unavailable");
    let executable = Path::new(&server.command);
    anyhow::ensure!(executable.is_absolute(), "GLM stdio requires an absolute executable");
    let executable = executable.canonicalize()?;
    anyhow::ensure!(
        executable.starts_with("/usr/bin") && executable.is_file(),
        "GLM isolated stdio requires a system executable under /usr/bin"
    );

    let bwrap = Path::new("/usr/bin/bwrap");
    anyhow::ensure!(bwrap.is_file(), "GLM isolated stdio requires /usr/bin/bwrap");
    let mut command = Command::new(bwrap);
    command.env_clear().current_dir("/").args([
        "--unshare-all",
        "--die-with-parent",
        "--new-session",
        "--ro-bind",
        "/usr",
        "/usr",
        "--dev",
        "/dev",
        "--proc",
        "/proc",
        "--tmpfs",
        "/tmp",
        "--chdir",
        "/tmp",
        "--clearenv",
        "--setenv",
        "HOME",
        "/tmp",
        "--setenv",
        "TMPDIR",
        "/tmp",
        "--setenv",
        "PATH",
        "/usr/bin",
        "--setenv",
        "LANG",
        "C",
    ]);
    for dir in ["/lib", "/lib64", "/bin"] {
        if Path::new(dir).is_dir() {
            command.args(["--ro-bind", dir, dir]);
        }
    }
    command.arg("--").arg(executable).args(&server.args);
    Ok(command)
}
