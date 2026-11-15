//! 交互领域客户端兼容转发。

use super::GatewayClient;
use serde_json::Value;

impl GatewayClient {
    pub async fn auth_set(&self, provider_id: &str, info: &Value) -> Result<bool, String> {
        gateway_interaction::GatewayInteractionClient::new(self.transport().clone())
            .auth_set(provider_id, info).await
    }

    pub async fn auth_remove(&self, provider_id: &str) -> Result<bool, String> {
        gateway_interaction::GatewayInteractionClient::new(self.transport().clone())
            .auth_remove(provider_id).await
    }

    pub async fn global_dispose(&self) -> Result<bool, String> {
        gateway_interaction::GatewayInteractionClient::new(self.transport().clone())
            .global_dispose().await
    }

    pub async fn instance_dispose(&self, directory: Option<&str>) -> Result<bool, String> {
        gateway_interaction::GatewayInteractionClient::new(self.transport().clone())
            .instance_dispose(directory).await
    }
}
