use super::*;
use crate::providers::live::commit::*;
use std::{
    fs,
    os::unix::fs::PermissionsExt,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
};
use vw_api_types::id::{ProjectId, TenantScope};
struct Fixture {
    home: tempfile::TempDir,
    _root: tempfile::TempDir,
    tx: ProviderTransactions,
    scope: ProjectScope,
}
impl Fixture {
    fn new() -> Self {
        let home = tempfile::tempdir().unwrap();
        let root = tempfile::tempdir().unwrap();
        fs::set_permissions(root.path(), fs::Permissions::from_mode(0o700)).unwrap();
        let tx =
            ProviderTransactions::new(root.path(), ExecutionHostId::new("host").unwrap()).unwrap();
        let scope =
            ProjectScope::new(TenantScope::new("p", "c").unwrap(), ProjectId::from("project"))
                .unwrap();
        let mut files = CodexFiles::open(home.path()).unwrap();
        tx.commit_project_checked(
            &scope,
            &mut files.guard,
            &AppState::default(),
            true,
            &[],
            PendingTarget::default(),
            || Ok(()),
        )
        .unwrap();
        drop(files);
        Self { home, _root: root, tx, scope }
    }
    fn revision(&self, files: &CodexFiles) -> String {
        catalog_generation::read(files, &self.tx, &self.scope).unwrap().revision()
    }
    fn daemon_record(&self, pid: u32, identity: &Identity) {
        let dir = self.home.path().join(".codex/app-server-daemon");
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("daemon.pid"), serde_json::json!({"pid":pid,"processStartTime":"legacy field retained by Codex","processIdentity":{"startSeconds":identity.start_seconds,"startMicroseconds":identity.start_microseconds}}).to_string()).unwrap();
    }
}
struct Child(std::process::Child);
impl Drop for Child {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}
fn real_codex() -> PathBuf {
    std::env::var_os("T29_CODEX_BINARY")
        .map(PathBuf::from)
        .expect("T29_CODEX_BINARY must point to the installed real Codex CLI")
        .canonicalize()
        .unwrap()
}
#[test]
fn aggregation_t29_daemon_restart_preserves_contract() {
    assert_eq!(lifecycle_result(true, b"garbage"), Err(RestartError::TargetFailed));
    assert_eq!(lifecycle_result(true, b"{}"), Err(RestartError::TargetFailed));
    assert_eq!(
        lifecycle_result(true, b"{\"status\":\"unknown\"}"),
        Err(RestartError::TargetFailed)
    );
    assert_eq!(
        lifecycle_result(false, b"{\"status\":\"restarted\"}"),
        Err(RestartError::TargetFailed)
    );
    assert_eq!(
        lifecycle_result(true, b"warning\n{\"status\":\"restarted\"}\n"),
        Ok(RestartOutcome::Restarted)
    );
    assert_eq!(
        lifecycle_result(true, b"{\"status\":\"notRunning\"}"),
        Ok(RestartOutcome::NotRunning)
    );
}
#[tokio::test]
async fn aggregation_t29_real_codex_identity_and_capabilities_gate_restart() {
    let f = Fixture::new();
    let executable = real_codex();
    let mut child = Child(
        std::process::Command::new(&executable)
            .args(["app-server", "--managed-daemon"])
            .env_clear()
            .env("HOME", f.home.path())
            .env("CODEX_HOME", f.home.path().join(".codex"))
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap(),
    );
    let identity = native_identity(child.0.id()).unwrap();
    f.daemon_record(child.0.id(), &identity);
    let files = CodexFiles::open(f.home.path()).unwrap();
    let revision = f.revision(&files);
    let calls = Arc::new(AtomicUsize::new(0));
    let attempt = |confirmed, expected, allowed| {
        let calls = calls.clone();
        restart_with(
            &files,
            &f.tx,
            &f.scope,
            RestartRequest { expected_revision: expected, confirmed, executable: &executable },
            || allowed,
            move |binary, home| {
                assert_eq!(binary, real_codex());
                assert_eq!(home.file_name().unwrap(), ".codex");
                calls.fetch_add(1, Ordering::SeqCst);
                Ok(tokio::spawn(async { Ok(RestartOutcome::Restarted) }))
            },
        )
    };
    assert!(matches!(attempt(false, &revision, true), Err(RestartError::ConfirmationRequired)));
    assert!(matches!(attempt(true, &revision, false), Err(RestartError::Forbidden)));
    assert!(matches!(attempt(true, "old", true), Err(RestartError::RevisionConflict)));
    // sysctl argv and proc_pidpath must see the real isolated managed client, not a shell fixture.
    let mut operation = attempt(true, &revision, true).unwrap();
    assert_eq!(operation.wait().await, Ok(RestartOutcome::Restarted));
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    assert!(
        child.0.try_wait().unwrap().is_none(),
        "dangerous restart boundary must be substituted"
    );
    let checks = AtomicUsize::new(0);
    let denied = restart_with(
        &files,
        &f.tx,
        &f.scope,
        RestartRequest { expected_revision: &revision, confirmed: true, executable: &executable },
        || checks.fetch_add(1, Ordering::SeqCst) == 0,
        |_, _| panic!("revoked authorization must not execute"),
    );
    assert!(matches!(denied, Err(RestartError::Forbidden)));
    // Same PID with the wrong native start generation must never execute.
    f.daemon_record(
        child.0.id(),
        &Identity { start_seconds: identity.start_seconds + 1, ..identity },
    );
    assert!(matches!(attempt(true, &revision, true), Err(RestartError::TargetUnavailable)));
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    f.daemon_record(child.0.id(), &native_identity(child.0.id()).unwrap());
    let checks = AtomicUsize::new(0);
    let changed = restart_with(
        &files,
        &f.tx,
        &f.scope,
        RestartRequest { expected_revision: &revision, confirmed: true, executable: &executable },
        || {
            if checks.fetch_add(1, Ordering::SeqCst) == 1 {
                fs::write(f.home.path().join(".codex/shiju-model-catalog.json"), "changed-catalog")
                    .unwrap();
                fs::write(
                    f.home.path().join(".codex/config.toml"),
                    "model_catalog_json = 'shiju-model-catalog.json'\n",
                )
                .unwrap();
            }
            true
        },
        |_, _| panic!("changed revision must not execute"),
    );
    assert!(matches!(changed, Err(RestartError::RevisionConflict)));
    println!(
        "real isolated managed Codex identified by PID/start/executable/argv; restart boundary substituted; owned child reaped"
    );
}
#[tokio::test]
async fn aggregation_t29_missing_unsafe_record_and_foreign_clients_never_start() {
    let f = Fixture::new();
    let executable = real_codex();
    let files = CodexFiles::open(f.home.path()).unwrap();
    let revision = f.revision(&files);
    let attempt = || {
        restart_with(
            &files,
            &f.tx,
            &f.scope,
            RestartRequest {
                expected_revision: &revision,
                confirmed: true,
                executable: &executable,
            },
            || true,
            |_, _| panic!("must not execute"),
        )
    };
    assert_eq!(attempt().unwrap().wait().await, Ok(RestartOutcome::NotRunning));
    f.daemon_record(std::process::id(), &native_identity(std::process::id()).unwrap());
    assert!(matches!(attempt(), Err(RestartError::TargetUnavailable)));
    let daemon_dir = f.home.path().join(".codex/app-server-daemon");
    fs::set_permissions(&daemon_dir, fs::Permissions::from_mode(0o777)).unwrap();
    assert!(matches!(attempt(), Err(RestartError::TargetUnavailable)));
    fs::set_permissions(&daemon_dir, fs::Permissions::from_mode(0o700)).unwrap();
    let record = f.home.path().join(".codex/app-server-daemon/daemon.pid");
    fs::write(&record, "{broken").unwrap();
    assert!(matches!(attempt(), Err(RestartError::UnsupportedTarget)));
    fs::remove_file(&record).unwrap();
    let outside = tempfile::NamedTempFile::new().unwrap();
    fs::write(outside.path(), "untouched").unwrap();
    std::os::unix::fs::symlink(outside.path(), &record).unwrap();
    assert!(matches!(attempt(), Err(RestartError::TargetUnavailable)));
    assert_eq!(fs::read(outside.path()).unwrap(), b"untouched");
    assert!(matches!(trusted_executable(Path::new("codex")), Err(RestartError::UnsupportedTarget)));
}
#[tokio::test]
async fn aggregation_t29_fixed_command_failure_timeout_and_cancelled_wait() {
    let f = Fixture::new();
    let home = f.home.path().join(".codex");
    let script = f.home.path().join("codex");
    fs::write(&script,"#!/bin/sh\nprintf '%s\\n' \"$*\" > \"$CODEX_HOME/argv\"\nprintf '%s\\n' \"$HOME\" > \"$CODEX_HOME/home\"\nprintf '{\"status\":\"restarted\"}\\n'\n").unwrap();
    fs::set_permissions(&script, fs::Permissions::from_mode(0o700)).unwrap();
    assert_eq!(
        run_command(script.clone(), home.clone(), Duration::from_secs(2)).unwrap().await.unwrap(),
        Ok(RestartOutcome::Restarted)
    );
    assert_eq!(fs::read_to_string(home.join("argv")).unwrap(), "app-server daemon restart\n");
    assert_eq!(
        fs::read_to_string(home.join("home")).unwrap().trim(),
        f.home.path().to_str().unwrap()
    );
    fs::write(&script, "#!/bin/sh\nexit 7\n").unwrap();
    assert_eq!(
        run_command(script.clone(), home.clone(), Duration::from_secs(2)).unwrap().await.unwrap(),
        Err(RestartError::TargetFailed)
    );
    fs::write(
        &script,
        "#!/bin/sh\nprintf '%s' \"$$\" > \"$CODEX_HOME/command.pid\"\nexec /bin/sleep 5\n",
    )
    .unwrap();
    assert_eq!(
        run_command(script.clone(), home.clone(), Duration::from_millis(500))
            .unwrap()
            .await
            .unwrap(),
        Err(RestartError::TargetTimeout)
    );
    let pid: i32 = fs::read_to_string(home.join("command.pid")).unwrap().parse().unwrap();
    assert_eq!(unsafe { libc::kill(pid, 0) }, -1);
    assert_eq!(std::io::Error::last_os_error().raw_os_error(), Some(libc::ESRCH));
    fs::write(&script,"#!/bin/sh\n/bin/sleep 0.08\nprintf complete > \"$CODEX_HOME/completed\"\nprintf '{\"status\":\"restarted\"}\\n'\n").unwrap();
    let mut operation = RestartOperation {
        task: run_command(script, home.clone(), Duration::from_secs(2)).unwrap(),
    };
    assert!(tokio::time::timeout(Duration::from_millis(5), operation.wait()).await.is_err());
    assert_eq!(operation.wait().await, Ok(RestartOutcome::Restarted));
    assert_eq!(fs::read(home.join("completed")).unwrap(), b"complete");
}

