//! macOS lifecycle acceptance through production HTTP routes and the real SDK.
//! The child process uses a temporary home and a loopback archive server.
use std::{fs, io::Write, path::{Path, PathBuf}, sync::{Arc, atomic::{AtomicUsize, Ordering}}};

use axum::{Extension, Router, extract::State, routing::get};
use app_settings_store::agent_apps_skills_store::SkillReposStore;
use vw_agent_apps::skills::repos::SkillRepo;
use {
    vw_api_app_skills::install_api::InstallSkillRequest,
    vw_api_app_skills::install_api::SkillInstallSource,
    vw_api_app_skills::install_api::SkillInstallStatus,
    vw_api_app_skills::jobs_api::SkillJobPhase,
    vw_api_app_skills::repos_api::SkillReposQuery,
    vw_api_app_skills::uninstall_api::UninstallSkillRequest,
    vw_api_app_skills::updates_api::CheckSkillUpdatesRequest,
    vw_api_app_skills::updates_api::UpdateSkillRequest,
    vw_api_transport::id::{ProjectScope, TenantScope},
};
use gateway_endpoint::{GatewayAuth, GatewayEndpoint};
use gateway_app_skills::{GatewayAppSkillsClient, SkillInstallError};
use agent_database::GlobalDatabase;

const TOKEN: &str = "macos-acceptance-fixture-token";

// Runtime paths are cached; initialize them only in a private child process.
fn isolated_test() -> bool {
    const CHILD: &str = "SHIJU_MACOS_SKILLS_CHILD";
    if std::env::var_os(CHILD).is_some() {
        let root = PathBuf::from(std::env::var_os("VIBEWINDOW_TEST_ROOT").unwrap());
        assert!(root.starts_with(std::env::temp_dir().canonicalize().unwrap()));
        assert_eq!(std::env::var_os("HOME").as_deref(), Some(root.as_os_str()));
        assert_eq!(std::env::current_dir().unwrap(), root);
        assert_eq!(std::env::var_os("CODEX_HOME").as_deref(), Some(root.join(".codex").as_os_str()));
        assert_eq!(agent_paths::paths().home, root);
        assert!(agent_paths::paths().config.starts_with(&root));
        return false;
    }
    let root = tempfile::tempdir().unwrap();
    let isolated_home = root.path().canonicalize().unwrap();
    let status = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", std::thread::current().name().unwrap(), "--nocapture"])
        .current_dir(&isolated_home)
        .env(CHILD, "1")
        .env("HOME", &isolated_home)
        .env("CODEX_HOME", isolated_home.join(".codex"))
        .env("VIBEWINDOW_TEST_ROOT", &isolated_home)
        .status().unwrap();
    assert!(status.success(), "isolated macOS skills child failed: {status}");
    true
}

struct Fixture {
    root: tempfile::TempDir,
    database: GlobalDatabase,
}

impl Fixture {
    fn new(database: GlobalDatabase) -> Self {
        let root = tempfile::tempdir().unwrap();
        let execution_root = agent_paths::paths().home.join(".agents/skills");
        fs::create_dir_all(&execution_root).unwrap();
        std::os::unix::fs::symlink(&execution_root, root.path().join("ssot")).unwrap();
        Self { root, database }
    }

    async fn serve_at(&self, archives: String) -> Server {
        let routes = super::install_api::router_at(None, archives.clone())
            .merge(super::discovery_api::router_at(None, archives.clone(), archives.clone()))
            .merge(super::inventory_api::router_with_policy(None))
            .merge(super::bindings_api::router())
            .merge(super::updates_api::router_at(None, archives))
            .merge(super::jobs_api::router_with_policy(None))
            .merge(super::uninstall_api::router_with_policy(None))
            .layer(Extension(self.database.clone()))
            .layer(axum::middleware::from_fn(agent_gateway::middleware::tenant_scope_middleware));
        let state = agent_gateway::gateway_test_fixture::state::<(), (), ()>(
            Default::default(), true, &[TOKEN.to_owned()],
        );
        let app = Router::new().nest("/v1", routes).layer(axum::middleware::from_fn_with_state(
            state, agent_gateway::request_policy::skey_auth_middleware::<(), (), ()>,
        ));
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let endpoint = GatewayEndpoint::new("127.0.0.1", listener.local_addr().unwrap().port())
            .with_tenant_scope(TenantScope::new("p", "c").unwrap());
        let task = tokio::spawn(async move { axum::serve(listener, app).await.unwrap(); });
        Server { endpoint, task }
    }
}

