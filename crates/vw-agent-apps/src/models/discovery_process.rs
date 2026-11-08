//! Fixed model-list subprocesses on the authorized macOS execution host.

use std::{future::Future, process::Stdio, time::Duration};

use tokio::{
    io::AsyncReadExt,
    process::{Child, Command},
};

const TIMEOUT: Duration = Duration::from_secs(20);
const OUTPUT_LIMIT: usize = 1024 * 1024;

pub(super) async fn output(
    cancelled: impl Future<Output = ()>,
    prepare: impl FnOnce() -> Result<std::process::Command, &'static str>,
) -> Result<String, &'static str> {
    let mut child = None;
    let result = tokio::select! {
        biased;
        () = cancelled => Err("cancelled"),
        result = tokio::time::timeout(TIMEOUT, async { run(prepare()?, &mut child).await }) => {
            result.unwrap_or(Err("timed_out"))
        }
    };
    if result.is_err()
        && let Some(mut child) = child
    {
        child.kill_group();
        let _ = child.0.kill().await;
    }
    result
}

struct ModelProcess(Child);

impl ModelProcess {
    fn kill_group(&self) {
        if let Some(pid) = self.0.id() {
            // SAFETY: the unreaped CLI owns the new group created by process_group(0).
            unsafe { libc::kill(-(pid as i32), libc::SIGKILL) };
        }
    }
}

impl Drop for ModelProcess {
    fn drop(&mut self) {
        self.kill_group();
    }
}

async fn run(
    command: std::process::Command,
    process: &mut Option<ModelProcess>,
) -> Result<String, &'static str> {
    let child = Command::from(command)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .process_group(0)
        .kill_on_drop(true)
        .spawn()
        .map_err(|_| "unavailable")?;
    let child = &mut process.insert(ModelProcess(child)).0;
    let stdout = child.stdout.take().ok_or("unavailable")?;
    let mut bytes = Vec::new();
    stdout
        .take((OUTPUT_LIMIT + 1) as u64)
        .read_to_end(&mut bytes)
        .await
        .map_err(|_| "unavailable")?;
    if bytes.len() > OUTPUT_LIMIT {
        return Err("invalid_response");
    }
    if !child.wait().await.map_err(|_| "unavailable")?.success() {
        return Err("unavailable");
    }
    String::from_utf8(bytes).map_err(|_| "invalid_response")
}
