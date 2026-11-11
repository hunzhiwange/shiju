use super::{catalog_ack::*, catalog_generation::*, stale_clients::ClientsState};
use crate::providers::{
    codex_catalog::{CATALOG_FILENAME, CodexFiles},
    live::{commit::*, write_guard::WriteError},
};
use std::{fs, os::unix::fs::PermissionsExt};
use vw_api_types::id::{ProjectId, ProjectScope, TenantScope};

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
        let f = Self { home, _root: root, tx, scope };
        f.write("config.toml", &format!("model_catalog_json = '{CATALOG_FILENAME}'\n"));
        f.write(CATALOG_FILENAME, "catalog-a");
        f.login("user-a", "token-a");
        f
    }
    fn write(&self, name: &str, value: &str) {
        fs::write(self.home.path().join(".codex").join(name), value).unwrap();
    }
    fn login(&self, user: &str, token: &str) {
        use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
        let header = URL_SAFE_NO_PAD.encode(r#"{"alg":"none"}"#);
        let payload =
            URL_SAFE_NO_PAD.encode(serde_json::json!({"sub":user,"nonce":token}).to_string());
        self.write("auth.json", &serde_json::json!({"tokens":{"account_id":"shared-workspace","id_token":format!("{header}.{payload}.signature"),"access_token":token}}).to_string());
    }
    fn observe(&self, time: u64) -> Snapshot {
        observe(&mut CodexFiles::open(self.home.path()).unwrap(), &self.tx, &self.scope, time)
            .unwrap()
    }
    fn read(&self) -> Snapshot {
        read(&CodexFiles::open(self.home.path()).unwrap(), &self.tx, &self.scope).unwrap()
    }
}
fn clients(f: &Fixture, confirmed: bool) -> ClientsState {
    ClientsState {
        revision: f.read().revision(),
        daemon: confirmed,
        others: confirmed,
        auth: confirmed,
        unverified: !confirmed,
    }
}
#[test]
fn aggregation_t28_catalog_ack_preserves_contract() {
    let f = Fixture::new();
    f.observe(10_000);
    let state = clients(&f, false);
    let before = fs::read(f.home.path().join(".codex").join(HISTORY_FILENAME)).unwrap();
    let mut files = CodexFiles::open(f.home.path()).unwrap();
    assert!(reminder(&files, &f.tx, &f.scope, &state).unwrap().is_some());
    assert_eq!(acknowledge(&mut files, &f.tx, &f.scope, &state.revision).unwrap(), state.revision);
    assert!(reminder(&files, &f.tx, &f.scope, &state).unwrap().is_none());
    drop(files);
    assert_eq!(before, fs::read(f.home.path().join(".codex").join(HISTORY_FILENAME)).unwrap());
    f.write(CATALOG_FILENAME, "catalog-b");
    let mut files = CodexFiles::open(f.home.path()).unwrap();
    assert_eq!(
        acknowledge(&mut files, &f.tx, &f.scope, &state.revision),
        Err(WriteError::Conflict)
    );
    drop(files);
    let changed = clients(&f, false);
    let files = CodexFiles::open(f.home.path()).unwrap();
    assert!(reminder(&files, &f.tx, &f.scope, &changed).unwrap().is_some());
}
#[test]
fn aggregation_t28_login_changes_and_confirmed_processes_remain_visible() {
    let f = Fixture::new();
    f.observe(10_000);
    let state = clients(&f, false);
    let mut files = CodexFiles::open(f.home.path()).unwrap();
    acknowledge(&mut files, &f.tx, &f.scope, &state.revision).unwrap();
    drop(files);
    f.login("user-b", "changed-secret");
    let changed = clients(&f, false);
    let mut files = CodexFiles::open(f.home.path()).unwrap();
    let hint = reminder(&files, &f.tx, &f.scope, &changed).unwrap().unwrap();
    assert!(hint.contains("彻底退出") && hint.contains("编辑器窗口"));
    acknowledge(&mut files, &f.tx, &f.scope, &changed.revision).unwrap();
    assert!(reminder(&files, &f.tx, &f.scope, &changed).unwrap().is_none());
    let confirmed =
        ClientsState { daemon: true, others: true, auth: true, unverified: false, ..changed };
    assert!(reminder(&files, &f.tx, &f.scope, &confirmed).unwrap().is_some());
    assert!(confirmed.daemon && confirmed.others && confirmed.auth);
}

