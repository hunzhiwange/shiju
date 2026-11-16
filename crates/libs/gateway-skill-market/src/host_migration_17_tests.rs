use crate::expert_fixture::install_expert_fixture;
use axum::http::StatusCode;
#[tokio::test]
async fn expert_package_install_route_writes_verified_skill_files() {
 let project = tempfile::tempdir().expect("fixture project");
 let (status, body, calls) = install_expert_fixture(project.path(), std::sync::Arc::new(crate::expert_fixture::FixtureConfig)).await;
 assert_eq!(status, StatusCode::OK, "{body}");
 let root = project.path().join(".vibewindow/skills");
 assert!(root.join("fixture--skillset--fixture-suite/SKILL.md").is_file());
 assert!(root.join("fixture--public--first-skill/SKILL.md").is_file());
 assert_eq!(calls.as_slice(), ["skill_set_artifact:fixture:fixture-suite"]);
}
