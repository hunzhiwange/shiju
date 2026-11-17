//! Gateway-side owner of a private control App lease. Dropping it closes the
//! bootstrap channel and kills the child; no reconnection can revive its ticket.
use super::*;
use computer_control::service::{
    ServiceBootstrap, ServiceCommand, ServiceImageChunk, ServiceReady,
};
use std::os::fd::{AsRawFd, FromRawFd};
use std::os::unix::process::CommandExt;

struct Lease {
    channel: UnixStream,
    stream: UnixStream,
    child: std::process::Child,
    ready: ServiceReady,
    sequence: u64,
}
impl Drop for Lease {
    fn drop(&mut self) {
        // Let service shutdown drop drag guards and release held buttons first.
        unsafe {
            libc::shutdown(self.channel.as_raw_fd(), libc::SHUT_RDWR);
        }
        for _ in 0..50 {
            if matches!(self.child.try_wait(), Ok(Some(_))) {
                return;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}
pub(super) struct ServiceBackend {
    config: ComputerControlConfig,
    target: ComputerControlTarget,
    cancel: CancellationToken,
    lease: Arc<Mutex<Option<Lease>>>,
    images: parking_lot::Mutex<HashMap<String, (ScreenshotReference, String)>>,
}
impl ServiceBackend {
    pub fn new(
        config: ComputerControlConfig,
        target: ComputerControlTarget,
        cancel: CancellationToken,
    ) -> Self {
        let lease = Arc::new(Mutex::new(None));
        let cleanup = lease.clone();
        let stop = cancel.clone();
        tokio::spawn(async move {
            stop.cancelled().await;
            cleanup.lock().await.take();
        });
        Self { config, target, cancel, lease, images: parking_lot::Mutex::new(HashMap::new()) }
    }
    async fn launch(&self) -> anyhow::Result<Lease> {
        let executable = computer_control::service::executable()?;
        let (parent, child_socket) = std::os::unix::net::UnixStream::pair()?;
        // Duplicate above fd 3 to avoid accidentally replacing the source in pre_exec.
        let fd = unsafe { libc::fcntl(child_socket.as_raw_fd(), libc::F_DUPFD_CLOEXEC, 4) };
        if fd < 0 {
            return Err(std::io::Error::last_os_error().into());
        }
        let inherited = unsafe { std::os::fd::OwnedFd::from_raw_fd(fd) };
        let mut command = std::process::Command::new(executable);
        command
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null());
        unsafe {
            command.pre_exec(move || {
                if libc::dup2(fd, 3) < 0 {
                    return Err(std::io::Error::last_os_error());
                }
                Ok(())
            });
        }
        let child = command.spawn()?;
        drop(inherited);
        drop(child_socket);
        parent.set_nonblocking(true)?;
        let channel = UnixStream::from_std(parent)?;
        // Install the child guard before any cancellable handshake await.
        let (placeholder, _) = UnixStream::pair()?;
        let mut lease = Lease {
            channel,
            stream: placeholder,
            child,
            ready: ServiceReady {
                socket_path: Default::default(),
                session_id: ControlSessionId(String::new()),
                credential: String::new(),
                permission_version: PermissionVersion(self.config.permission_version),
            },
            sequence: 1,
        };
        write_frame(
            &mut lease.channel,
            &ServiceBootstrap {
                config: self.config.clone(),
                target: self.target.clone(),
                lifetime_seconds: 300,
            },
        )
        .await?;
        lease.ready = read_frame(&mut lease.channel).await?;
        anyhow::ensure!(
            lease.ready.permission_version == PermissionVersion(self.config.permission_version),
            "control service policy mismatch"
        );
        lease.stream = UnixStream::connect(&lease.ready.socket_path).await?;
        write_frame(
            &mut lease.stream,
            &HostMessage::Hello {
                session_id: lease.ready.session_id.clone(),
                credential: lease.ready.credential.clone(),
            },
        )
        .await?;
        anyhow::ensure!(
            matches!(read_frame::<_, HostReply>(&mut lease.stream).await?, HostReply::Ready),
            "control service handshake rejected"
        );
        Ok(lease)
    }
    async fn action(&self, action: ComputerControlAction) -> NativeResult<ControlOutput> {
        let operation = async {
            let mut slot = self.lease.lock().await;
            if slot.is_none() {
                *slot = Some(tokio::time::timeout(Duration::from_secs(10), self.launch()).await??);
            }
            let lease = slot.as_mut().context("control service disconnected")?;
            let id = ControlRequestId(uuid::Uuid::new_v4().to_string());
            let request = ComputerControlRequest {
                request_id: id.clone(),
                session_id: lease.ready.session_id.clone(),
                permission_version: lease.ready.permission_version,
                target: self.target.clone(),
                action,
            };
            let sequence = lease.sequence;
            lease.sequence = sequence.checked_add(1).context("control sequence exhausted")?;
            write_frame(&mut lease.stream, &HostMessage::Action { sequence, request }).await?;
            let output = match read_frame::<_, HostReply>(&mut lease.stream).await? {
                HostReply::Completed {
                    request_id,
                    outcome: ControlOutcome::Succeeded { output },
                } if request_id == id => output,
                _ => bail!("control service rejected action"),
            };
            self.images.lock().clear();
            if let ControlOutput::Observation { screenshots, .. } = &output {
                for reference in screenshots {
                    let mut data = String::new();
                    loop {
                        write_frame(
                            &mut lease.channel,
                            &ServiceCommand::Resolve {
                                reference: reference.clone(),
                                offset: data.len(),
                            },
                        )
                        .await?;
                        let chunk: ServiceImageChunk = read_frame(&mut lease.channel).await?;
                        anyhow::ensure!(
                            !chunk.data.is_empty()
                                && data.len() + chunk.data.len() <= 24 * 1024 * 1024,
                            "invalid service image"
                        );
                        data.push_str(&chunk.data);
                        if chunk.done {
                            break;
                        }
                    }
                    self.images
                        .lock()
                        .insert(reference.media_id().to_string(), (reference.clone(), data));
                }
            }
            Ok::<_, anyhow::Error>(output)
        };
        tokio::select! { biased;
            _ = self.cancel.cancelled() => Err(ControlErrorCode::Cancelled.into()),
            result = operation => result.map_err(|_| NativeObservationError::Control(ControlErrorCode::BackendFailure)),
        }
    }
}
#[async_trait::async_trait]
impl NativeBackend for ServiceBackend {
    async fn observe(&self, screenshot: bool) -> NativeResult<ControlOutput> {
        self.action(ComputerControlAction::NativeObserve { screenshot }).await
    }
    async fn input(&self, input: &NativeInput) -> NativeResult<()> {
        self.action(ComputerControlAction::NativeInput { input: input.clone() }).await.map(|_| ())
    }
    fn resolve(&self, reference: &ScreenshotReference) -> NativeResult<String> {
        if self.cancel.is_cancelled() {
            return Err(ControlErrorCode::Cancelled.into());
        }
        self.images
            .lock()
            .get(reference.media_id())
            .filter(|(stored, _)| stored == reference)
            .map(|(_, data)| data.clone())
            .ok_or(ControlErrorCode::PermissionDenied.into())
    }
}

impl Drop for ServiceBackend {
    fn drop(&mut self) {
        self.cancel.cancel();
    }
}
