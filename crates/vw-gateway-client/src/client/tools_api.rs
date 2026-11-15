//! 交互领域客户端兼容转发。

use super::GatewayClient;

impl GatewayClient {
    pub async fn tools_list(&self) -> Result<Vec<String>, String> {
        gateway_interaction::GatewayInteractionClient::new(self.transport().clone())
            .tools_list().await
    }
}
