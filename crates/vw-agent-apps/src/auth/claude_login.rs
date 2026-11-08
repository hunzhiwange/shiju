//! Start an official Claude login on its authorized execution host.

use super::{
    claude_login_files::Directory,
    credential_file::{AccountCredential, CredentialFileStore},
    flow::AuthAccountScope,
};
use reqwest::Url;
use std::{
    fs,
    io::{self, Write},
    os::unix::fs::OpenOptionsExt,
    path::Path,
    process::{Command, Stdio},
    time::Duration,
};
use tokio::{
    io::{AsyncBufReadExt, AsyncRead, BufReader},
    process::Child,
    sync::oneshot,
    time::{self, Instant},
};

use vw_api_types::{agent_apps::AppId, agent_apps_auth::AuthService};

const LINK_WAIT: Duration = Duration::from_secs(30);
const BROWSER_GRACE: Duration = Duration::from_secs(3);
const CHECK_INTERVAL: Duration = Duration::from_millis(50);
const OUTPUT_LINE_LIMIT: usize = 16 * 1024;
const CAPTURE_FILE: &str = "opened-url";
const OPENER: &[u8] = br#"#!/bin/sh
set -eu
umask 077
capture="$SHIJU_CLAUDE_LOGIN_CAPTURE"
printf '%s' "$1" > "$capture.$$"
/bin/mv -f "$capture.$$" "$capture"
"#;

#[derive(Debug, PartialEq, Eq)]
pub enum ClaudeLoginError {
    InvalidAccount,
    Unavailable,
    Exited,
    LinkUnavailable,
    TimedOut,
    Cancelled,
}

/// A host-private process handle. Dropping it requests cancellation.
pub struct ClaudeLogin {
    url: String,
    stop: oneshot::Sender<()>,
    done: oneshot::Receiver<Result<(), ClaudeLoginError>>,
}

impl ClaudeLogin {
    /// Use an absolute executable selected by host discovery. The authenticated
    /// owner must already have a managed directory for this account.
    pub async fn start(
        program: &Path,
        store: &CredentialFileStore,
        scope: AuthAccountScope<'_>,
        account_id: &str,
        timeout: Duration,
    ) -> Result<Self, ClaudeLoginError> {
        Self::start_with_command(Command::new(program), store, scope, account_id, timeout).await
    }

    pub(super) async fn start_with_command(
        mut command: Command,
        store: &CredentialFileStore,
        scope: AuthAccountScope<'_>,
        account_id: &str,
        timeout: Duration,
    ) -> Result<Self, ClaudeLoginError> {
        if !Path::new(command.get_program()).is_absolute() {
            return Err(ClaudeLoginError::Unavailable);
        }
        if scope.service != AuthService::ClaudeOfficial || scope.validate().is_err() {
            return Err(ClaudeLoginError::InvalidAccount);
        }
        let lease = store
            .for_owner(scope.principal)
            .and_then(|owner| {
                owner.lease(scope.tenant, scope.host, AppId::Claude, account_id, false)
            })
            .map_err(|_| ClaudeLoginError::InvalidAccount)?;
        let AccountCredential::ClaudeDirectory(directory) =
            lease.read().map_err(|_| ClaudeLoginError::InvalidAccount)?
        else {
            return Err(ClaudeLoginError::InvalidAccount);
        };
        let path = directory.path();
        // Claude's macOS keychain service suffix hashes the config path as NFC.
        if !path.to_str().is_some_and(|path| path.is_ascii() && !path.chars().any(char::is_control))
        {
            return Err(ClaudeLoginError::InvalidAccount);
        }

        let scratch = tempfile::Builder::new()
            .prefix(".shiju-login-")
            .permissions(std::os::unix::fs::PermissionsExt::from_mode(0o700))
            .tempdir_in(path)
            .map_err(|_| ClaudeLoginError::Unavailable)?;
        let opener = scratch.path().join("browser");
        fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o700)
            .open(&opener)
            .and_then(|mut file| file.write_all(OPENER))
            .map_err(|_| ClaudeLoginError::Unavailable)?;
        let capture = Directory::open(scratch.path()).map_err(|_| ClaudeLoginError::Unavailable)?;

        command
            .env_clear()
            .args(["auth", "login", "--claudeai"])
            .current_dir(path)
            .env("CLAUDE_CONFIG_DIR", path)
            .env("BROWSER", opener)
            .env("SHIJU_CLAUDE_LOGIN_CAPTURE", scratch.path().join(CAPTURE_FILE));
        for name in ["PATH", "HOME", "USER", "LOGNAME", "TMPDIR", "LANG", "LC_ALL", "LC_CTYPE"] {
            if let Some(value) = std::env::var_os(name) {
                command.env(name, value);
            }
        }
        let deadline = Instant::now().checked_add(timeout).ok_or(ClaudeLoginError::Unavailable)?;
        if timeout.is_zero() {
            return Err(ClaudeLoginError::TimedOut);
        }
        let child = tokio::process::Command::from(command)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .process_group(0)
            .kill_on_drop(true)
            .spawn()
            .map_err(|_| ClaudeLoginError::Unavailable)?;

