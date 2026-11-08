use super::{
    claude_login::{ClaudeLogin, ClaudeLoginError},
    credential_file::{AccountCredential, CredentialFileStore},
    credential_lease::CredentialLease,
    flow::AuthAccountScope,
};
use std::{
    fs,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    process::Command,
    time::Duration,
};
use vw_api_types::{agent_apps::AppId, agent_apps_auth::AuthService, id::TenantScope};

const TIMEOUT: Duration = Duration::from_secs(8);
const TRUSTED_URL: &str = "https://claude.com/cai/oauth/authorize?code=true&state=fixture";
const PRINTED_URL: &str = "https://platform.claude.com/oauth/authorize?state=printed";

struct Fixture {
    root: tempfile::TempDir,
    store: CredentialFileStore,
    tenant: TenantScope,
    account: PathBuf,
    other: PathBuf,
}

impl Fixture {
    fn new() -> Self {
        let root = tempfile::Builder::new()
            .permissions(fs::Permissions::from_mode(0o700))
            .tempdir()
            .unwrap();
        let store = CredentialFileStore::new(root.path()).unwrap();
        let tenant = TenantScope::new("platform", "company").unwrap();
        let register = |id| {
            let store = store.for_owner("owner").unwrap();
            store.register_claude_directory(&tenant, "host", id).unwrap();
            let AccountCredential::ClaudeDirectory(directory) =
                store.read(&tenant, "host", AppId::Claude, id).unwrap()
            else {
                panic!("expected managed directory");
            };
            directory.path().to_owned()
        };
        let account = register("selected");
        let other = register("other");
        Self { root, store, tenant, account, other }
    }

    fn scope(&self) -> AuthAccountScope<'_> {
        AuthAccountScope {
            tenant: &self.tenant,
            principal: "owner",
            host: "host",
            service: AuthService::ClaudeOfficial,
        }
    }

    async fn start(
        &self,
        program: &Path,
        timeout: Duration,
    ) -> Result<ClaudeLogin, ClaudeLoginError> {
        ClaudeLogin::start(program, &self.store, self.scope(), "selected", timeout).await
    }

    fn lease(&self) -> Result<CredentialLease, String> {
        self.store.for_owner("owner")?.lease(&self.tenant, "host", AppId::Claude, "selected", false)
    }

    fn program(&self, body: &str) -> PathBuf {
        let path = self.root.path().join("claude");
        fs::write(&path, format!("#!/bin/sh\nset -eu\n{body}\n")).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o700)).unwrap();
        path
    }
}

fn read(path: &Path, name: &str) -> String {
    fs::read_to_string(path.join(name)).unwrap()
}

#[tokio::test]
async fn auth6_claude_login_commits_expected_scoped_result() {
    let fixture = Fixture::new();
    let program = fixture.program(&format!(
        r#"
test "$*" = 'auth login --claudeai'
test "$(/bin/pwd -P)" = "$(cd "$CLAUDE_CONFIG_DIR" && /bin/pwd -P)"
test -z "${{ANTHROPIC_API_KEY:-}}${{CLAUDE_CODE_OAUTH_TOKEN:-}}${{CLAUDE_CODE_USE_BEDROCK:-}}${{CLAUDE_SECURESTORAGE_CONFIG_DIR:-}}"
printf '%s' "$CLAUDE_CONFIG_DIR" > scope
printf '%s\n' '{PRINTED_URL}'
"$BROWSER" '{TRUSTED_URL}'
while [ ! -f finish ]; do /bin/sleep 0.05; done
printf '%s' complete > profile
"#
    ));
    let mut command = Command::new(program);
    command
        .env("CLAUDE_CONFIG_DIR", &fixture.other)
        .env("CLAUDE_SECURESTORAGE_CONFIG_DIR", &fixture.other)
        .env("CLAUDE_CODE_OAUTH_TOKEN", "other-account")
        .env("ANTHROPIC_API_KEY", "other-key")
        .env("CLAUDE_CODE_USE_BEDROCK", "1")
        .env("BROWSER", "/bin/false");
    let login = ClaudeLogin::start_with_command(
        command,
        &fixture.store,
        fixture.scope(),
        "selected",
        TIMEOUT,
    )
    .await
    .expect("trusted browser URL from the scoped CLI");
    assert_eq!(login.authorization_url(), TRUSTED_URL);
    assert_eq!(read(&fixture.account, "scope"), fixture.account.to_str().unwrap());
    assert!(fixture.lease().is_err());
    assert!(!fixture.other.join("scope").exists());
    fs::write(fixture.account.join("finish"), "").unwrap();
    assert_eq!(login.wait().await, Ok(()));
    assert_eq!(read(&fixture.account, "profile"), "complete");
    assert!(fixture.lease().is_ok());
}

