//! Actual installed Codex manager, isolated HOME, native records and production restart.
use super::*;
use crate::{
    aggregation::{catalog_ack, stale_clients},
    providers::live::commit::*,
};
use std::{
    fs,
    os::unix::fs::{PermissionsExt, symlink},
    time::{SystemTime, UNIX_EPOCH},
};
use vw_api_types::id::{ProjectId, TenantScope};
struct IsolatedDaemon {
    executable: PathBuf,
    home: tempfile::TempDir,
}
impl IsolatedDaemon {
    fn command(&self, action: &str) -> std::process::Output {
        std::process::Command::new(&self.executable)
            .args(["app-server", "daemon", action])
            .env_clear()
            .env("HOME", self.home.path())
            .env("CODEX_HOME", self.home.path().join(".codex"))
            .env("PATH", "/usr/bin:/bin")
            .env("LC_ALL", "C")
            .current_dir(self.home.path())
            .output()
            .unwrap()
    }
    fn pid(&self) -> u32 {
        let value: serde_json::Value = serde_json::from_slice(
            &fs::read(self.home.path().join(".codex/app-server-daemon/daemon.pid")).unwrap(),
        )
        .unwrap();
        assert!(
            value["processIdentity"]["startMicroseconds"].is_number(),
            "manager must write native identity"
        );
        value["pid"].as_u64().unwrap().try_into().unwrap()
    }
}
impl Drop for IsolatedDaemon {
    fn drop(&mut self) {
        assert!(self.command("stop").status.success(), "isolated manager cleanup must succeed");
    }
}
fn now() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis().try_into().unwrap()
}
#[tokio::test]
async fn aggregation_t62_real_manager_stale_ack_restart_and_not_running() {
    let executable =
        PathBuf::from(std::env::var_os("T62_CODEX_BINARY").expect("installed real Codex required"))
            .canonicalize()
            .unwrap();
    let home = tempfile::tempdir().unwrap();
    let daemon = IsolatedDaemon { executable, home };
    let package = daemon.home.path().join(".codex/packages/app-server-daemon/current/bin");
    fs::create_dir_all(&package).unwrap();
    // Use the installed Mach-O; no replacement binary, PID record, argv or process table.
    symlink(&daemon.executable, package.join("codex")).unwrap();
    let root = tempfile::tempdir().unwrap();
    fs::set_permissions(root.path(), fs::Permissions::from_mode(0o700)).unwrap();
    let tx =
        ProviderTransactions::new(root.path(), ExecutionHostId::new("host-t62").unwrap()).unwrap();
    let scope = ProjectScope::new(
        TenantScope::new("fixture", "t62").unwrap(),
        ProjectId::from("project-t62"),
    )
    .unwrap();
    let mut files = CodexFiles::open(daemon.home.path()).unwrap();
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
    catalog_generation::observe(&mut files, &tx, &scope, now()).unwrap();
    let start = daemon.command("start");
    assert!(start.status.success(), "real manager must start");
    let before = daemon.pid();
    tokio::time::sleep(Duration::from_secs(3)).await;
    fs::write(daemon.home.path().join(".codex/shiju-model-catalog.json"), r#"{"models":[]}"#)
        .unwrap();
    fs::write(
        daemon.home.path().join(".codex/config.toml"),
        "model_catalog_json = 'shiju-model-catalog.json'\n",
    )
    .unwrap();
    let snapshot = catalog_generation::observe(&mut files, &tx, &scope, now()).unwrap();
    let plugins = daemon.home.path().join(".codex/plugins");
    let stale = stale_clients::read(
        &snapshot,
        true,
        stale_clients::ProcessIdentity { daemon_pid: Some(before), plugins_dir: &plugins },
    )
    .await;
    assert!(stale.daemon, "real process must be stale after catalog change");
    assert!(!stale.unverified);
    catalog_ack::acknowledge(&mut files, &tx, &scope, &stale.revision).unwrap();
    let after_ack = stale_clients::read(
        &snapshot,
        true,
        stale_clients::ProcessIdentity { daemon_pid: Some(before), plugins_dir: &plugins },
    )
    .await;
    assert!(after_ack.daemon, "ACK must not refresh or stop real daemon");
    assert_eq!(daemon.pid(), before);
    let request = || RestartRequest {
        expected_revision: &stale.revision,
        confirmed: true,
        executable: &daemon.executable,
    };
    assert!(matches!(
        restart_daemon(&files, &tx, &scope, request(), || false),
        Err(RestartError::Forbidden)
    ));
    // Preserve the production two-second ambiguity margin; do not fake the clock.
    tokio::time::sleep(Duration::from_secs(3)).await;
    let mut operation = restart_daemon(&files, &tx, &scope, request(), || true)
        .expect("native manager record must satisfy production identity");
    assert_eq!(operation.wait().await, Ok(RestartOutcome::Restarted));
    let after = daemon.pid();
    assert_ne!(before, after, "actual restart must replace daemon process");
    tokio::time::sleep(Duration::from_secs(3)).await;
    let refreshed = stale_clients::read(
        &snapshot,
        true,
        stale_clients::ProcessIdentity { daemon_pid: Some(after), plugins_dir: &plugins },
    )
    .await;
    assert!(!refreshed.daemon, "replacement started with current catalog");
    assert!(!refreshed.unverified);
    assert!(daemon.command("stop").status.success());
    let mut stopped = restart_daemon(&files, &tx, &scope, request(), || true).unwrap();
    assert_eq!(stopped.wait().await, Ok(RestartOutcome::NotRunning));
    assert!(!daemon.home.path().join(".codex/app-server-daemon/daemon.pid").exists());
    println!(
        "installed real Codex: manager-native record, stale -> ACK still stale -> production restart -> replacement current -> stop -> NotRunning; isolated cleanup complete"
    );
}
