use crate::{HookCatalogEntryDto, HookMutationResponseDto, HookMutationRequestDto, HookAuditEntryDto};
use crate::GatewayDesktopSettingsClient;

impl GatewayDesktopSettingsClient {
    /// List statically audited plugin hook declarations in stable order.
    pub async fn hooks_get(&self) -> Result<Vec<HookCatalogEntryDto>, String> {
        self.transport.get_json("/v1/hooks", &[]).await
    }

    /// Trust one current hook declaration by server-verified hash.
    pub async fn hook_trust(
        &self,
        plugin_id: &str,
        hook_hash: &str,
    ) -> Result<HookMutationResponseDto, String> {
        self.transport.post_json(
            "/v1/hooks/trust",
            &[],
            &HookMutationRequestDto {
                plugin_id: plugin_id.to_string(),
                hook_hash: hook_hash.to_string(),
            },
        )
        .await
    }

    /// Revoke one current hook declaration by server-verified hash.
    pub async fn hook_revoke(
        &self,
        plugin_id: &str,
        hook_hash: &str,
    ) -> Result<HookMutationResponseDto, String> {
        self.transport.post_json(
            "/v1/hooks/revoke",
            &[],
            &HookMutationRequestDto {
                plugin_id: plugin_id.to_string(),
                hook_hash: hook_hash.to_string(),
            },
        )
        .await
    }

    /// Read bounded command audit records containing only safe fields.
    pub async fn hook_audit_get(&self) -> Result<Vec<HookAuditEntryDto>, String> {
        self.transport.get_json("/v1/hooks/audit", &[]).await
    }
}
