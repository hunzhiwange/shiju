//! Read-only macOS Codex client hints; never acknowledges or restarts a process.
use super::catalog_generation::Snapshot;
use std::{
    path::Path,
    process::Stdio,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use tokio::{io::AsyncReadExt, process::Command};

const OUTPUT_LIMIT: usize = 4 * 1024 * 1024;
const TIMEOUT: Duration = Duration::from_secs(2);

/// Execution-host observations, never values accepted from an HTTP request.
/// The daemon PID must come from the authorized Codex home's daemon record.
pub struct ProcessIdentity<'a> {
    pub daemon_pid: Option<u32>,
    pub plugins_dir: &'a Path,
}
#[derive(Debug, PartialEq, Eq)]
pub struct ClientsState {
    pub revision: String,
    pub daemon: bool,
    pub others: bool,
    pub auth: bool,
    pub unverified: bool,
}

/// Call after authorizing the host/project and reading its guarded Snapshot.
/// Fixed executable, bounded output and timeout; raw process arguments never leave this module.
pub async fn read(
    snapshot: &Snapshot,
    check_catalog: bool,
    identity: ProcessIdentity<'_>,
) -> ClientsState {
    let table = process_table().await;
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()
        .and_then(|d| u64::try_from(d.as_millis()).ok());
    inspect(
        snapshot,
        check_catalog,
        identity,
        table.as_deref().filter(|_| now.is_some()),
        now.unwrap_or_default(),
    )
}

pub(super) async fn process_table() -> Option<String> {
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

/// The process table and clock are replaceable host boundaries. No payload is retained.
/// Flags identify inferred stale processes, never proof of their in-memory credentials.
pub fn inspect(
    snapshot: &Snapshot,
    check_catalog: bool,
    identity: ProcessIdentity<'_>,
    table: Option<&str>,
    now_ms: u64,
) -> ClientsState {
    let mut state = ClientsState {
        revision: snapshot.revision(),
        daemon: false,
        others: false,
        auth: false,
        unverified: snapshot.unverified(),
    };
    if !identity.plugins_dir.is_absolute()
        || identity.plugins_dir.file_name() != Some(std::ffi::OsStr::new("plugins"))
        || identity
            .plugins_dir
            .components()
            .any(|part| matches!(part, std::path::Component::ParentDir))
        || identity.daemon_pid == Some(0)
    {
        state.unverified = true;
        return state;
    }
    let Some(table) = table.filter(|t| t.len() <= OUTPUT_LIMIT) else {
        state.unverified = true;
        return state;
    };
    // External file changes have no observed generation. Do not upgrade a hint into a stale verdict.
    if state.unverified {
        return state;
    }
    for line in table.lines().filter(|l| !l.trim().is_empty()) {
        let Some((pid, elapsed, command)) = split_row(line) else {
            state.unverified = true;
            continue;
        };
        let Some((executable, daemon)) = app_server(command) else {
            continue;
        };
        if daemon {
            if identity.daemon_pid != Some(pid) {
                continue;
            }
        } else if Path::new(executable).starts_with(identity.plugins_dir) {
            continue;
        }
        let Some(started_ms) = elapsed.checked_mul(1000).and_then(|e| now_ms.checked_sub(e)) else {
            state.unverified = true;
            continue;
        };
        let auth = snapshot.login_stale_at(started_ms);
        let stale = auth || (check_catalog && snapshot.catalog_stale_at(started_ms));
        state.daemon |= daemon && stale;
        state.others |= !daemon && stale;
        state.auth |= auth;
    }
    state
}

fn split_row(line: &str) -> Option<(u32, u64, &str)> {
    let (pid, rest) = line.trim_start().split_once(char::is_whitespace)?;
    let (elapsed, command) = rest.trim_start().split_once(char::is_whitespace)?;
    let pid = pid.parse::<u32>().ok().filter(|p| *p != 0)?;
    Some((pid, elapsed_seconds(elapsed)?, command.trim_start()))
}

fn elapsed_seconds(value: &str) -> Option<u64> {
    let (days, clock) = match value.split_once('-') {
        Some((days, clock)) => (days.parse::<u64>().ok()?, clock),
        None => (0, value),
    };
    let parts: Vec<_> = clock.split(':').collect();
    if !(2..=3).contains(&parts.len()) || (days > 0 && parts.len() != 3) {
        return None;
    }
    let mut seconds = days.checked_mul(86_400)?;
    for (i, part) in parts.iter().enumerate() {
        if part.is_empty() || !part.bytes().all(|b| b.is_ascii_digit()) {
            return None;
        }
        let n = part.parse::<u64>().ok()?;
        if (i > 0 && n >= 60) || (i == 0 && parts.len() == 3 && days > 0 && n >= 24) {
            return None;
        }
        // Days already represent seconds; combine the clock separately below.
    }
    let clock = parts
        .iter()
        .try_fold(0u64, |s, p| s.checked_mul(60)?.checked_add(p.parse::<u64>().ok()?))?;
    seconds = seconds.checked_add(clock)?;
    Some(seconds)
}

/// ps flattens argv. Accept the executable prefix only, never a later `codex` argument.
/// Absolute executable paths may contain spaces (as in macOS application bundles).
fn app_server(command: &str) -> Option<(&str, bool)> {
    let end = if command.starts_with("codex ") {
        5
    } else {
        if !command.starts_with('/') {
            return None;
        }
        command.find("/codex ")? + 6
    };
    let executable = &command[..end];
    if executable.contains(" /") || executable.chars().any(char::is_control) {
        return None;
    }
    let mut words = command[end..].split_whitespace();
    let subcommand = loop {
        match words.next()? {
            "-c" | "--config" => {
                words.next()?;
            }
            flag if flag.starts_with('-') => {}
            word => break word,
        }
    };
    if subcommand != "app-server" {
        return None;
    }
    let rest: Vec<_> = words.collect();
    if matches!(rest.first(), Some(&"daemon" | &"proxy")) {
        return None;
    }
    Some((executable, rest.contains(&"--managed-daemon")))
}
