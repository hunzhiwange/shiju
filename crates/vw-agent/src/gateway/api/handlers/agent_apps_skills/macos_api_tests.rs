//! macOS lifecycle acceptance through production HTTP routes and the real SDK.
//! No public network, real HOME, environment mutation, or install hooks.
use std::{fs, os::unix::fs::PermissionsExt};

use axum::{Extension, Router};
use serde_json::json;
use sha2::{Digest, Sha256};
use vw_api_types::{
    agent_apps_skills::{
        install_api::{InstallSkillRequest, SkillInstallSource, SkillInstallStatus},
        jobs_api::SkillJobPhase,
        repos_api::SkillReposQuery,
        uninstall_api::UninstallSkillRequest,
        updates_api::{CheckSkillUpdatesRequest, UpdateSkillRequest},
    },
    id::TenantScope,
};
use vw_gateway_client::{GatewayClient, GatewayEndpoint};
use crate::global_database::GlobalDatabase;

const TOKEN: &str = "macos-acceptance-fixture-token";

struct Fixture {
    root: tempfile::TempDir,
    policy: std::path::PathBuf,
    database: GlobalDatabase,
}

impl Fixture {
    fn new(database: GlobalDatabase) -> Self {
        let root = tempfile::tempdir().unwrap();
        fs::set_permissions(root.path(), fs::Permissions::from_mode(0o700)).unwrap();
        let source = root.path().join("source");
        let ssot = root.path().join("ssot");
        fs::create_dir(&source).unwrap();
        fs::create_dir(&ssot).unwrap();
        fs::write(source.join("SKILL.md"), "---\nname: Tool\ndescription: Fixture\n---\nInstructions").unwrap();
        // A packaged hook must remain inert during installation.
        fs::write(source.join("install.sh"), "#!/bin/sh\nexit 99\n").unwrap();
        let policy = root.path().join("access.json");
        fs::write(&policy, json!({
            "principals":[{"id":"installer","token_sha256":hex::encode(Sha256::digest(TOKEN)),
                "grants":[{"platform_id":"p","company_id":"c","host_id":"h",
                    "skill_repos_write_projects":["project"],"skill_repos_read_projects":["project"],
                    "skill_install_projects":["project"],"skill_inventory_read_projects":["project"]}]}],
            "hosts":[{"id":"h","online":true,"binding":"local","home":root.path(),
                "applications":root.path(),"search_paths":[],"skills_root":ssot,
                "skill_install_files":{"selected":source}}]
        }).to_string()).unwrap();
        fs::set_permissions(&policy, fs::Permissions::from_mode(0o600)).unwrap();
        Self { root, policy, database }
    }

    async fn serve(&self) -> Server {
        self.serve_at("https://codeload.github.com".into()).await
    }

