//! 模型提供商兼容转发。
use super::GatewayClient;
pub use gateway_model_provider::{
    ProviderAuthMethodDto, ProviderAuthMethodsResponse, ProviderListResponse,
    ProviderOAuthAuthorizationDto,
};

impl GatewayClient {
    /// 获取指定目录上下文下可用的 provider 列表。
    pub async fn provider_list(&self, directory: Option<&str>) -> Result<ProviderListResponse, String> {
        gateway_model_provider::GatewayModelProviderClient::new(self.transport.clone())
            .provider_list(directory).await
    }

    /// 获取全部 provider 的可用认证方式。
    pub async fn provider_auth_methods(&self) -> Result<ProviderAuthMethodsResponse, String> {
        gateway_model_provider::GatewayModelProviderClient::new(self.transport.clone())
            .provider_auth_methods().await
    }

    /// 启动指定 provider 的 OAuth 授权流程。
    pub async fn provider_oauth_authorize(
        &self,
        provider_id: &str,
        method: usize,
    ) -> Result<Option<ProviderOAuthAuthorizationDto>, String> {
        gateway_model_provider::GatewayModelProviderClient::new(self.transport.clone())
            .provider_oauth_authorize(provider_id, method).await
    }

    /// 完成指定 provider 的 OAuth 回调。
    pub async fn provider_oauth_callback(
        &self,
        provider_id: &str,
        method: usize,
        code: &str,
    ) -> Result<bool, String> {
        gateway_model_provider::GatewayModelProviderClient::new(self.transport.clone())
            .provider_oauth_callback(provider_id, method, code).await
    }
}

#[cfg(test)]
#[path = "provider_api_tests.rs"]
mod provider_api_tests;
