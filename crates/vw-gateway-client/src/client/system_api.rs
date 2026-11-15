//! 系统访问兼容门面。
use serde_json::Value;
use super::GatewayClient;
use gateway_system::GatewaySystemClient;

impl GatewayClient {
    pub async fn system_health(&self) -> Result<Value, String> {
        GatewaySystemClient::new(self.transport.clone()).system_health().await
    }
    pub async fn system_metrics(&self) -> Result<String, String> {
        GatewaySystemClient::new(self.transport.clone()).system_metrics().await
    }
    pub async fn system_status(&self) -> Result<Value, String> {
        GatewaySystemClient::new(self.transport.clone()).system_status().await
    }
    pub async fn system_tools(&self) -> Result<Value, String> {
        GatewaySystemClient::new(self.transport.clone()).system_tools().await
    }
    pub async fn system_mcp_inspect(&self) -> Result<Value, String> {
        GatewaySystemClient::new(self.transport.clone()).system_mcp_inspect().await
    }
    pub async fn system_api_health(&self) -> Result<Value, String> {
        GatewaySystemClient::new(self.transport.clone()).system_api_health().await
    }
    pub async fn system_global_health(&self) -> Result<Value, String> {
        GatewaySystemClient::new(self.transport.clone()).system_global_health().await
    }
    pub async fn system_models(&self) -> Result<Value, String> {
        GatewaySystemClient::new(self.transport.clone()).system_models().await
    }
    pub async fn system_path(&self, directory: Option<&str>) -> Result<Value, String> {
        GatewaySystemClient::new(self.transport.clone()).system_path(directory).await
    }
    pub async fn system_vcs(&self, directory: Option<&str>) -> Result<Value, String> {
        GatewaySystemClient::new(self.transport.clone()).system_vcs(directory).await
    }
    pub async fn system_commands(&self, directory: Option<&str>) -> Result<Value, String> {
        GatewaySystemClient::new(self.transport.clone()).system_commands(directory).await
    }
    pub async fn system_agents(&self, directory: Option<&str>) -> Result<Value, String> {
        GatewaySystemClient::new(self.transport.clone()).system_agents(directory).await
    }
    pub async fn system_skills(&self, directory: Option<&str>) -> Result<Value, String> {
        GatewaySystemClient::new(self.transport.clone()).system_skills(directory).await
    }
    pub async fn system_cli_tools(&self) -> Result<Value, String> {
        GatewaySystemClient::new(self.transport.clone()).system_cli_tools().await
    }
}
