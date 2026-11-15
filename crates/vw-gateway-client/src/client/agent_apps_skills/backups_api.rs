use super::super::GatewayClient;
use vw_api_app_skills::{backups_api::*, repos_api::SkillReposQuery};
pub use gateway_app_skills::{SkillBackupError};

impl GatewayClient {
    pub async fn agent_apps_skills_backups(&self, scope: &SkillReposQuery, token: &str) -> Result<SkillBackupsResponse, SkillBackupError> {
        gateway_app_skills::GatewayAppSkillsClient::new(self.transport.clone())
            .agent_apps_skills_backups(scope, token).await
    }
    pub async fn agent_apps_skills_backup_delete(&self, id: &str, request: &DeleteSkillBackupRequest, token: &str) -> Result<DeleteSkillBackupResponse, SkillBackupError> {
        gateway_app_skills::GatewayAppSkillsClient::new(self.transport.clone())
            .agent_apps_skills_backup_delete(id, request, token).await
    }
    pub async fn agent_apps_skills_backup_restore(&self, id: &str, request: &RestoreSkillBackupRequest, token: &str) -> Result<RestoreSkillBackupResponse, SkillBackupError> {
        gateway_app_skills::GatewayAppSkillsClient::new(self.transport.clone())
            .agent_apps_skills_backup_restore(id, request, token).await
    }
}