struct Server {
    endpoint: GatewayEndpoint,
    task: tokio::task::JoinHandle<()>,
}
impl Server {
    fn client(&self) -> GatewayAppSkillsClient {
        self.client_with(self.endpoint.tenant_scope().clone(), TOKEN)
    }

    fn client_with(&self, tenant: TenantScope, token: &str) -> GatewayAppSkillsClient {
        let endpoint = self.endpoint.clone().with_tenant_scope(tenant)
            .with_auth(GatewayAuth { skey: Some(token.into()) });
        GatewayAppSkillsClient::new(gateway_http::GatewayTransport::new(endpoint).unwrap())
    }
}
impl Drop for Server {
    fn drop(&mut self) { self.task.abort(); }
}
fn scope() -> SkillReposQuery {
    SkillReposQuery { project_id: "project".into(), target_host_id: "h".into() }
}
fn install() -> InstallSkillRequest {
    InstallSkillRequest { scope: scope(), expected_version: 0,
        source: SkillInstallSource::Git { repo_id: "owner:repo".into(),
            selection: "skills/tool".into(), directory: "tool".into() } }
}

#[tokio::test]
async fn default_skill_directory_binding_needs_no_cli_installation() {
    use app_settings_store::agent_apps_skills_store::SkillInstallsStore;
    use vw_api_app_identity::AppId;
    use vw_api_app_skills::bindings_api::{
        SkillBindingMode, SkillBindingStatus, SkillBindingTarget, SkillBindingsRequest,
    };

    if isolated_test() {
        return;
    }
    let home = &agent_paths::paths().home;
    let fixture = Fixture::new(GlobalDatabase::for_home(home).unwrap());
    let source = fixture.root.path().join("source");
    fs::create_dir(&source).unwrap();
    fs::write(
        source.join("SKILL.md"),
        "---\nname: Tool\ndescription: Example\n---\nContent",
    ).unwrap();
    let scope = scope();
    let project = ProjectScope::new(TenantScope::new("p", "c").unwrap(), scope.project_id.clone())
        .unwrap();
    let root = home.join(".agents/skills");
    SkillInstallsStore::new(fixture.database.clone()).unwrap()
        .install_local(&project, &scope.target_host_id, 0, &root, &source, "tool").unwrap();

    let destination = home.join(".codex/skills/tool");
    assert!(!destination.parent().unwrap().exists());
    let server = fixture.serve_at("http://127.0.0.1:1".into()).await;
    let client = server.client();
    let target = SkillBindingTarget::application(AppId::Codex);
    let mut request = SkillBindingsRequest {
        scope,
        targets: vec![target.clone()],
        enabled: true,
        mode: SkillBindingMode::Auto,
    };
    let response = client.agent_apps_skills_bindings("tool", &request, TOKEN).await.unwrap();
    assert_eq!(response.results.len(), 1);
    assert_eq!(response.results[0].target, target);
    assert_eq!(response.results[0].status, SkillBindingStatus::Enabled);
    assert!(destination.join("SKILL.md").is_file());

    request.enabled = false;
    let response = client.agent_apps_skills_bindings("tool", &request, TOKEN).await.unwrap();
    assert_eq!(response.results[0].status, SkillBindingStatus::Disabled);
    assert!(!destination.exists());
    assert!(root.join("tool/SKILL.md").is_file());
}

