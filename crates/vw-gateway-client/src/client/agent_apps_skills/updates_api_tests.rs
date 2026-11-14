use crate::client::{test_support, agent_apps_skills::SkillUpdateError};
use serde_json::json;
use vw_api_types::agent_apps_skills::{updates_api::*, repos_api::SkillReposQuery};

#[tokio::test]
async fn cp12_updates_api_success() {
    let scope = SkillReposQuery { project_id: "project".into(), target_host_id: "host".into() };
    let report = json!({"scope":scope,"version":3,"updates":[{"directory":"tool","name":"Tool",
        "current_hash":"old","remote_hash":"new"}],"failures":[],"skillFailures":[],
        "withoutRepository":["local"],"unchanged":[]});
    let task = json!({"scope":scope,"taskId":"task-1","phase":"completed","directory":"tool",
        "requiresReconciliation":false,"version":4,"contentHash":"new"});
    let server = test_support::server(vec![(200,report.clone()),(200,task.clone()),(200,task.clone())]);
    let client = server.client();
    let response = client.agent_apps_skills_check_updates(&CheckSkillUpdatesRequest { scope: scope.clone() }, "token").await.unwrap();
    assert_eq!(serde_json::to_value(response).unwrap(), report);
    let sent = server.take_request();
    assert_eq!(sent.path, "/v1/agent-apps/skills/check-updates");
    assert_eq!(sent.header("x-agent-apps-token"), Some("token"));
    let request = UpdateSkillRequest { scope: scope.clone(), expected_version: 3, expected_hash: "old".into() };
    let response = client.agent_apps_skills_update("tool", &request, "token").await.unwrap();
    assert_eq!(serde_json::to_value(response).unwrap(), task);
    let sent = server.take_request();
    assert_eq!(sent.method, "POST");
    assert_eq!(sent.path, "/v1/agent-apps/skills/tool/update");
    assert_eq!(sent.body, serde_json::to_value(request).unwrap());
    let response = client.agent_apps_skills_update_task("task-1", &scope, "token").await.unwrap();
    assert_eq!(serde_json::to_value(response).unwrap(), task);
    assert_eq!(server.take_request().method, "GET");
    server.join();
}

#[tokio::test]
async fn cp12_updates_api_rejects_without_side_effects() {
    let scope = SkillReposQuery { project_id: "project".into(), target_host_id: "host".into() };
    let server = test_support::server(vec![(403,json!({"private":"must not surface"}))]);
    let client = server.client();
    let request = UpdateSkillRequest { scope, expected_version: 0, expected_hash: "old".into() };
    assert_eq!(client.agent_apps_skills_update("../escape", &request, "token").await.unwrap_err(), SkillUpdateError::InvalidId);
    assert_eq!(client.agent_apps_skills_update("tool", &request, "token").await.unwrap_err(), SkillUpdateError::Http { status: 403 });
    server.take_request();
    server.join();
}
