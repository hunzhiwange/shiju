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
    async fn call(&self, id: &str, token: &str, cancel: bool) -> (StatusCode, Value) {
        let app = super::jobs_api::router_with_policy(Some(self.policy.clone()))
            .layer(Extension(TenantScope::new("p", "c").unwrap()))
            .layer(Extension(self.db.clone()));
        let suffix = if cancel { "/cancel" } else { "" };
        let response = app.oneshot(Request::builder().method(if cancel { "POST" } else { "GET" })
            .uri(format!("/agent-apps/skills/jobs/{id}{suffix}?projectId=project&targetHostId=h"))
            .header("x-agent-apps-token", token).body(Body::empty()).unwrap()).await.unwrap();
        let status = response.status();
        let bytes = axum::body::to_bytes(response.into_body(), 16384).await.unwrap();
        (status, serde_json::from_slice(&bytes).unwrap_or(Value::Null))
    }
}
fn project() -> ProjectScope {
    ProjectScope::new(TenantScope::new("p", "c").unwrap(), "project".into()).unwrap()
}

#[tokio::test]
async fn cp12_jobs_api_success() {
    let f = Fixture::new(true, "local");
    let store = SkillInstallsStore::new(f.db.clone()).unwrap();
    let (job, ()) = store.run_file_install(&project(), "h", &f.root.path().join("ssot"), "installer", || Ok(())).unwrap();
    let (status, first) = f.call(job.task_id.as_ref(), TOKEN, false).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(first["phase"], "completed");
    for cancel in [false, true, true] {
        let (status, snapshot) = f.call(job.task_id.as_ref(), TOKEN, cancel).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(snapshot, first);
    }
}

#[tokio::test]
async fn cp12_jobs_api_rejects_without_side_effects() {
    let f = Fixture::new(true, "local");
    let store = SkillInstallsStore::new(f.db.clone()).unwrap();
    let (job, ()) = store.run_file_install(&project(), "h", &f.root.path().join("ssot"), "other-owner", || Ok(())).unwrap();
    for cancel in [false, true] {
        assert_eq!(f.call(job.task_id.as_ref(), TOKEN, cancel).await.0, StatusCode::NOT_FOUND);
        assert_eq!(f.call(job.task_id.as_ref(), "invalid", cancel).await.0, StatusCode::UNAUTHORIZED);
    }
    assert_eq!(store.job_snapshot(&project(), "h", &job.task_id).unwrap(), job);
}
