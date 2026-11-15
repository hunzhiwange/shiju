use super::super::GatewayClient;
use vw_api_app_skills::bindings_api::*;
pub use gateway_app_skills::{SkillBindingError};

impl GatewayClient {
    pub async fn agent_apps_skills_bindings(&self, id: &str, query: &SkillBindingsRequest, token: &str) -> Result<SkillBindingsResponse, SkillBindingError> {
        gateway_app_skills::GatewayAppSkillsClient::new(self.transport.clone())
            .agent_apps_skills_bindings(id, query, token).await
    }
    pub async fn agent_apps_skills_resync(&self, query: &SkillResyncRequest, token: &str) -> Result<SkillBindingsResponse, SkillBindingError> {
        gateway_app_skills::GatewayAppSkillsClient::new(self.transport.clone())
            .agent_apps_skills_resync(query, token).await
    }
    pub async fn agent_apps_skill_binding_targets(&self, scope: &vw_api_app_skills::repos_api::SkillReposQuery,
        change: Option<&SkillBindingPreferences>, token: &str) -> Result<SkillBindingSnapshot, SkillBindingError> {
        gateway_app_skills::GatewayAppSkillsClient::new(self.transport.clone())
            .agent_apps_skill_binding_targets(scope, change, token).await
    }
}
