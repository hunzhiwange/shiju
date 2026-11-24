//! Gateway-owned workbench sessions. The first release targets macOS. Other Unix
//! hosts compile the same backend, and non-Unix hosts have no native backend.
//! Sessions and their byte cursors live only for the lifetime of the gateway.
use crate::terminal_session::{ChildGroup, open_pty};
use std::{
    collections::VecDeque,
    fs::{File, Permissions},
    io::{self, Read, Write},
    os::{
        fd::{AsRawFd, RawFd},
        unix::fs::PermissionsExt,
    },
    path::Path,
    process::{Command, ExitStatus},
    sync::{Arc, Condvar, Mutex, Weak, mpsc},
    thread::{self, JoinHandle},
    time::Duration,
};

pub const REPLAY_CAPACITY: usize = 256 * 1024;
const POLL_INTERVAL: Duration = Duration::from_millis(10);

#[derive(Clone, Copy, Debug)]
pub struct PtySize {
    pub rows: u16,
    pub cols: u16,
}

impl PtySize {
    fn validate(self) -> io::Result<()> {
        if self.rows == 0 || self.cols == 0 {
            return Err(io::Error::new(io::ErrorKind::InvalidInput, "empty terminal size"));
        }
        Ok(())
    }
}

/// An owned host process. Dropping its owner cancels and reaps the child.
pub struct WorkbenchPty {
    pid: u32,
    shared: Arc<Shared>,
    worker: Option<JoinHandle<()>>,
}

/// Ephemeral access to one session. Each attachment has its own read cursor.
pub struct PtyAttachment {
    session: Weak<Shared>,
    cursor: u64,
}

impl PtyAttachment {
    pub fn cursor(&self) -> u64 {
        self.cursor
    }

    /// Dropping an attachment has the same effect as detaching explicitly.
    pub fn detach(self) {}
}

#[derive(Debug)]
pub struct PtyOutput {
    pub bytes: Vec<u8>,
    /// Absolute byte offset of the first returned byte.
    pub start: u64,
    pub next: u64,
    /// The requested cursor fell before the oldest retained byte.
    pub truncated: bool,
    /// The process has finished and all retained bytes are in this or earlier reads.
    pub finished: bool,
}

struct Shared {
    state: Mutex<State>,
    changed: Condvar,
}

struct State {
    master: Option<File>,
    replay: VecDeque<u8>,
    next: u64,
    stopping: bool,
    exit: Option<io::Result<ExitStatus>>,
}

