use crate::client::{test_support, agent_apps_skills::SkillBackupError};
use serde_json::json;
use vw_api_types::agent_apps_skills::{backups_api::*, repos_api::SkillReposQuery};

#[tokio::test]
async fn cp12_backups_api_success() {
    let scope = SkillReposQuery { project_id: "project".into(), target_host_id: "host".into() };
    let result = json!({"scope":scope,"backupId":"skill-uninstall-1","directory":"tool","version":3});
    let server = test_support::server(vec![(200, result.clone())]);
    let request = RestoreSkillBackupRequest { scope, expected_version: 2, apps: vec![] };
    let response = server.client().agent_apps_skills_backup_restore("skill-uninstall-1", &request, "token").await.unwrap();
    assert_eq!(serde_json::to_value(response).unwrap(), result);
    let sent = server.take_request();
    assert_eq!(sent.method, "POST");
    assert_eq!(sent.path, "/v1/agent-apps/skills/backups/skill-uninstall-1/restore");
    assert_eq!(sent.body, serde_json::to_value(request).unwrap());
    server.join();
}
#[tokio::test]
async fn cp12_backups_api_rejects_without_side_effects() {
    let server = test_support::server(vec![(403, json!({}))]);
    let request = DeleteSkillBackupRequest { scope: SkillReposQuery {
        project_id: "project".into(), target_host_id: "host".into() } };
    let client = server.client();
    assert_eq!(client.agent_apps_skills_backup_delete("../backup", &request, "token").await.unwrap_err(), SkillBackupError::InvalidId);
    assert_eq!(client.agent_apps_skills_backup_delete("skill-uninstall-1", &request, "token").await.unwrap_err(), SkillBackupError::Http { status: 403 });
    server.take_request();
    server.join();
}