    async fn serve_at(&self, archives: String) -> Server {
        let policy = Some(self.policy.clone());
        let routes = super::install_api::router_at(policy.clone(), archives.clone())
            .merge(super::discovery_api::router_at(policy.clone(), archives.clone(), archives.clone()))
            .merge(super::inventory_api::router_with_policy(policy.clone()))
            .merge(super::updates_api::router_at(policy.clone(), archives))
            .merge(super::jobs_api::router_with_policy(policy.clone()))
            .merge(super::uninstall_api::router_with_policy(policy))
            .layer(Extension(TenantScope::new("p", "c").unwrap()))
            .layer(Extension(self.database.clone()));
        let app = Router::new().nest("/v1", routes);
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
    fn client(&self) -> GatewayClient { GatewayClient::new(self.endpoint.clone()).unwrap() }
}
impl Drop for Server {
    fn drop(&mut self) { self.task.abort(); }
}
fn scope() -> SkillReposQuery {
    SkillReposQuery { project_id: "project".into(), target_host_id: "h".into() }
}
fn install() -> InstallSkillRequest {
    InstallSkillRequest { scope: scope(), expected_version: 0,
        source: SkillInstallSource::Local { file_id: "selected".into(), directory: "tool".into() } }
}

async fn lifecycle(f: &Fixture) {
    let server = f.serve().await;
    let client = server.client();
    assert!(client.agent_apps_skills_inventory(&scope(), TOKEN).await.unwrap().skills.is_empty());
    let result = client.agent_apps_skills_install(&install(), TOKEN).await.unwrap();
    assert_eq!(result.results.len(), 1);
    assert_eq!(result.results[0].status, SkillInstallStatus::Installed);
    assert_eq!(result.results[0].target_host_id, "h");
    let inventory = client.agent_apps_skills_inventory(&scope(), TOKEN).await.unwrap();
    assert_eq!(inventory.version, 1);
    assert_eq!(inventory.skills.len(), 1);
    assert!(inventory.skills[0].present);
    let detail = client.agent_apps_skills_detail(&scope(), "tool", TOKEN).await.unwrap();
    assert_eq!(detail.content_hash, inventory.skills[0].content_hash);
    let before = fs::read(f.root.path().join("ssot/tool/SKILL.md")).unwrap();
    let check = client.agent_apps_skills_check_updates(&CheckSkillUpdatesRequest { scope: scope() }, TOKEN).await.unwrap();
    assert_eq!(check.version, 1);
    assert_eq!(check.without_repository, vec!["tool"]);
    assert!(check.updates.is_empty());
    // Local provenance cannot be silently upgraded into a network source.
    assert!(client.agent_apps_skills_update("tool", &UpdateSkillRequest {
        scope: scope(), expected_version: 1, expected_hash: detail.content_hash.unwrap(),
    }, TOKEN).await.is_err());
    assert_eq!(fs::read(f.root.path().join("ssot/tool/SKILL.md")).unwrap(), before);
    let job = client.agent_apps_skills_job(&scope(), &result.task_id, TOKEN).await.unwrap();
    assert_eq!(job.phase, SkillJobPhase::Completed);
    // Closing connections and restarting the HTTP server preserves durable observation.
    drop(client);
    drop(server);
    let server = f.serve().await;
    let client = server.client();
    let reread = client.agent_apps_skills_job(&scope(), &result.task_id, TOKEN).await.unwrap();
    assert_eq!(serde_json::to_value(&reread).unwrap(), serde_json::to_value(&job).unwrap());
    let cancelled = client.agent_apps_skills_cancel_job(&scope(), &result.task_id, TOKEN).await.unwrap();
    assert_eq!(cancelled.phase, SkillJobPhase::Completed);
    let removed = client.agent_apps_skills_uninstall("tool", &UninstallSkillRequest {
        scope: scope(), expected_version: 1,
    }, TOKEN).await.unwrap();
    assert_eq!(removed.version, 2);
    assert!(!removed.cleanup_incomplete);
    assert!(removed.backup_path.join("skill/SKILL.md").is_file());
    assert!(!f.root.path().join("ssot/tool").exists());
    let inventory = client.agent_apps_skills_inventory(&scope(), TOKEN).await.unwrap();
    assert_eq!(inventory.version, 2);
    assert!(inventory.skills.is_empty());
}

#[tokio::test]
async fn cp12_macos_api_success() {
    let home = tempfile::tempdir().unwrap();
    let fixture = Fixture::new(GlobalDatabase::for_home(home.path()).unwrap());
    lifecycle(&fixture).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn cp12_macos_api_postgres_success() {
    // A missing database is an explicit failure, never a zero-work success.
    let database = crate::global_database::test_support::PostgresTestDatabase::from_environment()
        .expect("macOS API acceptance requires VIBEWINDOW_TEST_POSTGRES_URL");
    let fixture = Fixture::new(database.database().clone());
    lifecycle(&fixture).await;
}

#[tokio::test]
async fn cp12_macos_api_rejects_without_side_effects() {
    let home = tempfile::tempdir().unwrap();
    let f = Fixture::new(GlobalDatabase::for_home(home.path()).unwrap());
    let server = f.serve().await;
    let client = server.client();
    assert!(client.agent_apps_skills_install(&install(), "invalid").await.is_err());
    for (project, host) in [("other", "h"), ("project", "other")] {
        let mut request = install();
        request.scope.project_id = project.into();
        request.scope.target_host_id = host.into();
        assert!(client.agent_apps_skills_install(&request, TOKEN).await.is_err());
    }
    for (file_id, directory) in [("/client/path", "tool"), ("selected", "../escape")] {
        let mut request = install();
        request.source = SkillInstallSource::Local { file_id: file_id.into(), directory: directory.into() };
        assert!(client.agent_apps_skills_install(&request, TOKEN).await.is_err());
    }
    let inventory = client.agent_apps_skills_inventory(&scope(), TOKEN).await.unwrap();
    assert_eq!(inventory.version, 0);
    assert!(inventory.skills.is_empty());
    assert_eq!(fs::read_dir(f.root.path().join("ssot")).unwrap().count(), 0);
    assert!(!f.root.path().join("escape").exists());
    assert!(!f.root.path().join("ssot.backups").exists());
}

/// Only the external GitHub archive response is substituted. Extraction,
/// persistence, publication, authorization, routes and SDK remain production code.
async fn remote_update(f: &Fixture, cancel: bool) {
    use std::{io::Write, sync::{Arc, atomic::{AtomicUsize, Ordering}}};
    use axum::{extract::State, routing::get};
    use crate::gateway::agent_apps_skills_store::SkillReposStore;
    use vw_agent_apps::skills::repos::SkillRepo;
    use vw_api_types::{id::ProjectScope, agent_apps_skills::updates_api::SkillUpdatePhase};

    fn archive(content: &str) -> Vec<u8> {
        let mut zip = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
        zip.start_file("repo-main/skills/tool/SKILL.md", zip::write::SimpleFileOptions::default()).unwrap();
        zip.write_all(format!("---\nname: Tool\ndescription: Fixture\n---\n{content}").as_bytes()).unwrap();
        zip.finish().unwrap().into_inner()
    }
    #[derive(Clone)]
    struct ArchiveState {
        requests: Arc<AtomicUsize>,
        entered: Arc<tokio::sync::Notify>,
        release: Arc<tokio::sync::Notify>,
        cancel: bool,
    }
    async fn download(State(state): State<ArchiveState>) -> Vec<u8> {
        let request = state.requests.fetch_add(1, Ordering::SeqCst);
        if request < 2 { return archive("old"); }
        if state.cancel {
            state.entered.notify_one();
            state.release.notified().await;
        }
        archive("new")
    }
    let state = ArchiveState { requests: Arc::new(AtomicUsize::new(0)),
        entered: Arc::new(tokio::sync::Notify::new()), release: Arc::new(tokio::sync::Notify::new()), cancel };
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let endpoint = GatewayEndpoint::new("127.0.0.1", listener.local_addr().unwrap().port());
    let origin = format!("http://127.0.0.1:{}", listener.local_addr().unwrap().port());
    let app = Router::new().route("/owner/repo/zip/refs/heads/main", get(download)).with_state(state.clone());
    let task = tokio::spawn(async move { axum::serve(listener, app).await.unwrap(); });
    let _archive_server = Server { endpoint, task };
    let project = ProjectScope::new(TenantScope::new("p", "c").unwrap(), "project".into()).unwrap();
    SkillReposStore::new(f.database.clone()).unwrap().save(&project, 0,
        SkillRepo { owner: "owner".into(), name: "repo".into(), branch: "main".into(), enabled: true }).unwrap();
    let root = f.root.path().join("ssot");
    let server = f.serve_at(origin.clone()).await;
    let client = server.client();
    let discovered = client.agent_apps_skills_discover(
        &vw_api_types::agent_apps_skills::discovery_api::DiscoverSkillsRequest {
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
    tokio::time::timeout(std::time::Duration::from_secs(10), async {
        loop {
            let job = client.agent_apps_skills_job(&scope(), &installed.task_id, TOKEN).await.unwrap();
            if job.phase == SkillJobPhase::Completed { break; }
            assert!(!matches!(job.phase, SkillJobPhase::Failed | SkillJobPhase::Interrupted));
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
    }).await.unwrap();
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
    for cancel in [false, true] {
        let home = tempfile::tempdir().unwrap();
        remote_update(&Fixture::new(GlobalDatabase::for_home(home.path()).unwrap()), cancel).await;
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn cp12_macos_api_postgres_remote_update_and_precommit_cancel() {
    for cancel in [false, true] {
        let database = crate::global_database::test_support::PostgresTestDatabase::from_environment()
            .expect("macOS API acceptance requires VIBEWINDOW_TEST_POSTGRES_URL");
        remote_update(&Fixture::new(database.database().clone()), cancel).await;
    }
}
