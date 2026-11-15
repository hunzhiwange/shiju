use crate::{GatewayExternalSessionClient, AgentAppsReadError};
use vw_api_external_session::{SessionCatalogQuery, SessionCatalogPage};
impl GatewayExternalSessionClient {
    /// Query only the selected gateway and explicitly authorized host; never discover local history.
    pub async fn external_sessions_catalog(&self, query: &SessionCatalogQuery, token: &str) -> Result<SessionCatalogPage, AgentAppsReadError> {
        let value = serde_json::to_value(query).map_err(|e| AgentAppsReadError::Decode(e.to_string()))?;
        let fields = value.as_object().ok_or_else(|| AgentAppsReadError::Decode("invalid query".into()))?;
        let query: Vec<_> = fields.iter().filter(|(_, value)| !value.is_null()).map(|(key, value)| {
            (key.as_str(), value.as_str().map(str::to_owned).unwrap_or_else(|| value.to_string()))
        }).collect();
        gateway_app_read::GatewayAppReadClient::new(self.transport.clone()).agent_apps_read_request("/v1/external-sessions/catalog", &query, Some(token)).await
    }
}
