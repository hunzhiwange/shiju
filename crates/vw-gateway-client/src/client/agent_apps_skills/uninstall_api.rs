use super::super::GatewayClient;
use vw_api_app_skills::uninstall_api::*;
pub use gateway_app_skills::{SkillUninstallError};

impl GatewayClient {
    pub async fn agent_apps_skills_uninstall(&self, id: &str, request: &UninstallSkillRequest, token: &str) -> Result<UninstallSkillResponse, SkillUninstallError> {
        gateway_app_skills::GatewayAppSkillsClient::new(self.transport.clone())
            .agent_apps_skills_uninstall(id, request, token).await
    }
}