impl WorkbenchPty {
    /// Takes a gateway-prepared command with a validated cwd, argv and scoped
    /// environment. Use the host launch builder before adding app credentials.
    /// The runtime root must be owned by the gateway.
    pub fn spawn(mut command: Command, runtime_root: &Path, size: PtySize) -> io::Result<Self> {
        size.validate()?;
        if !command.get_current_dir().is_some_and(|cwd| cwd.is_absolute() && cwd.is_dir()) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "validated launch cwd required",
            ));
        }
        let private = tempfile::Builder::new()
            .prefix("sj-workbench-")
            .permissions(Permissions::from_mode(0o700))
            .tempdir_in(runtime_root)?;
        let (master, slave) = open_pty(size.rows, size.cols)?;
        command.env("TMPDIR", private.path()).env("TERM", "xterm-256color");
        let shared = Arc::new(Shared {
            state: Mutex::new(State {
                master: Some(master),
                replay: VecDeque::new(),
                next: 0,
                stopping: false,
                exit: None,
            }),
            changed: Condvar::new(),
        });
        // Start the worker before spawning the child so a thread creation failure
        // cannot leave a child waiting on a terminal with no reader.
        let worker_state = Arc::clone(&shared);
        let (started, startup) = mpsc::sync_channel(1);
        let worker = thread::Builder::new().name("workbench-pty".into()).spawn(move || {
            match ChildGroup::spawn(&mut command, slave) {
                Ok(child) => {
                    let _ = started.send(Ok(child.id()));
                    run(child, &worker_state, private);
                }
                Err(error) => {
                    let _ = started.send(Err(error));
                }
            }
        })?;
        let pid = match startup.recv() {
            Ok(Ok(pid)) => pid,
            result => {
                let _ = worker.join();
                return Err(result
                    .ok()
                    .and_then(Result::err)
                    .unwrap_or_else(|| io::Error::other("PTY worker exited during startup")));
            }
        };
        Ok(Self { pid, shared, worker: Some(worker) })
    }

    pub fn pid(&self) -> u32 {
        self.pid
    }

    /// Attach at an absolute byte cursor. Use zero to request the retained tail.
    pub fn attach(&self, cursor: u64) -> io::Result<PtyAttachment> {
        if cursor > self.shared.state.lock().unwrap().next {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "PTY cursor is ahead of output",
            ));
        }
        Ok(PtyAttachment { session: Arc::downgrade(&self.shared), cursor })
    }

    /// Block the calling thread for up to `timeout` waiting for output or completion.
    /// A running session without output returns an empty batch. Replay is available after exit.
    pub fn read(&self, attachment: &mut PtyAttachment, timeout: Duration) -> io::Result<PtyOutput> {
        self.check_attachment(attachment)?;
        let state = self.shared.state.lock().unwrap();
        let (state, _) = self
            .shared
            .changed
            .wait_timeout_while(state, timeout, |state| {
                state.next == attachment.cursor && state.exit.is_none()
            })
            .unwrap();
        let oldest = state.next - state.replay.len() as u64;
        let start = attachment.cursor.max(oldest);
        let truncated = start != attachment.cursor;
        let bytes = state.replay.range((start - oldest) as usize..).copied().collect();
        attachment.cursor = state.next;
        Ok(PtyOutput { bytes, start, next: state.next, truncated, finished: state.exit.is_some() })
    }

    /// Nonblocking input. Retry the unwritten suffix after a short write or WouldBlock.
    pub fn write(&self, attachment: &PtyAttachment, bytes: &[u8]) -> io::Result<usize> {
        self.check_attachment(attachment)?;
        let state = self.shared.state.lock().unwrap();
        let mut master = state.live_master()?;
        master.write(bytes).map_err(closed_terminal)
    }

    pub fn resize(&self, attachment: &PtyAttachment, size: PtySize) -> io::Result<()> {
        self.check_attachment(attachment)?;
        size.validate()?;
        let state = self.shared.state.lock().unwrap();
        let master = state.live_master()?;
        let size =
            libc::winsize { ws_row: size.rows, ws_col: size.cols, ws_xpixel: 0, ws_ypixel: 0 };
        // SAFETY: master is live under the state lock, and size is a valid winsize.
        if unsafe { libc::ioctl(master.as_raw_fd(), libc::TIOCSWINSZ as _, &size) } < 0 {
            return Err(closed_terminal(io::Error::last_os_error()));
        }
        Ok(())
    }

    /// The final status becomes visible after output drain and resource cleanup.
    pub fn exit_status(&self) -> io::Result<Option<ExitStatus>> {
        let state = self.shared.state.lock().unwrap();
        state.exit.as_ref().map(copy_exit).transpose()
    }

    /// Stop this process group and wait for cleanup. Repeated calls return the same status.
    pub fn stop(&self) -> io::Result<ExitStatus> {
        self.request_stop();
        let state = self.shared.state.lock().unwrap();
        let state = self.shared.changed.wait_while(state, |state| state.exit.is_none()).unwrap();
        copy_exit(state.exit.as_ref().unwrap())
    }

    fn request_stop(&self) {
        self.shared.state.lock().unwrap().stopping = true;
    }

    fn check_attachment(&self, attachment: &PtyAttachment) -> io::Result<()> {
        if !Weak::ptr_eq(&attachment.session, &Arc::downgrade(&self.shared)) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "attachment belongs to another PTY session",
            ));
        }
        Ok(())
    }
}

