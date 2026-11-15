use super::GatewayClient;
use vw_api_types::hermes_dashboard::*;
pub use gateway_hermes_memory::HermesDashboardError;

impl GatewayClient {
    pub async fn hermes_dashboard_probe(&self, scope: &DashboardScope, token: &str) -> Result<DashboardProbe, HermesDashboardError> {
        gateway_hermes_memory::GatewayHermesMemoryClient::new(self.transport.clone())
            .hermes_dashboard_probe(scope, token).await
    }
    pub async fn hermes_dashboard_start(&self, scope: &DashboardScope, token: &str) -> Result<DashboardOperation, HermesDashboardError> {
        gateway_hermes_memory::GatewayHermesMemoryClient::new(self.transport.clone())
            .hermes_dashboard_start(scope, token).await
    }
    pub async fn hermes_dashboard_operation(&self, scope: &DashboardScope, id: &str, token: &str) -> Result<DashboardOperation, HermesDashboardError> {
        gateway_hermes_memory::GatewayHermesMemoryClient::new(self.transport.clone())
            .hermes_dashboard_operation(scope, id, token).await
    }
    pub async fn hermes_dashboard_cancel(&self, scope: &DashboardScope, id: &str, token: &str) -> Result<DashboardOperation, HermesDashboardError> {
        gateway_hermes_memory::GatewayHermesMemoryClient::new(self.transport.clone())
            .hermes_dashboard_cancel(scope, id, token).await
    }
}