async fn lifecycle(f: &Fixture) {
    let (archive, _) = serve_archive(f, false, false).await;
    seed_repository(f).await;
    let origin = archive.endpoint.base_url();
    let server = f.serve_at(origin.clone()).await;
    let client = server.client();
    assert!(client.agent_apps_skills_inventory(&scope(), TOKEN).await.unwrap().skills.is_empty());
    let task_id = install_discovered(&client).await;
    let installed = f.root.path().join("ssot/tool");
    assert!(installed.join("install.sh").is_file());
    assert!(!f.root.path().join("hook-ran").exists());
    let inventory = client.agent_apps_skills_inventory(&scope(), TOKEN).await.unwrap();
    assert_eq!(inventory.version, 1);
    assert_eq!(inventory.skills.len(), 1);
    assert!(inventory.skills[0].present);
    let detail = client.agent_apps_skills_detail(&scope(), "tool", TOKEN).await.unwrap();
    assert_eq!(detail.content_hash, inventory.skills[0].content_hash);
    let before = fs::read(f.root.path().join("ssot/tool/SKILL.md")).unwrap();
    let check = client.agent_apps_skills_check_updates(&CheckSkillUpdatesRequest { scope: scope() }, TOKEN).await.unwrap();
    assert_eq!(check.version, 1);
    assert!(check.without_repository.is_empty());
    assert!(check.failures.is_empty());
    assert!(check.skill_failures.is_empty());
    assert_eq!(check.unchanged, vec!["tool"]);
    assert!(check.updates.is_empty());
    // A stale hash cannot change a repository-backed installation.
    assert!(client.agent_apps_skills_update("tool", &UpdateSkillRequest {
        scope: scope(), expected_version: 1, expected_hash: "0".repeat(64),
    }, TOKEN).await.is_err());
    assert_eq!(fs::read(f.root.path().join("ssot/tool/SKILL.md")).unwrap(), before);
    let job = client.agent_apps_skills_job(&scope(), &task_id, TOKEN).await.unwrap();
    assert_eq!(job.phase, SkillJobPhase::Completed);
    // Closing connections and restarting the HTTP server preserves durable observation.
    drop(client);
    drop(server);
    let server = f.serve_at(origin).await;
    let client = server.client();
    let reread = client.agent_apps_skills_job(&scope(), &task_id, TOKEN).await.unwrap();
    assert_eq!(serde_json::to_value(&reread).unwrap(), serde_json::to_value(&job).unwrap());
    let cancelled = client.agent_apps_skills_cancel_job(&scope(), &task_id, TOKEN).await.unwrap();
    assert_eq!(cancelled.phase, SkillJobPhase::Completed);
    let removed = client.agent_apps_skills_uninstall("tool", &UninstallSkillRequest {
        scope: scope(), expected_version: 1,
    }, TOKEN).await.unwrap();
    assert_eq!(removed.version, 2);
    assert!(!removed.cleanup_incomplete);
    assert!(removed.backup_path.starts_with(&agent_paths::paths().home));
    assert!(removed.backup_path.join("skill/SKILL.md").is_file());
    assert!(!f.root.path().join("hook-ran").exists());
    assert!(!f.root.path().join("ssot/tool").exists());
    let inventory = client.agent_apps_skills_inventory(&scope(), TOKEN).await.unwrap();
    assert_eq!(inventory.version, 2);
    assert!(inventory.skills.is_empty());
}

#[tokio::test]
async fn cp12_macos_api_success() {
    if isolated_test() { return; }
    let home = tempfile::tempdir().unwrap();
    let fixture = Fixture::new(GlobalDatabase::for_home(home.path()).unwrap());
    lifecycle(&fixture).await;
}