#[tokio::test]
async fn auth6_claude_login_rejects_invalid_or_stale_input() {
    let fixture = Fixture::new();
    let program = fixture.program(
        r#"
"$BROWSER" 'https://claude.com.evil.example/cai/oauth/authorize'
printf '%s\n' 'https://claude.com@evil.example/cai/oauth/authorize' >&2
/bin/sleep 0.3
printf '%s\n' 'private-fixture-output' >&2
exit 17
"#,
    );
    let other = AuthAccountScope { principal: "another-owner", ..fixture.scope() };
    let result = ClaudeLogin::start(&program, &fixture.store, other, "selected", TIMEOUT).await;
    assert_eq!(result.err(), Some(ClaudeLoginError::InvalidAccount));
    let result = fixture.start(&program, TIMEOUT).await;
    assert_eq!(result.err(), Some(ClaudeLoginError::Exited));
    assert!(fixture.lease().is_ok());
}

#[tokio::test]
async fn auth6_claude_login_uses_only_trusted_printed_fallback() {
    let fixture = Fixture::new();
    let program = fixture.program(&format!(
        r#"
"$BROWSER" 'https://claude.com/cai/oauth/authorize?redirect_uri=https%3A%2F%2Fevil.example%2Fcallback'
printf '%s\n' 'https://docs.claude.com/oauth/authorize' >&2
printf '%s\n' 'https://platform.claude.com/oauth/authorize?code=private-fixture-code' >&2
printf '\033[1m%s\033[0m\n' '{PRINTED_URL}' >&2
while [ ! -f finish ]; do /bin/sleep 0.05; done
"#
    ));
    let login = fixture.start(&program, TIMEOUT).await.expect("trusted printed fallback");
    assert_eq!(login.authorization_url(), PRINTED_URL);
    fs::write(fixture.account.join("finish"), "").unwrap();
    assert_eq!(login.wait().await, Ok(()));
}

