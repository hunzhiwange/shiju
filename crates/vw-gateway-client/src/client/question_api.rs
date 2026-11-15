//! 交互领域客户端兼容转发。

use super::GatewayClient;
use question_types as question;

impl GatewayClient {
    pub async fn question_list(&self) -> Result<Vec<question::Request>, String> {
        gateway_interaction::GatewayInteractionClient::new(self.transport().clone())
            .question_list().await
    }

    pub async fn question_reply(
        &self,
        request_id: &str,
        answers: Vec<Vec<String>>,
    ) -> Result<bool, String> {
        gateway_interaction::GatewayInteractionClient::new(self.transport().clone())
            .question_reply(request_id, answers).await
    }

    pub async fn question_reject(&self, request_id: &str) -> Result<bool, String> {
        gateway_interaction::GatewayInteractionClient::new(self.transport().clone())
            .question_reject(request_id).await
    }
}
