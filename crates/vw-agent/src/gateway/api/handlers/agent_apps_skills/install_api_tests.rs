use axum::{Extension, body::Body, http::{Request, StatusCode}};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{fs, os::unix::fs::PermissionsExt};
use tower::ServiceExt;
use vw_api_types::id::{ProjectScope, TenantScope};
use crate::{global_database::GlobalDatabase, gateway::agent_apps_skills_store::SkillInstallsStore};

const TOKEN: &str = "skill-install-test-token-00000000000";
struct Fixture { root: tempfile::TempDir, policy: std::path::PathBuf, db: GlobalDatabase }
impl Fixture {
    fn new(allowed: bool, binding: &str) -> Self {
        let root = tempfile::tempdir().unwrap();
        fs::set_permissions(root.path(), fs::Permissions::from_mode(0o700)).unwrap();
        let ssot = root.path().join("ssot");
        let source = root.path().join("source");
        fs::create_dir(&ssot).unwrap();
        fs::create_dir(&source).unwrap();
        fs::write(source.join("SKILL.md"), "---\nname: Fixture\n---\n# Instructions").unwrap();
        let policy = root.path().join("access.json");
        fs::write(&policy, json!({"principals":[{"id":"installer",
            "token_sha256":hex::encode(Sha256::digest(TOKEN)),"grants":[{
            "platform_id":"p","company_id":"c","host_id":"h",
            "skill_install_projects":if allowed { vec!["project"] } else { vec![] },
            "skill_repos_write_projects":["project"]}]}],
            "hosts":[{"id":"h","online":true,"binding":binding,"home":root.path(),
            "applications":root.path(),"search_paths":[],"skills_root":ssot,
            "skill_install_files":{"selected":source}}]}).to_string()).unwrap();
        fs::set_permissions(&policy, fs::Permissions::from_mode(0o600)).unwrap();
        let db = GlobalDatabase::for_home(root.path()).unwrap();
        Self { root, policy, db }
    }
    async fn call(&self, body: Value) -> (StatusCode, Value) {
        let app = super::install_api::router_with_policy(Some(self.policy.clone()))
            .layer(Extension(TenantScope::new("p", "c").unwrap()))
            .layer(Extension(self.db.clone()));
        let response = app.oneshot(Request::post("/agent-apps/skills/install")
            .header("content-type", "application/json").header("x-agent-apps-token", TOKEN)
            .body(Body::from(body.to_string())).unwrap()).await.unwrap();
        let status = response.status();
        let bytes = axum::body::to_bytes(response.into_body(), 16384).await.unwrap();
        (status, serde_json::from_slice(&bytes).unwrap_or(Value::Null))
    }
    fn state(&self) -> crate::gateway::agent_apps_skills_store::SkillInstallsState {
        SkillInstallsStore::new(self.db.clone()).unwrap().get(&project(), "h", &self.root.path().join("ssot")).unwrap()
    }
}
fn project() -> ProjectScope {
    ProjectScope::new(TenantScope::new("p", "c").unwrap(), "project".into()).unwrap()
}
fn request() -> Value {
    json!({"scope":{"projectId":"project","targetHostId":"h"},"expectedVersion":0,
        "source":{"kind":"local","fileId":"selected","directory":"fixture"}})
}

#[tokio::test]
async fn cp12_install_api_success() {
    let f = Fixture::new(true, "local");
    let (status, result) = f.call(request()).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(result["results"][0]["status"], "installed");
    assert_eq!(result["results"][0]["targetHostId"], "h");
    assert_eq!(f.state().version, 1);
    assert!(f.root.path().join("ssot/fixture/SKILL.md").is_file());
    let id: vw_api_types::id::TaskId = serde_json::from_value(result["taskId"].clone()).unwrap();
    let job = SkillInstallsStore::new(f.db.clone()).unwrap().job_snapshot(&project(), "h", &id).unwrap();
    assert_eq!(job.phase, vw_agent_apps::skills::jobs::SkillJobPhase::Completed);
    assert_eq!(f.call(request()).await.0, StatusCode::CONFLICT);
    assert_eq!(f.state().version, 1);
}

