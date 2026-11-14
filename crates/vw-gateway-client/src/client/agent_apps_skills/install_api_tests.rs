use crate::client::{test_support, agent_apps_skills::SkillInstallError};
use serde_json::json;
use vw_api_types::agent_apps_skills::{install_api::*, repos_api::SkillReposQuery};

#[tokio::test]
async fn cp12_install_api_success() {
    let scope = SkillReposQuery { project_id: "project".into(), target_host_id: "host".into() };
    for source in [SkillInstallSource::Local { file_id: "selected".into(), directory: "tool".into() },
        SkillInstallSource::Archive { file_id: "upload".into() },
        SkillInstallSource::Git { repo_id: "owner:repo".into(), selection: "skills/tool".into(), directory: "tool".into() }] {
        let body = json!({"scope":scope,"taskId":"task-1","results":[
            {"targetHostId":"host","directory":"tool","status":"pending"},
            {"targetHostId":"host","directory":"other","status":"failed"}]});
        let server = test_support::server(vec![(200, body.clone())]);
        let request = InstallSkillRequest { scope: scope.clone(), expected_version: 3, source };
        let response = server.client().agent_apps_skills_install(&request, "test-token").await.unwrap();
        assert_eq!(serde_json::to_value(response).unwrap(), body);
        let sent = server.take_request();
        assert_eq!(sent.method, "POST");
        assert_eq!(sent.path, "/v1/agent-apps/skills/install");
        assert_eq!(sent.header("x-agent-apps-token"), Some("test-token"));
        assert_eq!(sent.body, serde_json::to_value(request).unwrap());
        server.join();
    }
}

#[tokio::test]
async fn cp12_install_api_rejects_without_side_effects() {
    let server = test_support::server(vec![(403, json!({"private":"must not surface"}))]);
    let request = InstallSkillRequest {
        scope: SkillReposQuery { project_id: "project".into(), target_host_id: "host".into() },
        expected_version: 0, source: SkillInstallSource::Archive { file_id: "upload".into() },
    };
    assert!(matches!(server.client().agent_apps_skills_install(&request, "test-token").await,
        Err(SkillInstallError::Http { status: 403 })));
    server.take_request();
    server.join();
}
