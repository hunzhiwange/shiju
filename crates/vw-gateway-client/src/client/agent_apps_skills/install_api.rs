use super::super::GatewayClient;
use vw_api_app_skills::install_api::*;
pub use gateway_app_skills::{SkillInstallError};

impl GatewayClient {
    pub async fn agent_apps_skills_install(&self, query: &InstallSkillRequest, token: &str) -> Result<InstallSkillResponse, SkillInstallError> {
        gateway_app_skills::GatewayAppSkillsClient::new(self.transport.clone())
            .agent_apps_skills_install(query, token).await
    }
}
