use super::super::{GatewayClient, AgentAppsReadError};
use vw_api_external_session::{SessionCatalogQuery, SessionCatalogPage};

impl GatewayClient {
    pub async fn external_sessions_catalog(&self, query: &SessionCatalogQuery, token: &str) -> Result<SessionCatalogPage, AgentAppsReadError> {
        gateway_external_session::GatewayExternalSessionClient::new(self.transport.clone())
            .external_sessions_catalog(query, token).await
    }
}
