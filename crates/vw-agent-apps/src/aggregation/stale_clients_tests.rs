use super::{
    catalog_generation::{self, Snapshot},
    stale_clients::*,
};
use crate::providers::{
    codex_catalog::{CATALOG_FILENAME, CodexFiles},
    live::commit::*,
};
use std::{fs, os::unix::fs::PermissionsExt, path::Path};
use vw_api_types::id::{ProjectId, ProjectScope, TenantScope};
fn snapshot(change: bool, external: bool) -> Snapshot {
    let home = tempfile::tempdir().unwrap();
    let root = tempfile::tempdir().unwrap();
    fs::set_permissions(root.path(), fs::Permissions::from_mode(0o700)).unwrap();
    let tx = ProviderTransactions::new(root.path(), ExecutionHostId::new("host").unwrap()).unwrap();
    let scope =
        ProjectScope::new(TenantScope::new("p", "c").unwrap(), ProjectId::from("project")).unwrap();
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
    let dir = home.path().join(".codex");
    fs::write(dir.join("config.toml"), format!("model_catalog_json = '{CATALOG_FILENAME}'"))
        .unwrap();
    fs::write(dir.join(CATALOG_FILENAME), "a").unwrap();
    fs::write(dir.join("auth.json"), r#"{"tokens":{"account_id":"a"}}"#).unwrap();
    catalog_generation::observe(&mut files, &tx, &scope, 10_000).unwrap();
    if change {
        fs::write(dir.join(CATALOG_FILENAME), "b").unwrap();
        fs::write(dir.join("auth.json"), r#"{"tokens":{"account_id":"b"}}"#).unwrap();
        if !external {
            catalog_generation::observe(&mut files, &tx, &scope, 20_000).unwrap();
        }
    }
    catalog_generation::read(&files, &tx, &scope).unwrap()
}
fn inspect_table(s: &Snapshot, catalog: bool, table: Option<&str>, now: u64) -> ClientsState {
    inspect(
        s,
        catalog,
        ProcessIdentity { daemon_pid: Some(42), plugins_dir: Path::new("/home/.codex/plugins") },
        table,
        now,
    )
}
#[test]
fn aggregation_t27_stale_clients_preserves_contract() {
    let s = snapshot(true, false);
    let state = inspect_table(
        &s,
        true,
        Some(
            "42 00:18 /opt/codex app-server --managed-daemon\n43 00:18 /Applications/Codex.app/Contents/Resources/codex app-server",
        ),
        30_000,
    );
    assert!(state.daemon && state.others && state.auth);
    assert!(!state.unverified);
    assert_eq!(state.revision, s.revision());
    let state = inspect_table(&s, false, Some("43 00:18 codex app-server"), 30_000);
    assert!(state.others && state.auth, "login checking is independent of catalog mode");
}
#[test]
fn aggregation_t27_two_second_boundary() {
    let s = snapshot(true, false);
    assert!(
        inspect_table(&s, true, Some("42 00:08 codex app-server --managed-daemon"), 29_999).daemon
    );
    assert!(
        !inspect_table(&s, true, Some("42 00:08 codex app-server --managed-daemon"), 30_000).daemon
    );
}
#[test]
fn aggregation_t27_unreadable_or_external_is_unverified() {
    let s = snapshot(false, false);
    assert!(inspect_table(&s, true, None, 30_000).unverified);
    assert!(!inspect_table(&s, true, Some(""), 30_000).unverified);
    assert!(inspect_table(&snapshot(true, true), true, Some(""), 30_000).unverified);
}
#[test]
fn aggregation_t27_ignores_management_plugins_and_foreign_daemon() {
    let s = snapshot(true, false);
    let state = inspect_table(
        &s,
        true,
        Some(
            "99 00:18 codex app-server --managed-daemon\n45 00:18 codex app-server daemon restart\n46 00:18 codex app-server proxy\n47 00:18 /home/.codex/plugins/browser/codex app-server\n48 00:18 /usr/bin/echo codex app-server\n49 00:18 codex-other app-server",
        ),
        30_000,
    );
    assert!(!state.daemon && !state.others && !state.auth);
}

#[test]
fn aggregation_t27_invalid_elapsed_and_overflow_are_unverified() {
    let s = snapshot(true, false);
    for elapsed in ["bad", "00:99", "18446744073709551615-01:01:01", "999999:00:00"] {
        let row = format!("42 {elapsed} codex app-server --managed-daemon");
        let state = inspect_table(&s, true, Some(&row), 30_000);
        assert!(state.unverified, "invalid process time must remain unknown");
        assert!(!state.daemon && !state.auth);
    }
    let state = inspect_table(
        &s,
        true,
        Some("43 00:18 /Applications/Codex App.app/Contents/MacOS/codex -c key=value app-server"),
        30_000,
    );
    assert!(state.others);
    let state = inspect_table(
        &s,
        true,
        Some("43 00:18 /home/.codex/plugins-other/codex app-server"),
        30_000,
    );
    assert!(state.others, "plugin exclusion uses path components");
    let state = inspect_table(&s, true, Some("43 00:18 /bin/echo /opt/codex app-server"), 30_000);
    assert!(!state.others);
}

#[tokio::test]
async fn aggregation_t27_native_ps_detects_real_isolated_codex() {
    use std::process::{Command, Stdio};
    struct Client(std::process::Child);
    impl Drop for Client {
        fn drop(&mut self) {
            let _ = self.0.kill();
            let _ = self.0.wait();
        }
    }
    // Explicit test-only installation input: missing real CLI is a failure, never a skip.
    let executable = std::env::var_os("T27_CODEX_BINARY")
        .expect("set T27_CODEX_BINARY to the installed real Codex CLI");
    let home = tempfile::tempdir().unwrap();
    let mut client = Client(
        Command::new(executable)
            .arg("app-server")
            .env("CODEX_HOME", home.path())
            .env("HOME", home.path())
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap(),
    );
    let pid = client.0.id();
    let mut row = None;
    for _ in 0..30 {
        assert!(
            client.0.try_wait().unwrap().is_none(),
            "real app-server exited before observation"
        );
        let table =
            super::stale_clients::process_table().await.expect("native /bin/ps is required");
        row = table
            .lines()
            .find(|line| {
                line.split_whitespace().next().and_then(|s| s.parse::<u32>().ok()) == Some(pid)
            })
            .map(str::to_owned);
        if row.is_some() {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }
    let row = row.expect("real Codex process must appear in native ps");
    let s = snapshot(true, false);
    // Only the clock is replaced: this live process starts within the changed generation's 2s margin.
    let state = inspect_table(&s, true, Some(&row), 20_000);
    assert!(state.others && state.auth && !state.daemon && !state.unverified);
    let state = read(
        &s,
        true,
        ProcessIdentity { daemon_pid: None, plugins_dir: &home.path().join("plugins") },
    )
    .await;
    assert_eq!(state.revision, s.revision());
    assert!(!state.unverified, "bounded native probe must be verified");
    println!("real Codex app-server detected through native ps; isolated home; owned child reaped");
}

#[test]
fn aggregation_t27_first_login_and_bounded_table() {
    let s = snapshot(false, false);
    let state = inspect_table(&s, false, Some("43 00:30 codex app-server"), 30_000);
    assert!(!state.auth && !state.others && !state.unverified, "first login is not a switch");
    let oversized = " ".repeat(4 * 1024 * 1024 + 1);
    assert!(inspect_table(&s, true, Some(&oversized), 30_000).unverified);
    let s = snapshot(true, false);
    let state = inspect_table(
        &s,
        true,
        Some("42 1-00:00:00 codex app-server --managed-daemon"),
        86_412_000,
    );
    assert!(state.daemon && state.auth && !state.unverified);
}
