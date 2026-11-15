use super::{GatewayClient, HermesMemoryScope, HermesMemoryError};
use vw_api_types::hermes_memory::*;

impl GatewayClient {
    pub async fn hermes_memory(&self, scope: &HermesMemoryScope, kind: HermesMemoryKind, token: &str) -> Result<HermesMemoryResponse, HermesMemoryError> {
        gateway_hermes_memory::GatewayHermesMemoryClient::new(self.transport.clone())
            .hermes_memory(scope, kind, token).await
    }
    pub async fn set_hermes_memory(&self, scope: &HermesMemoryScope, kind: HermesMemoryKind, content: String, token: &str) -> Result<HermesWriteOutcome, HermesMemoryError> {
        gateway_hermes_memory::GatewayHermesMemoryClient::new(self.transport.clone())
            .set_hermes_memory(scope, kind, content, token).await
    }
    pub async fn hermes_memory_limits(&self, scope: &HermesMemoryScope, token: &str) -> Result<HermesMemoryLimits, HermesMemoryError> {
        gateway_hermes_memory::GatewayHermesMemoryClient::new(self.transport.clone())
            .hermes_memory_limits(scope, token).await
    }
    pub async fn set_hermes_memory_enabled(&self, scope: &HermesMemoryScope, kind: HermesMemoryKind, enabled: bool, token: &str) -> Result<HermesWriteOutcome, HermesMemoryError> {
        gateway_hermes_memory::GatewayHermesMemoryClient::new(self.transport.clone())
            .set_hermes_memory_enabled(scope, kind, enabled, token).await
    }
    pub async fn hermes_model(&self, scope: &HermesMemoryScope, token: &str) -> Result<Option<HermesModelSnapshot>, HermesMemoryError> {
        gateway_hermes_memory::GatewayHermesMemoryClient::new(self.transport.clone())
            .hermes_model(scope, token).await
    }
}