impl Drop for WorkbenchPty {
    fn drop(&mut self) {
        self.request_stop();
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

impl State {
    fn live_master(&self) -> io::Result<&File> {
        self.master.as_ref().filter(|_| !self.stopping).ok_or_else(|| {
            io::Error::new(io::ErrorKind::BrokenPipe, "PTY session ended or is stopping")
        })
    }
}

fn copy_exit(exit: &io::Result<ExitStatus>) -> io::Result<ExitStatus> {
    match exit {
        Ok(status) => Ok(*status),
        Err(error) => Err(io::Error::new(error.kind(), error.to_string())),
    }
}

fn closed_terminal(error: io::Error) -> io::Error {
    if error.raw_os_error() == Some(libc::EIO) {
        io::Error::new(io::ErrorKind::BrokenPipe, "PTY terminal closed")
    } else {
        error
    }
}

fn run(mut child: ChildGroup, shared: &Shared, private: tempfile::TempDir) {
    let outcome = monitor(&child, shared);

    // Disable writes and resizes before killing or draining the terminal.
    let mut master = shared.state.lock().unwrap().master.take().unwrap();
    child.terminate();
    let tail = {
        let mut state = shared.state.lock().unwrap();
        let State { replay, next, .. } = &mut *state;
        drain(&mut master, replay, next)
    };
    // Closing the master first releases terminal IO that can block exit on macOS.
    drop(master);
    let status = child.reap();
    let cleanup = private.close();
    let result = outcome.and(status).and_then(|status| tail.and(cleanup).map(|_| status));
    shared.state.lock().unwrap().exit = Some(result);
    shared.changed.notify_all();
}

fn monitor(child: &ChildGroup, shared: &Shared) -> io::Result<()> {
    loop {
        let (fd, full) = {
            let mut state = shared.state.lock().unwrap();
            if state.stopping || child.has_exited()? {
                return Ok(());
            }
            let State { master, replay, next, .. } = &mut *state;
            let master = master.as_mut().unwrap();
            let before = *next;
            let full = drain(master, replay, next)?;
            if *next != before {
                shared.changed.notify_all();
            }
            (master.as_raw_fd(), full)
        };
        if full {
            // Give input and stop callers a chance to acquire the lock under sustained output.
            thread::yield_now();
        } else {
            // Only this worker takes ownership of the master, after monitor returns.
            wait_readable(fd)?;
        }
    }
}

fn wait_readable(fd: RawFd) -> io::Result<()> {
    let mut ready = libc::pollfd { fd, events: libc::POLLIN, revents: 0 };
    // SAFETY: ready is a valid one-element array and the master stays open.
    let result = unsafe { libc::poll(&mut ready, 1, POLL_INTERVAL.as_millis() as _) };
    if result < 0 {
        let error = io::Error::last_os_error();
        if error.kind() != io::ErrorKind::Interrupted {
            return Err(error);
        }
    } else if ready.revents & (libc::POLLHUP | libc::POLLERR) != 0 {
        // The child may still be running after closing its terminal descriptors.
        thread::sleep(POLL_INTERVAL);
    }
    Ok(())
}

/// Drain a bounded batch so continuous output cannot starve stop or input.
fn drain(master: &mut File, replay: &mut VecDeque<u8>, next: &mut u64) -> io::Result<bool> {
    let mut bytes = [0; 8192];
    for _ in 0..8 {
        let count = match master.read(&mut bytes) {
            Ok(0) => return Ok(false),
            Ok(count) => count,
            Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
            Err(error)
                if error.kind() == io::ErrorKind::WouldBlock
                    || error.raw_os_error() == Some(libc::EIO) =>
            {
                return Ok(false);
            }
            Err(error) => return Err(error),
        };
        let discard = (replay.len() + count).saturating_sub(REPLAY_CAPACITY);
        replay.drain(..discard);
        replay.extend(&bytes[..count]);
        *next += count as u64;
    }
    Ok(true)
}