async fn assert_processes_stopped(path: &Path) {
    let leader = read(path, "leader");
    let helper = read(path, "helper");
    tokio::time::timeout(Duration::from_secs(3), async {
        loop {
            let status = |pid: &str| {
                let output =
                    Command::new("/bin/ps").args(["-o", "state=", "-p", pid]).output().unwrap();
                assert!(output.status.code().is_some_and(|code| code == 0 || code == 1));
                String::from_utf8(output.stdout).unwrap().trim().to_owned()
            };
            let helper_state = status(&helper);
            if status(&leader).is_empty()
                && (helper_state.is_empty() || helper_state.starts_with('Z'))
            {
                break;
            }
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
    })
    .await
    .expect("CLI was reaped and its helper stopped");
    assert!(
        fs::read_dir(path).unwrap().all(|entry| !entry
            .unwrap()
            .file_name()
            .to_string_lossy()
            .starts_with(".shiju-login-"))
    );
}

#[tokio::test]
async fn auth6_claude_login_reaps_process_group_on_cancel_or_exit_failure() {
    for cancel in [true, false] {
        let fixture = Fixture::new();
        let program = fixture.program(&format!(
            r#"
printf '%s' "$$" > leader
/bin/sleep 30 &
printf '%s' "$!" > helper
"$BROWSER" '{TRUSTED_URL}'
while [ ! -f finish ]; do /bin/sleep 0.05; done
printf '%s\n' 'private-fixture-output' >&2
exit 17
"#
        ));
        let login = fixture.start(&program, TIMEOUT).await.unwrap();
        assert_eq!(login.authorization_url(), TRUSTED_URL);
        if cancel {
            assert_eq!(login.cancel().await, Ok(()));
        } else {
            fs::write(fixture.account.join("finish"), "").unwrap();
            assert_eq!(login.wait().await, Err(ClaudeLoginError::Exited));
        }
        assert_processes_stopped(&fixture.account).await;
        assert!(fixture.lease().is_ok());
    }
}

#[tokio::test]
async fn auth6_claude_login_reaps_process_group_when_link_times_out() {
    let fixture = Fixture::new();
    let program = fixture.program(
        r#"
printf '%s' "$$" > leader
/bin/sleep 30 &
printf '%s' "$!" > helper
wait
"#,
    );
    let result = fixture.start(&program, Duration::from_secs(1)).await;
    assert_eq!(result.err(), Some(ClaudeLoginError::TimedOut));
    assert_processes_stopped(&fixture.account).await;
    assert!(fixture.lease().is_ok());
}

#[tokio::test]
async fn auth6_claude_login_reaps_process_group_when_handle_is_dropped() {
    let fixture = Fixture::new();
    let program = fixture.program(&format!(
        r#"
printf '%s' "$$" > leader
/bin/sleep 30 &
printf '%s' "$!" > helper
"$BROWSER" '{TRUSTED_URL}'
wait
"#
    ));
    let login = fixture.start(&program, TIMEOUT).await.unwrap();
    drop(login);
    assert_processes_stopped(&fixture.account).await;
    assert!(fixture.lease().is_ok());
}

async fn callback(listener: &tokio::net::TcpListener) -> (String, tokio::net::TcpStream) {
    use tokio::io::AsyncBufReadExt;

    tokio::time::timeout(Duration::from_secs(3), async {
        let (socket, _) = listener.accept().await.unwrap();
        let mut reader = tokio::io::BufReader::new(socket);
        let mut request = String::new();
        reader.read_line(&mut request).await.unwrap();
        loop {
            let mut header = String::new();
            assert_ne!(reader.read_line(&mut header).await.unwrap(), 0);
            if header == "\r\n" {
                break;
            }
        }
        (request, reader.into_inner())
    })
    .await
    .expect("callback received on the expected listener")
}

fn callback_url(port: u16, suffix: &str) -> String {
    format!("http://127.0.0.1:{port}/callback?{suffix}")
}

fn loopback_program(fixture: &Fixture, port: u16) -> PathBuf {
    fixture.program(&format!(
        r#"
"$BROWSER" 'https://claude.com/cai/oauth/authorize?redirect_uri=http%3A%2F%2Flocalhost%3A{port}%2Fcallback&state=fixture'
while [ ! -f finish ]; do /bin/sleep 0.05; done
"#
    ))
}

#[tokio::test]
async fn auth6_claude_paste_commits_expected_scoped_result() {
    use tokio::io::AsyncWriteExt;

    let fixture = Fixture::new();
    let program = fixture.program(&format!(
        r#"
"$BROWSER" '{TRUSTED_URL}'
IFS= read -r line
test "$line" = 'fixture-code#fixture'
printf '%s' accepted > received
"#
    ));
    let login = fixture.start(&program, TIMEOUT).await.unwrap();
    assert_eq!(login.submit_paste(" fixture-code#fixture\n").await, Ok(()));
    assert_eq!(login.wait().await, Ok(()));
    assert_eq!(read(&fixture.account, "received"), "accepted");
    assert!(!fixture.other.join("received").exists());

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let redirect = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let redirect_port = redirect.local_addr().unwrap().port();
    let program = loopback_program(&fixture, port);
    let login = fixture.start(&program, TIMEOUT).await.unwrap();
    let target = callback_url(port, "code=fixture-code&state=fixture");
    let respond = async {
        let (request, mut socket) = callback(&listener).await;
        assert!(request == "GET /callback?code=fixture-code&state=fixture HTTP/1.1\r\n");
        socket
            .write_all(
                format!(
                    "HTTP/1.1 302 Found\r\nLocation: http://127.0.0.1:{redirect_port}/\r\nContent-Length: 0\r\n\r\n"
                )
                .as_bytes(),
            )
            .await
            .unwrap();
    };
    let (result, ()) = tokio::join!(login.submit_paste(&target), respond);
    assert_eq!(result, Ok(()));
    assert!(tokio::time::timeout(Duration::from_millis(100), redirect.accept()).await.is_err());
    fs::write(fixture.account.join("finish"), "").unwrap();
    assert_eq!(login.wait().await, Ok(()));
}

#[tokio::test]
async fn auth6_claude_paste_rejects_invalid_or_stale_input() {
    use tokio::io::AsyncReadExt;

    let fixture = Fixture::new();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let unrelated = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let other_port = unrelated.local_addr().unwrap().port();
    let program = loopback_program(&fixture, port);
    let login = fixture.start(&program, TIMEOUT).await.unwrap();
    for invalid in [
        "plain".to_owned(),
        "fixture-code#other-state".to_owned(),
        "first\nsecond#fixture".to_owned(),
        "bad\u{1b}code#fixture".to_owned(),
        format!("{}fixture-code#fixture", " ".repeat(16 * 1024)),
        "https://example.com/callback?code=fixture-code&state=fixture".to_owned(),
        callback_url(other_port, "code=fixture-code&state=fixture"),
        callback_url(port, "code=fixture-code&state=other-state"),
        callback_url(port, "code=fixture-code&code=duplicate&state=fixture"),
        format!("http://127.0.0.1:{port}/other?code=fixture-code&state=fixture"),
    ] {
        assert_eq!(login.submit_paste(&invalid).await, Err(ClaudeLoginError::InvalidPaste));
    }
    assert!(tokio::time::timeout(Duration::from_millis(100), listener.accept()).await.is_err());
    assert!(tokio::time::timeout(Duration::from_millis(100), unrelated.accept()).await.is_err());

    let target = callback_url(port, "code=fixture-code&state=fixture");
    let mut socket = {
        let submit = login.submit_paste(&target);
        tokio::pin!(submit);
        tokio::select! {
            (_, socket) = callback(&listener) => socket,
            _ = &mut submit => panic!("callback completed before it received a response"),
        }
    };
    assert_eq!(login.cancel().await, Ok(()));
    let mut buffer = [0; 1];
    let closed = tokio::time::timeout(Duration::from_secs(2), socket.read(&mut buffer))
        .await
        .expect("cancellation closes the in-flight callback");
    assert_eq!(closed.unwrap(), 0);
    assert!(fixture.lease().is_ok());

    let login = fixture.start(&program, Duration::from_secs(1)).await.unwrap();
    tokio::time::sleep(Duration::from_secs(1)).await;
    assert_eq!(login.submit_paste(&target).await, Err(ClaudeLoginError::Inactive));
    assert_eq!(login.wait().await, Err(ClaudeLoginError::TimedOut));
    assert!(tokio::time::timeout(Duration::from_millis(100), listener.accept()).await.is_err());
    assert!(fixture.lease().is_ok());
}
