//! App-bundle executable. The gateway must pass a connected Unix socket as fd 3.
//! Credentials and policy never pass through argv, environment, or persistent files.
#[cfg(unix)]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    use computer_control::{
        host::{ControlHost, read_frame, write_frame},
        native::NativeObserver,
        policy::ComputerControlPolicy,
        service::{
            ControlService, ServiceBootstrap, ServiceCommand, ServiceImageChunk, ServiceReady,
        },
    };
    use parking_lot::RwLock;
    use std::{
        os::fd::FromRawFd,
        sync::{Arc, atomic::AtomicBool},
        time::Duration,
    };
    use tokio_util::sync::CancellationToken;
    use vw_api_computer_control::{ComputerControlAction, ControlErrorCode};
    if let Some(mode) = std::env::args().nth(1) {
        let permissions = match mode.as_str() {
            "--permissions" => computer_control::native::native_permissions(),
            "--request-accessibility" => computer_control::native::request_native_permission(
                computer_control::native::NativePermission::Accessibility,
            ),
            "--request-screen-recording" => computer_control::native::request_native_permission(
                computer_control::native::NativePermission::ScreenRecording,
            ),
            _ => return Err("unknown control App mode".into()),
        }
        .map_err(|_| "native permissions unavailable")?;
        println!(
            "{}",
            serde_json::to_string(&vw_api_computer_control::NativePermissionReport {
                executable: std::env::current_exe()?.display().to_string(),
                accessibility: permissions.accessibility,
                screen_recording: permissions.screen_recording,
            })?
        );
        return Ok(());
    }
    let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build()?;
    runtime.block_on(async {
        if unsafe { libc::fcntl(3, libc::F_GETFD) } == -1 {
            return Err(std::io::Error::last_os_error().into());
        }
        // SAFETY: fd 3 is open under the launch contract; this process takes sole ownership.
        let channel = unsafe { std::os::unix::net::UnixStream::from_raw_fd(3) };
        channel.set_nonblocking(true)?;
        let mut channel = tokio::net::UnixStream::from_std(channel)?;
        if channel.peer_cred()?.uid() != unsafe { libc::geteuid() } {
            return Err(std::io::Error::new(std::io::ErrorKind::PermissionDenied, "launcher uid mismatch").into());
        }
        let bootstrap: ServiceBootstrap = tokio::time::timeout(Duration::from_secs(5), read_frame(&mut channel)).await??;
        let policy = Arc::new(RwLock::new(ComputerControlPolicy::new(bootstrap.config)));
        let locked = Arc::new(AtomicBool::new(false));
        let host = Arc::new(ControlHost::new(policy.clone(), locked.clone()));
        let ticket = host.issue(bootstrap.target, Duration::from_secs(bootstrap.lifetime_seconds))
            .map_err(|_| std::io::Error::new(std::io::ErrorKind::PermissionDenied, "session denied"))?;
        let root = tempfile::Builder::new().prefix("shiju-control-").tempdir()?;
        let service = ControlService::bind(&root.path().join("session"))?;
        write_frame(&mut channel, &ServiceReady { socket_path: service.socket_path().into(), session_id: ticket.session_id.clone(), credential: ticket.credential, permission_version: ticket.permission_version }).await?;
        let shutdown = CancellationToken::new();
        let observer = Arc::new(tokio::sync::OnceCell::<Arc<NativeObserver>>::new());
        let shared = observer.clone();
        let execute = move |request: vw_api_computer_control::ComputerControlRequest, cancel| {
            let policy = policy.clone();
            let locked = locked.clone();
            let shared = shared.clone();
            async move {
                let observer = shared.get_or_try_init(|| async {
                    NativeObserver::new(policy, locked, request.session_id, request.permission_version, request.target, cancel).map(Arc::new).map_err(|e| e.code())
                }).await?;
                match request.action {
                    ComputerControlAction::NativeObserve { screenshot } => observer.observe(screenshot).await.map_err(|e| e.code()),
                    ComputerControlAction::NativeInput { input } => {
                        observer.execute_input(&input).await.map_err(|e| e.code())?;
                        Ok(vw_api_computer_control::ControlOutput::Acknowledged)
                    }
                    _ => Err(ControlErrorCode::Unsupported),
                }
            }
        };
        // Launcher death/revocation closes the private channel; session expiry also
        // terminates the app. Keeping this channel open is required for the lease.
        let running = service.run(host.clone(), shutdown.clone(), execute);
        tokio::pin!(running);
        tokio::select! {
            result = &mut running => result?,
            _ = async {
                loop {
                    let Ok(ServiceCommand::Resolve { reference, offset }) = read_frame::<_, ServiceCommand>(&mut channel).await else { break };
                    let Some(observer) = observer.get() else { break };
                    let Ok(image) = observer.resolve_screenshot(&reference) else { break };
                    if offset > image.len() || !image.is_char_boundary(offset) { break; }
                    let end = (offset + 512 * 1024).min(image.len());
                    let chunk = ServiceImageChunk { data: image[offset..end].into(), done: end == image.len() };
                    if write_frame(&mut channel, &chunk).await.is_err() { break; }
                }
            } => {
                host.stop(&ticket.session_id);
                shutdown.cancel();
                running.await?;
            }
            _ = tokio::time::sleep(Duration::from_secs(bootstrap.lifetime_seconds)) => {
                host.stop(&ticket.session_id);
                shutdown.cancel();
                running.await?;
            }
        }
        Ok(())
    })
}
#[cfg(not(unix))]
fn main() {
    eprintln!("computer control service requires Unix");
    std::process::exit(1);
}