        let (link, ready) = oneshot::channel();
        let (stop, cancelled) = oneshot::channel();
        let (finished, mut done) = oneshot::channel();
        tokio::spawn(async move {
            let mut process = LoginProcess(child);
            let result = watch(&mut process, capture, link, cancelled, deadline).await;
            let status = process.reap().await;
            let cleanup = scratch.close();
            drop(lease);
            let result = match (status, cleanup) {
                (Ok(status), Ok(())) => result.and(if status.success() {
                    Ok(())
                } else {
                    Err(ClaudeLoginError::Exited)
                }),
                _ => Err(ClaudeLoginError::Unavailable),
            };
            let _ = finished.send(result);
        });

        match ready.await {
            Ok(url) => Ok(Self { url, stop, done }),
            Err(_) => {
                let result = (&mut done).await.map_err(|_| ClaudeLoginError::Unavailable)?;
                Err(result.err().unwrap_or(ClaudeLoginError::LinkUnavailable))
            }
        }
    }

    pub fn authorization_url(&self) -> &str {
        &self.url
    }

    /// Wait until the CLI exits and its process and transient files have been cleaned up.
    pub async fn wait(self) -> Result<(), ClaudeLoginError> {
        // Keep the cancellation sender alive while waiting.
        let Self { stop: _stop, done, .. } = self;
        done.await.map_err(|_| ClaudeLoginError::Unavailable)?
    }

    /// Stop the CLI and wait for cleanup before reporting cancellation.
    pub async fn cancel(self) -> Result<(), ClaudeLoginError> {
        let _ = self.stop.send(());
        match self.done.await.map_err(|_| ClaudeLoginError::Unavailable)? {
            Err(ClaudeLoginError::Cancelled) => Ok(()),
            result => result,
        }
    }
}

struct LoginProcess(Child);

impl LoginProcess {
    fn exited(&self) -> io::Result<bool> {
        let pid = self.0.id().expect("login child has not been reaped");
        // Check without reaping so the process group ID remains reserved through cleanup.
        let mut info: libc::siginfo_t = unsafe { std::mem::zeroed() };
        let result = unsafe {
            libc::waitid(libc::P_PID, pid, &mut info, libc::WEXITED | libc::WNOHANG | libc::WNOWAIT)
        };
        if result == 0 {
            Ok(unsafe { info.si_pid() } != 0)
        } else {
            let error = io::Error::last_os_error();
            if error.kind() == io::ErrorKind::Interrupted { Ok(false) } else { Err(error) }
        }
    }

    fn kill_group(&self) {
        if let Some(pid) = self.0.id() {
            // SAFETY: the unreaped child owns the group created with process_group(0).
            unsafe { libc::kill(-(pid as i32), libc::SIGKILL) };
        }
    }

    async fn reap(&mut self) -> io::Result<std::process::ExitStatus> {
        self.kill_group();
        self.0.wait().await
    }
}

impl Drop for LoginProcess {
    fn drop(&mut self) {
        self.kill_group();
    }
}

async fn watch(
    process: &mut LoginProcess,
    capture: Directory,
    link: oneshot::Sender<String>,
    mut cancelled: oneshot::Receiver<()>,
    deadline: Instant,
) -> Result<(), ClaudeLoginError> {
    let stdout = process.0.stdout.take().ok_or(ClaudeLoginError::Unavailable)?;
    let stderr = process.0.stderr.take().ok_or(ClaudeLoginError::Unavailable)?;
    let mut stdout = Output::new(stdout);
    let mut stderr = Output::new(stderr);
    let mut tick = time::interval(CHECK_INTERVAL);
    let link_deadline = Instant::now() + LINK_WAIT;
    let mut link = Some(link);
    let mut printed = None;
    loop {
        tokio::select! {
            biased;
            _ = &mut cancelled => return Err(ClaudeLoginError::Cancelled),
            () = time::sleep_until(deadline) => return Err(ClaudeLoginError::TimedOut),
            _ = tick.tick() => {
                if process.exited().map_err(|_| ClaudeLoginError::Unavailable)? {
                    return if link.is_none() { Ok(()) } else { Err(ClaudeLoginError::Exited) };
                }
                if let Some(sender) = link.take() {
                    let captured = capture.read(CAPTURE_FILE).ok().flatten()
                        .and_then(|bytes| trusted_url(&bytes));
                    let now = Instant::now();
                    let ready = captured.or_else(|| {
                        let (url, printed_at) = printed.as_ref()?;
                        (now >= link_deadline || now - *printed_at >= BROWSER_GRACE)
                            .then(|| String::clone(url))
                    });
                    if let Some(url) = ready {
                        sender.send(url).map_err(|_| ClaudeLoginError::Cancelled)?;
                    } else if now >= link_deadline {
                        return Err(ClaudeLoginError::LinkUnavailable);
                    } else {
                        link = Some(sender);
                    }
                }
            }
            url = stdout.read(link.is_some() && printed.is_none()), if !stdout.closed => {
                if link.is_some() && let Some(url) = url {
                    printed.get_or_insert_with(|| (url, Instant::now()));
                }
            }
            url = stderr.read(link.is_some() && printed.is_none()), if !stderr.closed => {
                if link.is_some() && let Some(url) = url {
                    printed.get_or_insert_with(|| (url, Instant::now()));
                }
            }
        }
    }
}