#[test]
fn cp12_macos_api_postgres_success() {
    if isolated_test() { return; }
    // A missing database is an explicit failure, never a zero-work success.
    let database = agent_database::test_support::PostgresTestDatabase::from_environment()
        .expect("macOS API acceptance requires VIBEWINDOW_TEST_POSTGRES_URL");
    let fixture = Fixture::new(database.database().clone());
    tokio::runtime::Runtime::new().unwrap().block_on(lifecycle(&fixture));
}

#[tokio::test]
async fn cp12_macos_api_rejects_without_side_effects() {
    if isolated_test() { return; }
    let home = tempfile::tempdir().unwrap();
    let f = Fixture::new(GlobalDatabase::for_home(home.path()).unwrap());
    let (archive, state) = serve_archive(&f, false, false).await;
    seed_repository(&f).await;
    let server = f.serve_at(archive.endpoint.base_url()).await;
    let client = server.client();
    let denied = server.client_with(TenantScope::new("p", "c").unwrap(), "invalid");
    assert!(matches!(denied.agent_apps_skills_install(&install(), "invalid").await,
        Err(SkillInstallError::Http { status: 401 })));
    let foreign = server.client_with(TenantScope::new("p", "other-company").unwrap(), TOKEN);
    assert!(matches!(foreign.agent_apps_skills_install(&install(), TOKEN).await,
        Err(SkillInstallError::Http { status: 404 })));
    let mut request = install();
    request.scope.project_id = "other".into();
    assert!(matches!(client.agent_apps_skills_install(&request, TOKEN).await,
        Err(SkillInstallError::Http { status: 404 })));
    for (file_id, status) in [("/client/path", 400), ("unregistered", 403)] {
        let mut request = install();
        request.source = SkillInstallSource::Local { file_id: file_id.into(), directory: "tool".into() };
        assert!(matches!(client.agent_apps_skills_install(&request, TOKEN).await,
            Err(SkillInstallError::Http { status: actual }) if actual == status));
    }
    for (selection, directory) in [("../escape", "tool"), ("skills/tool", "../escape")] {
        let mut request = install();
        request.source = SkillInstallSource::Git { repo_id: "owner:repo".into(),
            selection: selection.into(), directory: directory.into() };
        assert!(matches!(client.agent_apps_skills_install(&request, TOKEN).await,
        Err(SkillInstallError::Http { status: 400 })));
    }
    assert_eq!(state.requests.load(Ordering::SeqCst), 0);
    let inventory = client.agent_apps_skills_inventory(&scope(), TOKEN).await.unwrap();
    assert_eq!(inventory.version, 0);
    assert!(inventory.skills.is_empty());
    assert_eq!(fs::read_dir(f.root.path().join("ssot")).unwrap().count(), 0);
    let root = agent_paths::paths().home.join(".agents/skills");
    assert!(!root.with_file_name("escape").exists());
    let project = ProjectScope::new(TenantScope::new("p", "c").unwrap(), "project".into()).unwrap();
    assert!(!super::backups_api::backup_root(&root, &project, "h").unwrap().exists());
}

// The archive is the only substituted external service.
#[derive(Clone)]
struct ArchiveState {
    requests: Arc<AtomicUsize>,
    entered: Arc<tokio::sync::Notify>,
    release: Arc<tokio::sync::Notify>,
    hook_marker: PathBuf,
    update: bool,
    cancel: bool,
}

fn archive(content: &str, hook_marker: &Path) -> Vec<u8> {
    let mut zip = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
    zip.start_file("repo-main/skills/tool/SKILL.md", zip::write::SimpleFileOptions::default()).unwrap();
    zip.write_all(format!("---\nname: Tool\ndescription: Fixture\n---\n{content}").as_bytes()).unwrap();
    zip.start_file("repo-main/skills/tool/install.sh",
        zip::write::SimpleFileOptions::default().unix_permissions(0o755)).unwrap();
    let marker = hook_marker.to_str().unwrap().replace('\'', "'\\''");
    write!(zip, "#!/bin/sh\n: > '{marker}'\nexit 99\n").unwrap();
    zip.finish().unwrap().into_inner()
}

