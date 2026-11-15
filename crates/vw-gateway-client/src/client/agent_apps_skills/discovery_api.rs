use super::super::GatewayClient;
use vw_api_app_skills::discovery_api::*;
pub use gateway_app_skills::{SkillDiscoveryError};

impl GatewayClient {
    pub async fn agent_apps_skills_discover(&self, query: &DiscoverSkillsRequest, token: &str) -> Result<DiscoverSkillsResponse, SkillDiscoveryError> {
        gateway_app_skills::GatewayAppSkillsClient::new(self.transport.clone())
            .agent_apps_skills_discover(query, token).await
    }
}
