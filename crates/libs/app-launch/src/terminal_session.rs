//! Gateway-owned PTY process; only a private socket path reaches the OS terminal.
use super::request::LaunchMaterial;
use std::{
    fs::File,
    io::{self, Read, Write},
    os::{
        fd::{AsRawFd, FromRawFd},
        unix::{
            fs::{OpenOptionsExt, PermissionsExt},
            net::{UnixListener, UnixStream},
            process::CommandExt,
        },
    },
    path::Path,
    process::{Command, ExitStatus, Stdio},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};

pub struct TerminalSession {
    stream: UnixStream,
    private: tempfile::TempDir,
}
impl TerminalSession {
    /// The boundary launches only a credential-free terminal client on the granted host.
    pub fn connect_with(
        root: &Path,
        launch: impl FnOnce(&Path, &LaunchMaterial) -> Result<(), String>,
    ) -> Result<Self, String> {
        let private = tempfile::Builder::new()
            .prefix("sj-tty-")
            .permissions(std::fs::Permissions::from_mode(0o700))
            .tempdir_in(root)
            .map_err(|_| "cannot create private terminal session")?;
        let socket = private.path().join("io");
        let listener =
            UnixListener::bind(&socket).map_err(|_| "cannot bind private terminal session")?;
        listener.set_nonblocking(true).map_err(|_| "cannot configure terminal session")?;
        let nc = ["/usr/bin/nc", "/bin/nc"]
            .into_iter()
            .find(|p| Path::new(p).is_file())
            .ok_or("Unix socket terminal client is unavailable")?;
        let quote = |value: &str| format!("'{}'", value.replace('\'', "'\\''"));
        let socket_text = socket
            .to_str()
            .filter(|s| !s.chars().any(char::is_control))
            .ok_or("invalid terminal session path")?;
        let script = private.path().join("attach.sh");
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o700)
            .open(&script)
            .map_err(|_| "cannot create terminal attach script")?;
        writeln!(file, "#!/bin/sh\nsaved=$(stty -g) || exit 1\ntrap 'stty \"$saved\"' EXIT\ntrap 'exit 1' HUP INT TERM\nstty raw -echo || exit 1\n{} -U {}", quote(nc), quote(socket_text)).map_err(|_| "cannot write terminal attach script")?;
        drop(file);
        let material = LaunchMaterial {
            program: "/bin/sh".into(),
            cwd: private.path().into(),
            args: vec![script.to_str().ok_or("invalid terminal client path")?.into()],
            skip_permissions: false,
        };
        launch(&socket, &material)?;
        let deadline = Instant::now() + Duration::from_secs(30);
        let stream = loop {
            match listener.accept() {
                Ok((stream, _)) => break stream,
                Err(e) if e.kind() == io::ErrorKind::WouldBlock && Instant::now() < deadline => {
                    std::thread::sleep(Duration::from_millis(20))
                }
                _ => return Err("terminal did not attach; CLI was not started".into()),
            }
        };
        drop(listener); // One terminal connection only; no reconnect or replay.
        Ok(Self { stream, private })
    }
    /// Holds the command's owning material in the caller until all IO/child resources are reaped.
    pub fn run(self, command: &mut Command, started: impl FnOnce()) -> io::Result<ExitStatus> {
        let Self { stream, private } = self;
        let (master, slave) = open_pty()?;
        command
            .env("TMPDIR", private.path())
            .env("TERM", "xterm-256color")
            .env("COLUMNS", "80")
            .env("LINES", "24");
        command
            .stdin(Stdio::from(slave.try_clone()?))
            .stdout(Stdio::from(slave.try_clone()?))
            .stderr(Stdio::from(slave.try_clone()?));
        let slave_fd = slave.as_raw_fd();
        // SAFETY: only async-signal-safe libc calls occur between fork and exec; slave is live.
        unsafe {
            command.pre_exec(move || {
                if libc::setsid() < 0 || libc::ioctl(slave_fd, libc::TIOCSCTTY as _, 0) < 0 {
                    return Err(io::Error::last_os_error());
                }
                Ok(())
            });
        }
        stream.set_read_timeout(Some(Duration::from_millis(100)))?;
        stream.set_write_timeout(Some(Duration::from_millis(100)))?;
        // All fallible descriptor setup precedes starting copy workers.
        let input_socket = stream.try_clone()?;
        let input_master = master.try_clone()?;
        let output_socket = stream.try_clone()?;
        let output_master = master.try_clone()?;
        let spawned = command.spawn();
        // Release the command's retained slave descriptors as well as our original copy.
        command.stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null());
        drop(slave);
        let mut child = ChildGroup { child: spawned?, active: true };
        let pid = child.child.id() as libc::pid_t;
        let done = Arc::new(AtomicBool::new(false));
        let disconnected = Arc::new(AtomicBool::new(false));
        started();
        let input = pump(input_socket, input_master, true, done.clone(), disconnected.clone());
        let output = pump(output_master, output_socket, false, done.clone(), disconnected.clone());
        let outcome = loop {
            // WNOWAIT retains the child PID until its whole process group has been stopped.
            let mut info: libc::siginfo_t = unsafe { std::mem::zeroed() };
            let result = unsafe {
                libc::waitid(
                    libc::P_PID,
                    pid as _,
                    &mut info,
                    libc::WEXITED | libc::WNOHANG | libc::WNOWAIT,
                )
            };
            if result != 0 {
                break Err(io::Error::last_os_error());
            }
            if info.si_signo != 0 {
                break Ok(());
            }
            if disconnected.load(Ordering::SeqCst) {
                break Ok(());
            }
            std::thread::sleep(Duration::from_millis(20));
        };
        // Terminate remaining descendants even when the direct CLI has already exited.
        // PID is still reserved by the unreaped child, so it cannot refer to a reused group.
        unsafe {
            libc::kill(-pid, libc::SIGKILL);
        }
        let result = child.child.wait();
        child.active = false;
        // Let already-buffered PTY output reach the terminal before closing its socket.
        std::thread::sleep(Duration::from_millis(40));
        done.store(true, Ordering::SeqCst);
        let _ = stream.shutdown(std::net::Shutdown::Both);
        let _ = input.join();
        let _ = output.join();
        drop(master);
        drop(stream);
        private.close()?;
        outcome?;
        result
    }
}
fn open_pty() -> io::Result<(File, File)> {
    let mut master = -1;
    let mut slave = -1;
    let mut size = libc::winsize { ws_row: 24, ws_col: 80, ws_xpixel: 0, ws_ypixel: 0 };
    // SAFETY: valid out-pointers and winsize; returned descriptors are consumed once.
    if unsafe {
        libc::openpty(
            &mut master,
            &mut slave,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            &mut size,
        )
    } != 0
    {
        return Err(io::Error::last_os_error());
    }
    let pair = unsafe { (File::from_raw_fd(master), File::from_raw_fd(slave)) };
    if unsafe { libc::fchmod(slave, 0o600) } < 0 {
        return Err(io::Error::last_os_error());
    }
    for fd in [master, slave] {
        if unsafe { libc::fcntl(fd, libc::F_SETFD, libc::FD_CLOEXEC) } < 0 {
            return Err(io::Error::last_os_error());
        }
    }
    // Slave remains blocking; master copies periodically observe shutdown.
    if unsafe { libc::fcntl(master, libc::F_SETFL, libc::O_NONBLOCK) } < 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(pair)
}
fn pump(
    mut reader: impl Read + Send + 'static,
    mut writer: impl Write + Send + 'static,
    socket_reader: bool,
    done: Arc<AtomicBool>,
    disconnected: Arc<AtomicBool>,
) -> std::thread::JoinHandle<()> {
    std::thread::spawn(move || {
        let mut bytes = [0; 8192];
        while !done.load(Ordering::SeqCst) {
            let count = match reader.read(&mut bytes) {
                Ok(0) => {
                    if socket_reader {
                        disconnected.store(true, Ordering::SeqCst);
                    }
                    return;
                }
                Ok(n) => n,
                Err(e)
                    if matches!(
                        e.kind(),
                        io::ErrorKind::WouldBlock
                            | io::ErrorKind::TimedOut
                            | io::ErrorKind::Interrupted
                    ) =>
                {
                    std::thread::sleep(Duration::from_millis(5));
                    continue;
                }
                Err(_) => {
                    if socket_reader {
                        disconnected.store(true, Ordering::SeqCst);
                    }
                    return;
                }
            };
            let mut written = 0;
            while written < count && !done.load(Ordering::SeqCst) {
                match writer.write(&bytes[written..count]) {
                    Ok(0) => {
                        if !socket_reader {
                            disconnected.store(true, Ordering::SeqCst);
                        }
                        return;
                    }
                    Ok(n) => written += n,
                    Err(e)
                        if matches!(
                            e.kind(),
                            io::ErrorKind::WouldBlock
                                | io::ErrorKind::TimedOut
                                | io::ErrorKind::Interrupted
                        ) =>
                    {
                        std::thread::sleep(Duration::from_millis(5))
                    }
                    Err(_) => {
                        if !socket_reader {
                            disconnected.store(true, Ordering::SeqCst);
                        }
                        return;
                    }
                }
            }
        }
    })
}

/// Unwinding or a transport setup failure must not leave a running credentialed child.
struct ChildGroup {
    child: std::process::Child,
    active: bool,
}
impl Drop for ChildGroup {
    fn drop(&mut self) {
        if self.active {
            unsafe {
                libc::kill(-(self.child.id() as libc::pid_t), libc::SIGKILL);
            }
            let _ = self.child.wait();
        }
    }
}