async fn serve_archive(f: &Fixture, update: bool, cancel: bool) -> (Server, ArchiveState) {
    async fn download(State(state): State<ArchiveState>) -> Vec<u8> {
        let request = state.requests.fetch_add(1, Ordering::SeqCst);
        if !state.update || request < 2 { return archive("old", &state.hook_marker); }
        if state.cancel {
            state.entered.notify_one();
            state.release.notified().await;
        }
        archive("new", &state.hook_marker)
    }

    let state = ArchiveState {
        requests: Arc::new(AtomicUsize::new(0)),
        entered: Arc::new(tokio::sync::Notify::new()),
        release: Arc::new(tokio::sync::Notify::new()),
        hook_marker: f.root.path().join("hook-ran"),
        update,
        cancel,
    };
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let endpoint = GatewayEndpoint::new("127.0.0.1", listener.local_addr().unwrap().port());
    let app = Router::new().route("/owner/repo/zip/refs/heads/main", get(download)).with_state(state.clone());
    let task = tokio::spawn(async move { axum::serve(listener, app).await.unwrap(); });
    (Server { endpoint, task }, state)
}

async fn seed_repository(f: &Fixture) {
    let project = ProjectScope::new(TenantScope::new("p", "c").unwrap(), "project".into()).unwrap();
    let database = f.database.clone();
    tokio::task::spawn_blocking(move || {
        SkillReposStore::new(database).unwrap().save(&project, 0,
            SkillRepo { owner: "owner".into(), name: "repo".into(), branch: "main".into(), enabled: true }).unwrap();
    }).await.unwrap();
}

