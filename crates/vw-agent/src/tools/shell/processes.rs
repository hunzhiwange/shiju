//! Application-owned shell processes. No caller runtime owns a child or its I/O.
use anyhow::{Context, bail};
use parking_lot::Mutex;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::{Arc, OnceLock};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};
use tokio::io::AsyncReadExt;
use tokio::sync::{Notify, mpsc, oneshot};
#[path = "output.rs"]
mod output;
use output::{OutputBuffer, OutputDecoder};
use tokio::task::JoinSet;
use tokio_util::sync::CancellationToken;

const SESSION_LIMIT: usize = 32;
const GLOBAL_LIMIT: usize = 128;
const TERMINAL_LIMIT: usize = 512;
const RETENTION: Duration = Duration::from_secs(600);

/// Must be captured from trusted ingress before crossing into the host thread.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ShellProcessScope {
    tenant: String,
    workspace: PathBuf,
    session: String,
}

impl ShellProcessScope {
    pub(crate) fn new(tenant: String, workspace: &Path, session: String) -> anyhow::Result<Self> {
        if tenant.is_empty() || session.is_empty() {
            bail!("Shell process requires a trusted tenant and session");
        }
        let workspace = workspace.canonicalize().context("Invalid shell process workspace")?;
        if !workspace.is_dir() {
            bail!("Shell process workspace must be a directory");
        }
        Ok(Self { tenant, workspace, session })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ShellProcessState {
    Running,
    Exited,
    Cancelled,
    TimedOut,
}

#[derive(Debug, Clone)]
pub(crate) struct ShellProcessSnapshot {
    pub(crate) process_id: String,
    pub(crate) state: ShellProcessState,
    pub(crate) exit_code: Option<i32>,
    pub(crate) output: String,
    pub(crate) next_cursor: u64,
    pub(crate) truncated: bool,
    /// Total evicted display bytes when the requested cursor has a gap.
    pub(crate) dropped_bytes: u64,
}

struct Entry {
    scope: ShellProcessScope,
    snapshot: ShellProcessSnapshot,
    finished: Option<Instant>,
    output: Arc<Mutex<OutputBuffer>>,
    changed: Arc<Notify>,
}

type Records = Arc<Mutex<HashMap<String, Entry>>>;
type BuildCommand = Box<dyn FnOnce() -> anyhow::Result<tokio::process::Command> + Send>;

struct Start {
    scope: ShellProcessScope,
    build: BuildCommand,
    reply: oneshot::Sender<anyhow::Result<ShellProcessSnapshot>>,
}

/// Own this at application scope, independently of transient tool instances.
/// Drop and shutdown both join the host thread after all children are reaped.
pub(crate) struct ShellProcesses {
    sender: mpsc::Sender<Start>,
    records: Records,
    stopping: CancellationToken,
    thread: Mutex<Option<JoinHandle<()>>>,
}

impl ShellProcesses {
    pub(crate) fn new() -> anyhow::Result<Self> {
        let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build()?;
        let records = Records::default();
        let stopping = CancellationToken::new();
        let (sender, receiver) = mpsc::channel(128);
        let host_records = records.clone();
        let host_stopping = stopping.clone();
        let thread =
            std::thread::Builder::new().name("shell-processes".into()).spawn(move || {
                runtime.block_on(serve(receiver, host_records, host_stopping));
            })?;
        Ok(Self { sender, records, stopping, thread: Mutex::new(Some(thread)) })
    }

    /// Shared by rebuilt toolsets; explicit shutdown remains available to the app.
    pub(crate) fn shared() -> anyhow::Result<&'static Self> {
        static HOST: OnceLock<Result<ShellProcesses, String>> = OnceLock::new();
        HOST.get_or_init(|| Self::new().map_err(|error| error.to_string()))
            .as_ref()
            .map_err(|error| anyhow::anyhow!("Shell process host unavailable: {error}"))
    }

    /// The closure must capture validated policy and environment, never a live child.
    /// It runs inside the dedicated runtime, including command construction and spawn.
    pub(crate) async fn start(
        &self,
        scope: ShellProcessScope,
        build: impl FnOnce() -> anyhow::Result<tokio::process::Command> + Send + 'static,
    ) -> anyhow::Result<ShellProcessSnapshot> {
        let (reply, response) = oneshot::channel();
        self.sender
            .send(Start { scope, build: Box::new(build), reply })
            .await
            .map_err(|_| anyhow::anyhow!("Shell process host stopped"))?;
        response.await.context("Shell process host stopped")?
    }

    pub(crate) fn snapshot(
        &self,
        scope: &ShellProcessScope,
        process_id: &str,
    ) -> anyhow::Result<ShellProcessSnapshot> {
        let mut records = self.records.lock();
        prune(&mut records);
        match records.get(process_id) {
            Some(entry) if &entry.scope == scope => snapshot_output(entry, 0),
            _ => bail!("Shell process unavailable"),
        }
    }

    /// Cursor is an observed-chunk sequence number; zero starts at retained output.
    /// Numeric parsing belongs to the tool protocol boundary.
    pub(crate) async fn wait(
        &self,
        scope: &ShellProcessScope,
        process_id: &str,
        cursor: u64,
        yield_time: Duration,
    ) -> anyhow::Result<ShellProcessSnapshot> {
        let changed = {
            let records = self.records.lock();
            match records.get(process_id) {
                Some(entry) if &entry.scope == scope => entry.changed.clone(),
                _ => bail!("Shell process unavailable"),
            }
        };
        let deadline = tokio::time::Instant::now() + yield_time;
        loop {
            // Register before reading to avoid losing broadcasts between read and wait.
            let notified = changed.notified();
            tokio::pin!(notified);
            notified.as_mut().enable();
            let snapshot = {
                let mut records = self.records.lock();
                prune(&mut records);
                match records.get(process_id) {
                    Some(entry) if &entry.scope == scope => snapshot_output(entry, cursor)?,
                    _ => bail!("Shell process unavailable"),
                }
            };
            if snapshot.next_cursor != cursor
                || snapshot.state != ShellProcessState::Running
                || tokio::time::Instant::now() >= deadline
            {
                return Ok(snapshot);
            }
            let _ = tokio::time::timeout_at(deadline, notified).await;
        }
    }

    pub(crate) fn shutdown(&self) -> anyhow::Result<()> {
        self.stopping.cancel();
        if let Some(thread) = self.thread.lock().take() {
            thread.join().map_err(|_| anyhow::anyhow!("Shell process host panicked"))?;
        }
        Ok(())
    }
}

impl Drop for ShellProcesses {
    fn drop(&mut self) {
        let _ = self.shutdown();
    }
}

fn prune(records: &mut HashMap<String, Entry>) {
    records.retain(|_, entry| entry.finished.is_none_or(|time| time.elapsed() < RETENTION));
    let mut terminal: Vec<_> = records
        .iter()
        .filter_map(|(id, entry)| entry.finished.map(|time| (id.clone(), time)))
        .collect();
    if terminal.len() > TERMINAL_LIMIT {
        terminal.sort_unstable_by_key(|(_, time)| *time);
        let excess = terminal.len() - TERMINAL_LIMIT;
        for (id, _) in terminal.into_iter().take(excess) {
            records.remove(&id);
        }
    }
}

async fn serve(mut receiver: mpsc::Receiver<Start>, records: Records, stopping: CancellationToken) {
    let mut tasks = JoinSet::new();
    let mut cleanup = tokio::time::interval(Duration::from_secs(60));
    loop {
        tokio::select! {
            biased;
            _ = stopping.cancelled() => break,
            Some(_) = tasks.join_next(), if !tasks.is_empty() => {},
            _ = cleanup.tick() => prune(&mut records.lock()),
            request = receiver.recv() => {
                let Some(request) = request else { break };
                let result = start_child(&request.scope, request.build, &records, &stopping, &mut tasks);
                if let Err(Ok(snapshot)) = request.reply.send(result) {
                    // Detached callers retain their scope and application-owned cleanup.
                    tracing::debug!(process_id = %snapshot.process_id, "shell start caller detached");
                }
            }
        }
    }
    receiver.close();
    stopping.cancel();
    while tasks.join_next().await.is_some() {}
}

fn start_child(
    scope: &ShellProcessScope,
    build: BuildCommand,
    records: &Records,
    stopping: &CancellationToken,
    tasks: &mut JoinSet<()>,
) -> anyhow::Result<ShellProcessSnapshot> {
    {
        let mut entries = records.lock();
        prune(&mut entries);
        let running = entries.values().filter(|entry| entry.finished.is_none());
        if running.clone().count() >= GLOBAL_LIMIT
            || running.filter(|entry| &entry.scope == scope).count() >= SESSION_LIMIT
        {
            bail!("Shell process capacity reached");
        }
    }
    let mut command = build().context("Failed to build shell process")?;
    command.stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::piped()).kill_on_drop(true);
    #[cfg(unix)]
    command.process_group(0);
    let mut child = command.spawn().context("Failed to spawn shell process")?;
    let pid = child.id();
    let stdout = child.stdout.take();
    let stderr = child.stderr.take();
    let snapshot = ShellProcessSnapshot {
        process_id: uuid::Uuid::new_v4().to_string(),
        state: ShellProcessState::Running,
        exit_code: None,
        output: String::new(),
        next_cursor: 0,
        truncated: false,
        dropped_bytes: 0,
    };
    let output = Arc::new(Mutex::new(OutputBuffer::default()));
    let changed = Arc::new(Notify::new());
    records.lock().insert(
        snapshot.process_id.clone(),
        Entry {
            scope: scope.clone(),
            snapshot: snapshot.clone(),
            finished: None,
            output: output.clone(),
            changed: changed.clone(),
        },
    );
    let records = records.clone();
    let stopping = stopping.clone();
    let id = snapshot.process_id.clone();
    tasks.spawn(async move {
        let drain_stdout = tokio::spawn(drain(stdout, output.clone(), changed.clone()));
        let drain_stderr = tokio::spawn(drain(stderr, output, changed.clone()));
        let (state, status) = tokio::select! {
            biased;
            _ = stopping.cancelled() => {
                kill_group(pid);
                let _ = child.start_kill();
                (ShellProcessState::Cancelled, child.wait().await)
            }
            status = child.wait() => (ShellProcessState::Exited, status),
        };
        // A shell may exit while descendants still hold its output pipes open.
        kill_group(pid);
        let _ = drain_stdout.await;
        let _ = drain_stderr.await;
        let mut entries = records.lock();
        if let Some(entry) = entries.get_mut(&id) {
            entry.snapshot.state = state;
            entry.snapshot.exit_code = status.ok().and_then(|status| status.code());
            entry.finished = Some(Instant::now());
        }
        prune(&mut entries);
        changed.notify_waiters();
    });
    Ok(snapshot)
}

