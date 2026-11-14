use crate::client::test_support;
use serde_json::json;
use vw_api_types::agent_apps_skills::{discovery_api::*, repos_api::SkillReposQuery};

#[tokio::test]
async fn cp12_discovery_api_success() {
    let body = json!({"version":7,"skills":[{
        "key":"owner/repo:tool","name":"工具","description":"说明","directory":"tool",
        "repoOwner":"owner","repoName":"repo","repoBranch":"main","readmeUrl":null}],
        "failures":[{"owner":"other","name":"missing","branch":"main","error":{"code":"NOT_FOUND"}}]});
    let server = test_support::server(vec![(200, body.clone())]);
    let request = DiscoverSkillsRequest { scope: SkillReposQuery {
        project_id: "project-a".into(), target_host_id: "host-a".into() },
        repo_id: None, force_refresh: true };
    let result = server.client().agent_apps_skills_discover(&request, "test-token").await.unwrap();
    assert_eq!(serde_json::to_value(result).unwrap(), body);
    let sent = server.take_request();
    assert_eq!(sent.method, "POST");
    assert_eq!(sent.header("x-agent-apps-token"), Some("test-token"));
    server.join();
}
