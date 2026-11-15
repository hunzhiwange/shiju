use super::super::GatewayClient;
use vw_api_app_skills::{inventory_api::*, repos_api::SkillReposQuery};
pub use gateway_app_skills::{SkillInventoryError};

impl GatewayClient {
    pub async fn agent_apps_skills_inventory(&self, scope: &SkillReposQuery, token: &str) -> Result<SkillInventoryResponse, SkillInventoryError> {
        gateway_app_skills::GatewayAppSkillsClient::new(self.transport.clone())
            .agent_apps_skills_inventory(scope, token).await
    }
    pub async fn agent_apps_skills_detail(&self, scope: &SkillReposQuery, id: &str, token: &str) -> Result<InstalledSkill, SkillInventoryError> {
        gateway_app_skills::GatewayAppSkillsClient::new(self.transport.clone())
            .agent_apps_skills_detail(scope, id, token).await
    }
}