#[tokio::test]
async fn aggregation_t29_foreign_real_codex_home_is_rejected() {
    let f = Fixture::new();
    let other = tempfile::tempdir().unwrap();
    let executable = real_codex();
    let child = Child(
        std::process::Command::new(&executable)
            .args(["app-server", "--managed-daemon"])
            .env_clear()
            .env("HOME", other.path())
            .env("CODEX_HOME", other.path())
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap(),
    );
    f.daemon_record(child.0.id(), &native_identity(child.0.id()).unwrap());
    let files = CodexFiles::open(f.home.path()).unwrap();
    let revision = f.revision(&files);
    let result = restart_with(
        &files,
        &f.tx,
        &f.scope,
        RestartRequest { expected_revision: &revision, confirmed: true, executable: &executable },
        || true,
        |_, _| panic!("foreign CODEX_HOME must not restart or launch"),
    );
    assert!(matches!(result, Err(RestartError::TargetUnavailable)));
}

#[tokio::test]
async fn aggregation_t29_real_unmanaged_codex_is_never_restarted() {
    let f = Fixture::new();
    let executable = real_codex();
    let child = Child(
        std::process::Command::new(&executable)
            .arg("app-server")
            .env_clear()
            .env("HOME", f.home.path())
            .env("CODEX_HOME", f.home.path().join(".codex"))
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap(),
    );
    f.daemon_record(child.0.id(), &native_identity(child.0.id()).unwrap());
    let files = CodexFiles::open(f.home.path()).unwrap();
    let revision = f.revision(&files);
    let result = restart_with(
        &files,
        &f.tx,
        &f.scope,
        RestartRequest { expected_revision: &revision, confirmed: true, executable: &executable },
        || true,
        |_, _| panic!("unmanaged client must not restart"),
    );
    assert!(matches!(result, Err(RestartError::TargetUnavailable)));
}
