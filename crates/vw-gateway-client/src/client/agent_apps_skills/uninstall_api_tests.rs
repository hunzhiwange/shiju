use crate::client::{test_support, agent_apps_skills::SkillUninstallError};
use serde_json::json;
use vw_api_types::agent_apps_skills::{uninstall_api::*, repos_api::SkillReposQuery};

#[tokio::test]
async fn cp12_uninstall_api_success() {
    let scope = SkillReposQuery { project_id: "project".into(), target_host_id: "host".into() };
    let result = json!({"scope":scope,"version":2,"backupId":"backup-1",
        "backupPath":"/host/backups/backup-1","preservedPaths":["/host/app/tool"],"cleanupIncomplete":true});
    let server = test_support::server(vec![(200,result.clone())]);
    let request = UninstallSkillRequest { scope, expected_version: 1 };
    let response = server.client().agent_apps_skills_uninstall("tool", &request, "token").await.unwrap();
    assert_eq!(serde_json::to_value(response).unwrap(), result);
    let sent = server.take_request();
    assert_eq!(sent.method, "DELETE");
    assert_eq!(sent.path, "/v1/agent-apps/skills/tool");
    assert_eq!(sent.header("x-agent-apps-token"), Some("token"));
    assert_eq!(sent.body, serde_json::to_value(request).unwrap());
    server.join();
}

#[tokio::test]
async fn cp12_uninstall_api_rejects_without_side_effects() {
    let server = test_support::server(vec![(403,json!({"private":"hidden"})), (404,json!({}))]);
    let client = server.client();
    let request = UninstallSkillRequest { scope: SkillReposQuery {
        project_id: "project".into(), target_host_id: "host".into() }, expected_version: 1 };
    assert_eq!(client.agent_apps_skills_uninstall("../tool", &request, "token").await.unwrap_err(), SkillUninstallError::InvalidId);
    assert_eq!(client.agent_apps_skills_uninstall("tool", &request, "token").await.unwrap_err(), SkillUninstallError::Http { status: 403 });
    server.take_request();
    assert_eq!(client.agent_apps_skills_uninstall("tool", &request, "token").await.unwrap_err(), SkillUninstallError::Http { status: 404 });
    server.take_request();
    server.join();
}