#[tokio::test]
async fn cp12_install_api_rejects_without_side_effects() {
    for (allowed, binding, expected) in [(false, "local", StatusCode::FORBIDDEN),
        (true, "remote", StatusCode::NOT_IMPLEMENTED)] {
        let f = Fixture::new(allowed, binding);
        assert_eq!(f.call(request()).await.0, expected);
        assert_eq!(f.state().version, 0);
        assert_eq!(fs::read_dir(f.root.path().join("ssot")).unwrap().count(), 0);
    }
    let f = Fixture::new(true, "local");
    for field in ["projectId", "targetHostId"] {
        let mut body = request();
        body["scope"][field] = json!("other");
        assert!(!f.call(body).await.0.is_success());
    }
    for source in [json!({"kind":"local","fileId":"missing","directory":"fixture"}),
        json!({"kind":"archive","fileId":"/client/archive.zip"}),
        json!({"kind":"local","fileId":"selected","directory":"../escape"}),
        json!({"kind":"archive","fileId":"selected","path":"/client/archive.zip"})] {
        let mut body = request(); body["source"] = source;
        assert!(!f.call(body).await.0.is_success());
    }
    assert_eq!(f.state().version, 0);
    assert_eq!(fs::read_dir(f.root.path().join("ssot")).unwrap().count(), 0);
}

#[tokio::test]
async fn cp12_install_api_archive_preserves_individual_results() {
    use std::io::Write;
    let f = Fixture::new(true, "local");
    let archive = f.root.path().join("bundle.zip");
    let mut writer = zip::ZipWriter::new(fs::File::create(&archive).unwrap());
    for name in ["alpha/SKILL.md", "existing/SKILL.md", ".bad/SKILL.md"] {
        writer.start_file(name, zip::write::SimpleFileOptions::default()).unwrap();
        writer.write_all(b"---\nname: Fixture\n---\nInstructions").unwrap();
    }
    writer.finish().unwrap();
    fs::create_dir(f.root.path().join("ssot/existing")).unwrap();
    fs::write(f.root.path().join("ssot/existing/user"), "preserve").unwrap();
    let mut policy: Value = serde_json::from_slice(&fs::read(&f.policy).unwrap()).unwrap();
    policy["hosts"][0]["skill_install_files"]["bundle"] = json!(archive);
    fs::write(&f.policy, policy.to_string()).unwrap();
    let mut body = request();
    body["source"] = json!({"kind":"archive","fileId":"bundle"});
    let (status, result) = f.call(body).await;
    assert_eq!(status, StatusCode::OK);
    let items = result["results"].as_array().unwrap();
    for (directory, status) in [("alpha", "installed"), ("existing", "skipped"), (".bad", "failed")] {
        assert!(items.iter().any(|item| item["directory"] == directory && item["status"] == status));
    }
    assert_eq!(items.len(), 3);
    assert_eq!(f.state().version, 1);
    assert_eq!(fs::read_to_string(f.root.path().join("ssot/existing/user")).unwrap(), "preserve");
    assert!(!f.root.path().join("ssot/.bad").exists());
}

#[tokio::test]
async fn cp12_install_api_git_requires_enabled_saved_repository() {
    use crate::gateway::agent_apps_skills_store::SkillReposStore;
    use vw_agent_apps::skills::repos::SkillRepo;
    let f = Fixture::new(true, "local");
    let mut body = request();
    body["source"] = json!({"kind":"git","repoId":"owner:repo","selection":"skills/tool","directory":"tool"});
    assert_eq!(f.call(body.clone()).await.0, StatusCode::NOT_FOUND);
    SkillReposStore::new(f.db.clone()).unwrap().save(&project(), 0,
        SkillRepo { owner: "owner".into(), name: "repo".into(), branch: "main".into(), enabled: false }).unwrap();
    assert_eq!(f.call(body).await.0, StatusCode::CONFLICT);
    assert_eq!(f.state().version, 0);
    assert_eq!(fs::read_dir(f.root.path().join("ssot")).unwrap().count(), 0);
}
