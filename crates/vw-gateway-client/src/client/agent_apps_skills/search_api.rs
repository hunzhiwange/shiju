use super::super::GatewayClient;
use vw_api_app_skills::search_api::*;
pub use gateway_app_skills::{MarketSearchError};

impl GatewayClient {
    pub async fn agent_apps_market_search(&self, query: &MarketSearchQuery, token: &str) -> Result<MarketSearchResponse, MarketSearchError> {
        gateway_app_skills::GatewayAppSkillsClient::new(self.transport.clone())
            .agent_apps_market_search(query, token).await
    }
}
