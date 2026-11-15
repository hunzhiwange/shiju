use super::super::GatewayClient;
use vw_api_app_skills::{storage_api::*, repos_api::SkillReposQuery};
pub use gateway_app_skills::{SkillStorageError};

impl GatewayClient {
    pub async fn agent_apps_skills_storage(&self, scope: &SkillReposQuery, token: &str) -> Result<SkillStorageResponse, SkillStorageError> {
        gateway_app_skills::GatewayAppSkillsClient::new(self.transport.clone())
            .agent_apps_skills_storage(scope, token).await
    }
    pub async fn agent_apps_skills_storage_patch(&self, request: &SkillStoragePatch, token: &str) -> Result<SkillStorageResponse, SkillStorageError> {
        gateway_app_skills::GatewayAppSkillsClient::new(self.transport.clone())
            .agent_apps_skills_storage_patch(request, token).await
    }
    pub async fn agent_apps_skills_storage_move(&self, request: &SkillStorageMoveRequest, token: &str) -> Result<SkillStorageMoveResponse, SkillStorageError> {
        gateway_app_skills::GatewayAppSkillsClient::new(self.transport.clone())
            .agent_apps_skills_storage_move(request, token).await
    }
}