#[test]
fn aggregation_t28_persists_only_private_revision_and_rejects_unsafe_inputs() {
    let f = Fixture::new();
    f.observe(10_000);
    let state = clients(&f, false);
    let dir = f.home.path().join(".codex");
    let names = ["config.toml", "auth.json", CATALOG_FILENAME, HISTORY_FILENAME];
    let before: Vec<_> = names.iter().map(|n| fs::read(dir.join(n)).unwrap()).collect();
    let other =
        ProjectScope::new(f.scope.tenant_scope().clone(), ProjectId::from("other")).unwrap();
    let mut files = CodexFiles::open(f.home.path()).unwrap();
    assert_eq!(
        acknowledge(&mut files, &f.tx, &other, &state.revision),
        Err(WriteError::InvalidResource)
    );
    assert!(!dir.join(ACK_FILENAME).exists());
    assert_eq!(acknowledge(&mut files, &f.tx, &f.scope, "old"), Err(WriteError::Conflict));
    assert!(!dir.join(ACK_FILENAME).exists());
    acknowledge(&mut files, &f.tx, &f.scope, &state.revision).unwrap();
    drop(files);
    let record = fs::read(dir.join(ACK_FILENAME)).unwrap();
    let value: serde_json::Value = serde_json::from_slice(&record).unwrap();
    assert_eq!(value, serde_json::json!({"revision":state.revision}));
    assert_eq!(fs::metadata(dir.join(ACK_FILENAME)).unwrap().permissions().mode() & 0o777, 0o600);
    for (name, original) in names.iter().zip(before) {
        assert_eq!(fs::read(dir.join(name)).unwrap(), original);
    }
    let mut files = CodexFiles::open(f.home.path()).unwrap();
    assert!(reminder(&files, &f.tx, &f.scope, &state).unwrap().is_none());
    acknowledge(&mut files, &f.tx, &f.scope, &state.revision).unwrap();
    assert_eq!(fs::read(dir.join(ACK_FILENAME)).unwrap(), record);
    drop(files);
    f.write(ACK_FILENAME, "{broken");
    let mut files = CodexFiles::open(f.home.path()).unwrap();
    assert_eq!(reminder(&files, &f.tx, &f.scope, &state), Err(WriteError::InvalidResource));
    fs::remove_file(dir.join(ACK_FILENAME)).unwrap();
    let outside = tempfile::NamedTempFile::new().unwrap();
    fs::write(outside.path(), b"protected").unwrap();
    std::os::unix::fs::symlink(outside.path(), dir.join(ACK_FILENAME)).unwrap();
    assert!(acknowledge(&mut files, &f.tx, &f.scope, &state.revision).is_err());
    assert!(reminder(&files, &f.tx, &f.scope, &state).is_err());
    assert_eq!(fs::read(outside.path()).unwrap(), b"protected");
}

#[tokio::test]
async fn aggregation_t28_real_codex_ack_does_not_restart_or_hide_stale_client() {
    use super::stale_clients::{self, ProcessIdentity};
    use std::{
        path::Path,
        process::{Command, Stdio},
    };
    struct Client(std::process::Child);
    impl Drop for Client {
        fn drop(&mut self) {
            let _ = self.0.kill();
            let _ = self.0.wait();
        }
    }
    let executable = std::env::var_os("T28_CODEX_BINARY")
        .expect("set T28_CODEX_BINARY to the installed real Codex CLI");
    let home = tempfile::tempdir().unwrap();
    let mut child = Client(
        Command::new(executable)
            .arg("app-server")
            .env("HOME", home.path())
            .env("CODEX_HOME", home.path())
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap(),
    );
    let f = Fixture::new();
    f.observe(10_000);
    f.write(CATALOG_FILENAME, "catalog-b");
    f.login("user-b", "changed-secret");
    let snapshot = f.observe(20_000);
    let table = stale_clients::process_table().await.expect("native ps is required");
    let row = table
        .lines()
        .find(|line| {
            line.split_whitespace().next().and_then(|v| v.parse::<u32>().ok()) == Some(child.0.id())
        })
        .expect("real Codex must be present");
    // Replace only the clock; the live process is within the new generation's 2s margin.
    let state = stale_clients::inspect(
        &snapshot,
        true,
        ProcessIdentity { daemon_pid: None, plugins_dir: Path::new("/isolated/.codex/plugins") },
        Some(row),
        20_000,
    );
    assert!(state.others && state.auth && !state.unverified);
    let mut files = CodexFiles::open(f.home.path()).unwrap();
    acknowledge(&mut files, &f.tx, &f.scope, &state.revision).unwrap();
    assert!(reminder(&files, &f.tx, &f.scope, &state).unwrap().is_some());
    assert!(child.0.try_wait().unwrap().is_none(), "ACK must leave the same real client alive");
    let after = stale_clients::process_table().await.unwrap();
    assert!(
        after.lines().any(|line| line
            .split_whitespace()
            .next()
            .and_then(|v| v.parse::<u32>().ok())
            == Some(child.0.id()))
    );
    println!(
        "real isolated Codex stayed alive with the same PID after ACK; stale reminder retained; owned child reaped"
    );
}