async fn install_discovered(client: &GatewayAppSkillsClient) -> vw_api_transport::id::TaskId {
    let discovered = client.agent_apps_skills_discover(
        &vw_api_app_skills::discovery_api::DiscoverSkillsRequest {
            scope: scope(), repo_id: Some("owner:repo".into()), force_refresh: true,
        }, TOKEN).await.unwrap();
    assert!(discovered.failures.is_empty());
    assert_eq!(discovered.skills.len(), 1);
    assert_eq!(discovered.skills[0].directory, "skills/tool");
    let installed = client.agent_apps_skills_install(&InstallSkillRequest {
        scope: scope(), expected_version: 0,
        source: SkillInstallSource::Git { repo_id: "owner:repo".into(),
            selection: discovered.skills[0].directory.clone(), directory: "tool".into() },
    }, TOKEN).await.unwrap();
    assert_eq!(installed.results.len(), 1);
    assert_eq!(installed.results[0].status, SkillInstallStatus::Pending);
    assert_eq!(installed.results[0].target_host_id, "h");
    tokio::time::timeout(std::time::Duration::from_secs(10), async {
        loop {
            let job = client.agent_apps_skills_job(&scope(), &installed.task_id, TOKEN).await.unwrap();
            if job.phase == SkillJobPhase::Completed { break; }
            assert!(!matches!(job.phase, SkillJobPhase::Failed | SkillJobPhase::Interrupted));
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
    }).await.unwrap();
    installed.task_id
}

async fn remote_update(f: &Fixture, cancel: bool) {
    use vw_api_app_skills::updates_api::SkillUpdatePhase;

    let (archive, state) = serve_archive(f, true, cancel).await;
    seed_repository(f).await;
    let root = f.root.path().join("ssot");
    let origin = archive.endpoint.base_url();
    let server = f.serve_at(origin.clone()).await;
    let client = server.client();
    install_discovered(&client).await;
    let before = client.agent_apps_skills_inventory(&scope(), TOKEN).await.unwrap();
    let old_bytes = fs::read(root.join("tool/SKILL.md")).unwrap();
    if !cancel {
        let report = client.agent_apps_skills_check_updates(&CheckSkillUpdatesRequest { scope: scope() }, TOKEN).await.unwrap();
        assert_eq!(report.updates.len(), 1);
        assert_eq!(report.updates[0].directory, "tool");
        assert!(report.failures.is_empty());
    }
    for rejected_scope in [
        SkillReposQuery { project_id: "other".into(), target_host_id: "h".into() },
        SkillReposQuery { project_id: "project".into(), target_host_id: "other".into() },
    ] {
        assert!(client.agent_apps_skills_update("tool", &UpdateSkillRequest {
            scope: rejected_scope, expected_version: 1,
            expected_hash: before.skills[0].content_hash.clone().unwrap(),
        }, TOKEN).await.is_err());
    }
    assert_eq!(client.agent_apps_skills_inventory(&scope(), TOKEN).await.unwrap(), before);
    let started = client.agent_apps_skills_update("tool", &UpdateSkillRequest {
        scope: scope(), expected_version: 1, expected_hash: before.skills[0].content_hash.clone().unwrap(),
    }, TOKEN).await.unwrap();
    if cancel {
        tokio::time::timeout(std::time::Duration::from_secs(5), state.entered.notified()).await.unwrap();
        let snapshot = client.agent_apps_skills_cancel_job(&scope(), &started.task_id, TOKEN).await.unwrap();
        assert_eq!(snapshot.phase, SkillJobPhase::Started);
        state.release.notify_one();
    }
    let terminal = tokio::time::timeout(std::time::Duration::from_secs(10), async {
        loop {
            let snapshot = client.agent_apps_skills_job(&scope(), &started.task_id, TOKEN).await.unwrap();
            if matches!(snapshot.phase, SkillJobPhase::Completed | SkillJobPhase::Failed | SkillJobPhase::Interrupted) {
                break snapshot;
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
    }).await.unwrap();
    assert_eq!(terminal.phase, if cancel { SkillJobPhase::Interrupted } else { SkillJobPhase::Completed });
    assert!(!terminal.requires_reconciliation);
    drop(client);
    drop(server);
    let server = f.serve_at(origin).await;
    let client = server.client();
    let update = client.agent_apps_skills_update_task(started.task_id.as_ref(), &scope(), TOKEN).await.unwrap();
    assert!(matches!((&update.phase, cancel), (SkillUpdatePhase::Completed, false) | (SkillUpdatePhase::Interrupted, true)));
    let after = client.agent_apps_skills_inventory(&scope(), TOKEN).await.unwrap();
    if cancel {
        assert_eq!(after, before);
        assert_eq!(fs::read(root.join("tool/SKILL.md")).unwrap(), old_bytes);
    } else {
        assert_eq!(after.version, 2);
        assert_ne!(after.skills[0].content_hash, before.skills[0].content_hash);
        assert!(fs::read_to_string(root.join("tool/SKILL.md")).unwrap().ends_with("new"));
    }
    let removed = client.agent_apps_skills_uninstall("tool", &UninstallSkillRequest {
        scope: scope(), expected_version: after.version,
    }, TOKEN).await.unwrap();
    assert_eq!(removed.version, after.version + 1);
    assert!(client.agent_apps_skills_inventory(&scope(), TOKEN).await.unwrap().skills.is_empty());
}

#[tokio::test]
async fn cp12_macos_api_remote_update_and_precommit_cancel() {
    if isolated_test() { return; }
    for cancel in [false, true] {
        let home = tempfile::tempdir().unwrap();
        remote_update(&Fixture::new(GlobalDatabase::for_home(home.path()).unwrap()), cancel).await;
    }
}

#[test]
fn cp12_macos_api_postgres_remote_update_and_precommit_cancel() {
    if isolated_test() { return; }
    for cancel in [false, true] {
        let database = agent_database::test_support::PostgresTestDatabase::from_environment()
            .expect("macOS API acceptance requires VIBEWINDOW_TEST_POSTGRES_URL");
        let fixture = Fixture::new(database.database().clone());
        tokio::runtime::Runtime::new().unwrap().block_on(remote_update(&fixture, cancel));
    }
}
