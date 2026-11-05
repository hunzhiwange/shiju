//! Replace a local CLI gateway occupying the requested address before startup.

use anyhow::{Context, Result, bail};
use std::net::{SocketAddr, TcpListener};
#[cfg(unix)]
use std::time::Duration;

pub(crate) async fn prepare_gateway_port(host: &str, port: u16) -> Result<()> {
    let address: SocketAddr = format!("{host}:{port}").parse()?;
    match TcpListener::bind(address) {
        Ok(_) => return Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::AddrInUse => {}
        Err(error) => return Err(error).context("Failed to probe gateway address"),
    }

    tokio::task::spawn_blocking(move || replace_gateway(address)).await?
}

#[cfg(unix)]
fn replace_gateway(address: SocketAddr) -> Result<()> {
    use std::process::Command;

    let output = Command::new("lsof")
        .args(["-nP", "-t", &format!("-iTCP:{}", address.port()), "-sTCP:LISTEN"])
        .output()
        .context("Failed to inspect occupied gateway port with lsof")?;
    let mut pids = String::from_utf8(output.stdout)?
        .lines()
        .map(str::parse::<u32>)
        .collect::<std::result::Result<Vec<_>, _>>()?;
    pids.sort_unstable();
    pids.dedup();
    let [pid] = pids.as_slice() else {
        bail!("Gateway address {address} is occupied; cannot identify a unique gateway process");
    };
    let pid = *pid;
    verify_gateway_process(pid)?;
    tracing::info!(pid, %address, "Stopping existing ShiJu gateway");
    signal_process(pid, libc::SIGTERM)?;
    if wait_for_port(address, Duration::from_secs(5))? {
        return Ok(());
    }
    // Recheck before escalating: the original process may already have exited.
    verify_gateway_process(pid)?;
    signal_process(pid, libc::SIGKILL)?;
    if !wait_for_port(address, Duration::from_secs(2))? {
        bail!("Gateway address {address} is still occupied after stopping process {pid}");
    }
    Ok(())
}

#[cfg(not(unix))]
fn replace_gateway(address: SocketAddr) -> Result<()> {
    bail!("Gateway address {address} is occupied; stop the existing gateway before restarting")
}

#[cfg(unix)]
fn verify_gateway_process(pid: u32) -> Result<()> {
    use std::process::Command;

    if pid <= 1 || pid == std::process::id() {
        bail!("Refusing to stop gateway process {pid}");
    }
    let read_field = |field: &str| -> Result<String> {
        let output = Command::new("ps")
            .args(["-ww", "-p", &pid.to_string(), "-o", field])
            .output()
            .context("Failed to inspect gateway process")?;
        if !output.status.success() {
            bail!("Gateway process {pid} disappeared during inspection; retry startup");
        }
        Ok(String::from_utf8(output.stdout)?.trim().to_owned())
    };
    let executable = read_field("comm=")?;
    let command = read_field("args=")?;
    if !is_gateway_command(&executable, &command) {
        bail!(
            "Port is occupied by process {pid}, which is not a ShiJu CLI gateway; refusing to stop it"
        );
    }
    // Only replace processes owned by the current user.
    let owner = read_field("uid=")?.parse::<u32>()?;
    if owner != unsafe { libc::geteuid() } {
        bail!("Refusing to stop gateway process {pid} owned by another user");
    }
    Ok(())
}

#[cfg(unix)]
fn is_gateway_command(executable: &str, command: &str) -> bool {
    let name = std::path::Path::new(executable).file_name().and_then(|name| name.to_str());
    if !matches!(name, Some("shiju" | "vibe-agent")) {
        return false;
    }
    let arguments =
        command.strip_prefix(executable).filter(|args| args.starts_with(' ')).or_else(|| {
            let (program, arguments) = command.split_once(char::is_whitespace)?;
            (std::path::Path::new(program).file_name().and_then(|name| name.to_str()) == name)
                .then_some(arguments)
        });
    let Some(arguments) = arguments else {
        return false;
    };
    let mut arguments = arguments.split_whitespace();
    while let Some(argument) = arguments.next() {
        match argument {
            "--config-dir" => {
                if arguments.next().is_none() {
                    return false;
                }
            }
            argument if argument.starts_with("--config-dir=") => {}
            "gateway" => return true,
            _ => return false,
        }
    }
    false
}

#[cfg(unix)]
fn signal_process(pid: u32, signal: i32) -> Result<()> {
    let pid = i32::try_from(pid).context("Gateway process ID is out of range")?;
    if unsafe { libc::kill(pid, signal) } != 0 {
        let error = std::io::Error::last_os_error();
        if error.raw_os_error() != Some(libc::ESRCH) {
            return Err(error).context("Failed to stop existing gateway");
        }
    }
    Ok(())
}

#[cfg(unix)]
fn wait_for_port(address: SocketAddr, timeout: Duration) -> Result<bool> {
    let deadline = std::time::Instant::now() + timeout;
    loop {
        match TcpListener::bind(address) {
            Ok(_) => return Ok(true),
            Err(error) if error.kind() == std::io::ErrorKind::AddrInUse => {}
            Err(error) => return Err(error).context("Failed to probe gateway address"),
        }
        if std::time::Instant::now() >= deadline {
            return Ok(false);
        }
        std::thread::sleep(Duration::from_millis(50));
    }
}

#[cfg(all(test, unix))]
#[path = "gateway_process_tests.rs"]
mod tests;