/// Drain CLI output using a bounded buffer while searching for the first printed link.
struct Output<R> {
    reader: BufReader<R>,
    line: Vec<u8>,
    oversized: bool,
    closed: bool,
}

impl<R: AsyncRead + Unpin> Output<R> {
    fn new(reader: R) -> Self {
        Self { reader: BufReader::new(reader), line: Vec::new(), oversized: false, closed: false }
    }

    async fn read(&mut self, search: bool) -> Option<String> {
        let Ok(bytes) = self.reader.fill_buf().await else {
            self.closed = true;
            return None;
        };
        self.closed = bytes.is_empty();
        if !search {
            let len = bytes.len();
            self.line.clear();
            self.reader.consume(len);
            return None;
        }
        let end = bytes.iter().position(|&byte| byte == b'\n');
        let len = end.map_or(bytes.len(), |end| end + 1);
        if self.line.len() + len > OUTPUT_LINE_LIMIT {
            self.oversized = true;
        }
        if !self.oversized {
            self.line.extend_from_slice(&bytes[..len]);
        }
        self.reader.consume(len);
        if end.is_none() && !self.closed {
            return None;
        }
        let url = (!self.oversized).then(|| printed_url(&self.line)).flatten();
        self.line.clear();
        self.oversized = false;
        url
    }
}

fn printed_url(line: &[u8]) -> Option<String> {
    let line = String::from_utf8_lossy(line);
    let mut plain = String::with_capacity(line.len());
    let mut chars = line.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '\x1b' && chars.next_if_eq(&'[').is_some() {
            // Skip the ANSI CSI parameters and final byte.
            chars.by_ref().find(|ch| ('@'..='~').contains(ch));
        } else {
            plain.push(ch);
        }
    }
    plain.split_ascii_whitespace().find_map(|part| trusted_url(part.as_bytes()))
}

fn trusted_url(bytes: &[u8]) -> Option<String> {
    let text = std::str::from_utf8(bytes).ok()?.trim();
    if text.len() > OUTPUT_LINE_LIMIT
        || text.chars().any(|ch| ch.is_control() || ch.is_whitespace())
    {
        return None;
    }
    let url = Url::parse(text).ok()?;
    if !text.starts_with("https://")
        || !url.username().is_empty()
        || url.password().is_some()
        || url.port().is_some()
        || url.fragment().is_some()
        || !matches!(
            (url.host_str()?, url.path()),
            ("claude.com", "/cai/oauth/authorize")
                | ("claude.ai" | "platform.claude.com", "/oauth/authorize")
        )
    {
        return None;
    }

    let mut has_redirect = false;
    for (key, value) in url.query_pairs() {
        match key.as_ref() {
            "redirect_uri" if !has_redirect && trusted_redirect(&value) => has_redirect = true,
            "code" if value == "true" => {}
            "redirect_uri" | "code" | "access_token" | "refresh_token" | "id_token"
            | "code_verifier" | "client_secret" => {
                return None;
            }
            _ => {}
        }
    }
    Some(url.into())
}

fn trusted_redirect(text: &str) -> bool {
    if text.chars().any(|ch| ch.is_control() || ch.is_whitespace()) {
        return false;
    }
    let Ok(url) = Url::parse(text) else { return false };
    if !url.username().is_empty() || url.password().is_some() || url.fragment().is_some() {
        return false;
    }
    match (url.scheme(), url.host_str(), url.port()) {
        ("http", Some("localhost" | "127.0.0.1" | "[::1]"), Some(port)) => port != 0,
        ("https", Some("platform.claude.com" | "console.anthropic.com"), None) => {
            url.path() == "/oauth/code/callback" && url.query().is_none()
        }
        _ => false,
    }
}
