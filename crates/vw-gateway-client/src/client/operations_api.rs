//! 交互领域客户端兼容转发。

use super::GatewayClient;
use serde_json::Value;

impl GatewayClient {
    pub async fn integrations_get(&self) -> Result<Value, String> {
        gateway_interaction::GatewayInteractionClient::new(self.transport().clone())
            .integrations_get().await
    }

    pub async fn integration_settings_get(&self) -> Result<Value, String> {
        gateway_interaction::GatewayInteractionClient::new(self.transport().clone())
            .integration_settings_get().await
    }

    pub async fn integration_credentials_put(
        &self,
        integration_id: &str,
        body: &Value,
    ) -> Result<Value, String> {
        gateway_interaction::GatewayInteractionClient::new(self.transport().clone())
            .integration_credentials_put(integration_id, body).await
    }

    pub async fn memory_query_get(
        &self,
        query: Option<&str>,
        category: Option<&str>,
    ) -> Result<Value, String> {
        gateway_interaction::GatewayInteractionClient::new(self.transport().clone())
            .memory_query_get(query, category).await
    }

    pub async fn memory_store(&self, body: &Value) -> Result<Value, String> {
        gateway_interaction::GatewayInteractionClient::new(self.transport().clone())
            .memory_store(body).await
    }

    pub async fn memory_delete(&self, key: &str) -> Result<Value, String> {
        gateway_interaction::GatewayInteractionClient::new(self.transport().clone())
            .memory_delete(key).await
    }

    pub async fn doctor_get(&self) -> Result<Value, String> {
        gateway_interaction::GatewayInteractionClient::new(self.transport().clone())
            .doctor_get().await
    }

    pub async fn doctor_post(&self) -> Result<Value, String> {
        gateway_interaction::GatewayInteractionClient::new(self.transport().clone())
            .doctor_post().await
    }
}
