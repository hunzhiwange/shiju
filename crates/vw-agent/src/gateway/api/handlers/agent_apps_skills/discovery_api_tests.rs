use super::discovery_api::*;
use axum::{Extension, body::Body, http::{Request, StatusCode}};
use vw_api_types::id::TenantScope;
use vw_agent_apps::skills::repos::SkillRepo;
use tower::ServiceExt;
use vw_agent_apps::skills::discovery::{SkillDiscoveryResult, SkillRepoFailure, DiscoveryError};

#[test]
fn cp12_discovery_api_success() {
    let report = SkillDiscoveryResult {
        skills: vec![vw_agent_apps::skills::discovery::DiscoverableSkill {
            key: "owner/repo:tool".into(), name: "工具".into(), description: "说明".into(),
            directory: "tool".into(), readme_url: Some("https://github.com/owner/repo".into()),
            repo_owner: "owner".into(), repo_name: "repo".into(), repo_branch: "main".into(),
        }],
        failures: vec![SkillRepoFailure { owner: "other".into(), name: "missing".into(),
            branch: "main".into(), error: DiscoveryError::NotFound }],
    };
    let expected = serde_json::json!({"version": 7, "skills": report.skills, "failures": report.failures});
    let report: SkillDiscoveryResult = serde_json::from_value(serde_json::json!({
        "skills": expected["skills"], "failures": expected["failures"]})).unwrap();
    assert_eq!(serde_json::to_value(project_report(7, report).unwrap()).unwrap(), expected);
}

#[tokio::test]
async fn cp12_discovery_api_rejects_without_side_effects() {
    // Unknown URL fields cannot reach authorization, storage, cache or the network.
    let app = router().layer(Extension(TenantScope::new("platform-a", "company-a").unwrap()));
    let response = app.oneshot(Request::post("/agent-apps/skills/discover")
        .header("content-type", "application/json")
        .body(Body::from(r#"{"scope":{"projectId":"project-a","targetHostId":"host-a"},"repoId":null,"url":"https://example.com/archive.zip"}"#)).unwrap()).await.unwrap();
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let repos = vec![SkillRepo { owner: "owner".into(), name: "repo".into(), branch: "main".into(), enabled: false }];
    assert_eq!(select_repositories(&repos, Some("owner:repo")), Err(StatusCode::CONFLICT));
    assert_eq!(select_repositories(&repos, Some("missing:repo")), Err(StatusCode::NOT_FOUND));
    assert!(!repos[0].enabled);
}
