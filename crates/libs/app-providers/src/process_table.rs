//! Bounded macOS process table observation.
use std::{
    process::Stdio,
    time::Duration,
};
use tokio::{io::AsyncReadExt, process::Command};

const OUTPUT_LIMIT: usize = 4 * 1024 * 1024;
const TIMEOUT: Duration = Duration::from_secs(2);

pub async fn process_table() -> Option<String> {
    let mut child = Command::new("/bin/ps")
        .args(["-ww", "-Ao", "pid=,etime=,command="])
        .env("LC_ALL", "C")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .kill_on_drop(true)
        .spawn()
        .ok()?;
    let result = tokio::time::timeout(TIMEOUT, async {
        let mut bytes = Vec::new();
        child.stdout.take()?.take((OUTPUT_LIMIT + 1) as u64).read_to_end(&mut bytes).await.ok()?;
        if bytes.len() > OUTPUT_LIMIT || !child.wait().await.ok()?.success() {
            return None;
        }
        String::from_utf8(bytes).ok()
    })
    .await
    .ok()
    .flatten();
    if result.is_none() {
        let _ = child.kill().await;
        let _ = child.wait().await;
    }
    result
}
