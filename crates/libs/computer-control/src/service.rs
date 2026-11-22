//! Private Unix listener for an already-authorized ControlHost. Ticket issuance
//! belongs to the trusted launcher, never to a client of this socket.
//!
//! macOS starts the signed app through Launch Services. Both ends authenticate
//! the peer before exchanging ServiceBootstrap or permission reports. Keep the
//! channel open for the grant lease and close it on revocation or shutdown.
//! A same-UID connection to control.sock still needs Hello.
//! The binary dispatches native observation and input with session-scoped media.
use crate::host::ControlHost;
use std::{
    future::Future,
    io,
    os::unix::fs::{DirBuilderExt, MetadataExt, PermissionsExt},
    path::{Path, PathBuf},
    sync::Arc,
};
use tokio::net::UnixListener;
use tokio_util::sync::CancellationToken;
use vw_api_computer_control::{ComputerControlRequest, ControlErrorCode, ControlOutput};

#[cfg(target_os = "macos")]
#[path = "service_macos.rs"]
pub mod macos;

/// Owns a fresh directory: existing paths (including stale sockets and symlinks)
/// are deliberately not reused or deleted. Drop removes only this instance's files.
pub struct ControlService {
    listener: UnixListener,
    directory: PathBuf,
    socket: PathBuf,
}
impl ControlService {
    pub fn bind(directory: &Path) -> io::Result<Self> {
        std::fs::DirBuilder::new().mode(0o700).create(directory)?;
        let socket = directory.join("control.sock");
        let result = (|| {
            let metadata = std::fs::symlink_metadata(directory)?;
            if !metadata.is_dir()
                || metadata.uid() != unsafe { libc::geteuid() }
                || metadata.mode() & 0o777 != 0o700
            {
                return Err(io::Error::new(
                    io::ErrorKind::PermissionDenied,
                    "invalid control directory",
                ));
            }
            let listener = UnixListener::bind(&socket)?;
            std::fs::set_permissions(&socket, std::fs::Permissions::from_mode(0o600))?;
            Ok(listener)
        })();
        match result {
            Ok(listener) => Ok(Self { listener, directory: directory.into(), socket }),
            Err(error) => {
                let _ = std::fs::remove_file(&socket);
                let _ = std::fs::remove_dir(directory);
                Err(error)
            }
        }
    }
    pub fn socket_path(&self) -> &Path {
        &self.socket
    }

    /// Serve connections until shutdown; all host futures are dropped on shutdown,
    /// causing their SessionGuard to cancel in-flight effects. Native drag guards
    /// synchronously release held buttons when these futures drop. No detached tasks.
    pub async fn run<F, Fut>(
        self,
        host: Arc<ControlHost>,
        shutdown: CancellationToken,
        execute: F,
    ) -> io::Result<()>
    where
        F: Fn(ComputerControlRequest, CancellationToken) -> Fut + Clone + Send + 'static,
        Fut: Future<Output = Result<ControlOutput, ControlErrorCode>> + Send + 'static,
    {
        let mut connections = tokio::task::JoinSet::new();
        loop {
            tokio::select! {
                biased;
                _ = shutdown.cancelled() => break,
                Some(_) = connections.join_next(), if !connections.is_empty() => {},
                accepted = self.listener.accept(), if connections.len() < 32 => {
                    let (stream, _) = accepted?;
                    let host = host.clone();
                    let execute = execute.clone();
                    connections.spawn(async move { host.serve(stream, execute).await });
                }
            }
        }
        connections.abort_all();
        while connections.join_next().await.is_some() {}
        Ok(())
    }
}
impl Drop for ControlService {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.socket);
        let _ = std::fs::remove_dir(&self.directory);
    }
}

/// Sent only over a launcher-created inherited socket, never the public listener.
/// The launcher supplies its live policy snapshot and exact target after approval.
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ServiceBootstrap {
    pub config: config_computer_control::ComputerControlConfig,
    pub target: vw_api_computer_control::ComputerControlTarget,
    pub lifetime_seconds: u64,
}

/// Secret-bearing reply; intentionally does not implement Debug.
#[derive(serde::Serialize, serde::Deserialize)]
pub struct ServiceReady {
    pub socket_path: PathBuf,
    pub session_id: vw_api_computer_control::ControlSessionId,
    pub credential: String,
    pub permission_version: vw_api_computer_control::PermissionVersion,
}

/// Private launcher channel operations; this endpoint is never exposed to tools.
#[derive(serde::Serialize, serde::Deserialize)]
pub enum ServiceCommand {
    Resolve { reference: vw_api_computer_control::ScreenshotReference, offset: usize },
}
#[derive(serde::Serialize, serde::Deserialize)]
pub struct ServiceImageChunk {
    pub data: String,
    pub done: bool,
}
/// Locate the independently installed controller. The enclosing ShiJu bundle is only the source.
pub fn executable() -> io::Result<PathBuf> {
    let current = std::env::current_exe()?;
    let contents = current
        .parent()
        .and_then(Path::parent)
        .ok_or_else(|| io::Error::other("missing application bundle"))?;
    let helper = contents.join("Helpers/ShiJu Computer Use.app/Contents/MacOS/shiju-computer-use");
    if !helper.is_file() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            "bundled computer control App unavailable",
        ));
    }
    let helper = helper.canonicalize()?;
    #[cfg(target_os = "macos")]
    if helper.ancestors().skip(4).any(|path| path.extension().is_some_and(|ext| ext == "app")) {
        return Err(io::Error::new(io::ErrorKind::NotFound, "computer use app needs independent installation"));
    }
    Ok(helper)
}
