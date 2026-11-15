use super::super::GatewayClient;
use vw_api_app_skills::import_api::*;
pub use gateway_app_skills::{SkillImportError};

impl GatewayClient {
    pub async fn agent_apps_skills_import(&self, query: &ImportSkillRequest, token: &str) -> Result<ImportSkillResponse, SkillImportError> {
        gateway_app_skills::GatewayAppSkillsClient::new(self.transport.clone())
            .agent_apps_skills_import(query, token).await
    }
    pub async fn agent_apps_skills_unmanaged(&self, query: &UnmanagedSkillsQuery, token: &str) -> Result<UnmanagedSkillsResponse, SkillImportError> {
        gateway_app_skills::GatewayAppSkillsClient::new(self.transport.clone())
            .agent_apps_skills_unmanaged(query, token).await
    }
}
