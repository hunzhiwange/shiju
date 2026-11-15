use serde::de::DeserializeOwned;
use serde_json::Value;
use vw_api_transport::id::TenantScope;

use super::GatewayClient;

impl GatewayClient {
    pub async fn agent_profiles(
        &self,
        directory: Option<&str>,
        include_disabled: bool,
    ) -> Result<Value, String> {
        gateway_config::GatewayConfigClient::new(self.transport.clone())
            .agent_profiles(directory, include_disabled).await
    }

    pub async fn agent_profile_copy(
        &self,
        directory: Option<&str>,
        key: &str,
    ) -> Result<Value, String> {
        gateway_config::GatewayConfigClient::new(self.transport.clone())
            .agent_profile_copy(directory, key).await
    }

    pub async fn config_get(&self, directory: Option<&str>) -> Result<Value, String> {
        gateway_config::GatewayConfigClient::new(self.transport.clone())
            .config_get(directory).await
    }

    pub async fn config_patch(
        &self,
        directory: Option<&str>,
        patch: &Value,
    ) -> Result<Value, String> {
        gateway_config::GatewayConfigClient::new(self.transport.clone())
            .config_patch(directory, patch).await
    }

    pub async fn config_providers(&self, directory: Option<&str>) -> Result<Value, String> {
        gateway_config::GatewayConfigClient::new(self.transport.clone())
            .config_providers(directory).await
    }

    pub async fn dashboard_config_get(&self) -> Result<Value, String> {
        gateway_config::GatewayConfigClient::new(self.transport.clone())
            .dashboard_config_get().await
    }

    pub async fn dashboard_config_put(&self, config: &Value) -> Result<Value, String> {
        gateway_config::GatewayConfigClient::new(self.transport.clone())
            .dashboard_config_put(config).await
    }

    pub async fn global_config_get(&self) -> Result<Value, String> {
        gateway_config::GatewayConfigClient::new(self.transport.clone())
            .global_config_get().await
    }

    pub async fn global_config_patch(&self, patch: &Value) -> Result<(), String> {
        gateway_config::GatewayConfigClient::new(self.transport.clone())
            .global_config_patch(patch).await
    }

    pub async fn tenant_global_config_get(&self, tenant: &TenantScope) -> Result<Value, String> {
        gateway_config::GatewayConfigClient::new(self.transport.clone())
            .tenant_global_config_get(tenant).await
    }

    pub async fn tenant_global_config_patch(
        &self,
        tenant: &TenantScope,
        patch: &Value,
    ) -> Result<(), String> {
        gateway_config::GatewayConfigClient::new(self.transport.clone())
            .tenant_global_config_patch(tenant, patch).await
    }

    pub async fn global_acp_installed_get(
        &self,
    ) -> Result<std::collections::BTreeMap<String, bool>, String> {
        gateway_config::GatewayConfigClient::new(self.transport.clone())
            .global_acp_installed_get().await
    }

    pub async fn global_acp_defaults_get<T>(&self) -> Result<T, String>
    where
        T: DeserializeOwned,
    {
        gateway_config::GatewayConfigClient::new(self.transport.clone())
            .global_acp_defaults_get::<T>().await
    }

    pub async fn global_acp_config_get<T>(&self) -> Result<T, String>
    where
        T: DeserializeOwned,
    {
        gateway_config::GatewayConfigClient::new(self.transport.clone())
            .global_acp_config_get::<T>().await
    }

    pub(crate) async fn global_config_get_path(
        &self,
        path: &[&str],
    ) -> Result<Option<Value>, String> {
        gateway_config::GatewayConfigClient::new(self.transport.clone())
            .global_config_get_path(path).await
    }

    pub(crate) async fn global_config_patch_path(
        &self,
        path: &[&str],
        value: Value,
    ) -> Result<(), String> {
        gateway_config::GatewayConfigClient::new(self.transport.clone())
            .global_config_patch_path(path, value).await
    }
}

#[cfg(test)]
#[path = "config_api_tests.rs"]
mod config_api_tests;
