use crate::client::{test_support, agent_apps_skills::SkillJobError};
use serde_json::json;
use vw_api_types::{agent_apps_skills::repos_api::SkillReposQuery, id::TaskId};

#[tokio::test]
async fn cp12_jobs_api_success() {
    let scope = SkillReposQuery { project_id: "project".into(), target_host_id: "host".into() };
    let id = TaskId::from("task-1");
    let body = json!({"scope":scope,"taskId":"task-1","phase":"interrupted",
        "installedDirectory":null,"requiresReconciliation":true});
    for cancel in [false, true] {
        let server = test_support::server(vec![(200, body.clone())]);
        let client = server.client();
        let response = if cancel { client.agent_apps_skills_cancel_job(&scope, &id, "test-token").await }
            else { client.agent_apps_skills_job(&scope, &id, "test-token").await }.unwrap();
        assert_eq!(serde_json::to_value(response).unwrap(), body);
        let sent = server.take_request();
        assert_eq!(sent.method, if cancel { "POST" } else { "GET" });
        assert_eq!(sent.path, format!("/v1/agent-apps/skills/jobs/task-1{}?projectId=project&targetHostId=host",
            if cancel { "/cancel" } else { "" }));
        assert_eq!(sent.header("x-agent-apps-token"), Some("test-token"));
        server.join();
    }
}

#[tokio::test]
async fn cp12_jobs_api_rejects_without_side_effects() {
    let scope = SkillReposQuery { project_id: "project".into(), target_host_id: "host".into() };
    let server = test_support::server(vec![(403, json!({"secret":"never returned"}))]);
    assert_eq!(server.client().agent_apps_skills_cancel_job(&scope, &TaskId::from("task-1"), "test-token").await.unwrap_err(),
        SkillJobError::Http { status: 403 });
    server.take_request();
    server.join();
}
