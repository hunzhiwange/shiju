//! An application-owned foreground Gateway; never a service or a user instance.
use std::{
    net::SocketAddr,
    process::{Child, Command, Stdio},
    time::Duration,
};

pub(crate) struct OwnedGateway {
    command: Option<Command>,
    child: Option<Child>,
    address: SocketAddr,
}
impl OwnedGateway {
    pub(crate) fn new(mut command: Command, address: SocketAddr) -> Self {
        command.stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null());
        #[cfg(unix)]
        {
            use std::os::unix::process::CommandExt;
            command.process_group(0);
        }
        Self { command: Some(command), child: None, address }
    }
    pub(crate) async fn start(&mut self) -> anyhow::Result<()> {
        #[cfg(not(unix))]
        anyhow::bail!("owned Gateway process containment is unverified on this platform");
        #[cfg(unix)]
        {
            let mut command =
                self.command.take().ok_or_else(|| anyhow::anyhow!("Gateway already started"))?;
            self.child = Some(
                command
                    .spawn()
                    .map_err(|_| anyhow::anyhow!("unable to start owned OpenClaw Gateway"))?,
            );
            let deadline = tokio::time::Instant::now() + Duration::from_secs(30);
            loop {
                anyhow::ensure!(
                    self.child.as_mut().unwrap().try_wait()?.is_none(),
                    "owned OpenClaw Gateway exited before readiness"
                );
                if tokio::net::TcpStream::connect(self.address).await.is_ok() {
                    // The bridge still authenticates with this instance's fresh token.
                    return Ok(());
                }
                anyhow::ensure!(
                    tokio::time::Instant::now() < deadline,
                    "owned OpenClaw Gateway readiness timed out"
                );
                tokio::time::sleep(Duration::from_millis(50)).await;
            }
        }
    }
}
impl Drop for OwnedGateway {
    fn drop(&mut self) {
        if let Some(mut child) = self.child.take() {
            #[cfg(unix)]
            unsafe {
                libc::kill(-(child.id() as i32), libc::SIGKILL);
            }
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

#[cfg(all(test, unix))]
#[path = "gateway_tests.rs"]
mod tests;