fn snapshot_output(entry: &Entry, cursor: u64) -> anyhow::Result<ShellProcessSnapshot> {
    let read = entry.output.lock().read(cursor)?;
    let mut snapshot = entry.snapshot.clone();
    snapshot.output = read.output;
    snapshot.next_cursor = read.next_cursor;
    snapshot.truncated = read.truncated;
    snapshot.dropped_bytes = read.dropped_bytes;
    Ok(snapshot)
}

async fn drain(
    pipe: Option<impl tokio::io::AsyncRead + Unpin>,
    output: Arc<Mutex<OutputBuffer>>,
    changed: Arc<Notify>,
) {
    let Some(mut pipe) = pipe else { return };
    let mut decoder = OutputDecoder::default();
    let mut bytes = [0; 8192];
    loop {
        match pipe.read(&mut bytes).await {
            Ok(0) | Err(_) => {
                decoder.push(&[], true, &mut output.lock());
                changed.notify_waiters();
                break;
            }
            Ok(count) => {
                decoder.push(&bytes[..count], false, &mut output.lock());
                changed.notify_waiters();
            }
        }
    }
}

fn kill_group(pid: Option<u32>) {
    #[cfg(unix)]
    if let Some(pid) = pid.and_then(|pid| i32::try_from(pid).ok()) {
        // SAFETY: the child was placed in its own process group at spawn. A negative
        // PID targets only that group; no pointers or shared memory are accessed.
        unsafe {
            libc::kill(-pid, libc::SIGKILL);
        }
    }
    #[cfg(not(unix))]
    let _ = pid;
}

#[cfg(test)]
#[path = "processes_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "output_tests.rs"]
mod output_tests;
