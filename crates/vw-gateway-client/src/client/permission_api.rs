//! 交互领域客户端兼容转发。

use super::GatewayClient;
pub use gateway_interaction::{PendingPermissionReplyDto, PendingPermissionRequestDto, PendingPermissionToolDto};

impl GatewayClient {
    pub async fn permission_list(
        &self,
        directory: Option<&str>,
    ) -> Result<Vec<PendingPermissionRequestDto>, String> {
        gateway_interaction::GatewayInteractionClient::new(self.transport().clone())
            .permission_list(directory).await
    }

    pub async fn permission_reply(
        &self,
        request_id: &str,
        reply: PendingPermissionReplyDto,
        directory: Option<&str>,
        message: Option<String>,
    ) -> Result<bool, String> {
        gateway_interaction::GatewayInteractionClient::new(self.transport().clone())
            .permission_reply(request_id, reply, directory, message).await
    }
}
