use super::super::GatewayClient;
use vw_api_app_skills::{updates_api::*, repos_api::SkillReposQuery};
pub use gateway_app_skills::{SkillUpdateError};

impl GatewayClient {
    pub async fn agent_apps_skills_check_updates(&self, request: &CheckSkillUpdatesRequest, token: &str) -> Result<CheckSkillUpdatesResponse, SkillUpdateError> {
        gateway_app_skills::GatewayAppSkillsClient::new(self.transport.clone())
            .agent_apps_skills_check_updates(request, token).await
    }
    pub async fn agent_apps_skills_update(&self, id: &str, request: &UpdateSkillRequest, token: &str) -> Result<SkillUpdateTask, SkillUpdateError> {
        gateway_app_skills::GatewayAppSkillsClient::new(self.transport.clone())
            .agent_apps_skills_update(id, request, token).await
    }
    pub async fn agent_apps_skills_update_task(&self, id: &str, scope: &SkillReposQuery, token: &str) -> Result<SkillUpdateTask, SkillUpdateError> {
        gateway_app_skills::GatewayAppSkillsClient::new(self.transport.clone())
            .agent_apps_skills_update_task(id, scope, token).await
    }
}
