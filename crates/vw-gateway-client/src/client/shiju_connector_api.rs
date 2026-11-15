//! 识矩连接器兼容门面。
use super::{GatewayClient, ShijuCredential};
pub use gateway_shiju_connector::*;

impl GatewayClient {
    pub async fn shiju_connectors(
        &self,
        credential: &ShijuCredential,
    ) -> Result<Vec<ShijuConnector>, String> {
        gateway_shiju_connector::GatewayShijuConnectorClient::new(self.transport.clone())
            .shiju_connectors(credential).await
    }
    pub async fn shiju_connector_save(
        &self,
        credential: &ShijuCredential,
        request: &ShijuConnectorSaveRequest,
    ) -> Result<ShijuConnectorBinding, String> {
        gateway_shiju_connector::GatewayShijuConnectorClient::new(self.transport.clone())
            .shiju_connector_save(credential, request).await
    }
    pub async fn shiju_connector_authorize(
        &self,
        credential: &ShijuCredential,
        company_platform_id: u64,
    ) -> Result<ShijuConnectorChallenge, String> {
        gateway_shiju_connector::GatewayShijuConnectorClient::new(self.transport.clone())
            .shiju_connector_authorize(credential, company_platform_id).await
    }
    pub async fn shiju_connector_poll(
        &self,
        credential: &ShijuCredential,
        flow_id: &str,
    ) -> Result<ShijuConnectorPoll, String> {
        gateway_shiju_connector::GatewayShijuConnectorClient::new(self.transport.clone())
            .shiju_connector_poll(credential, flow_id).await
    }
}
