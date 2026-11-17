//! 识矩账号兼容门面，敏感请求由领域客户端持有。
use super::GatewayClient;
#[cfg(any(test, target_arch = "wasm32"))]
use serde::{Serialize, de::DeserializeOwned};
pub use gateway_shiju_account::*;

impl GatewayClient {
    pub async fn shiju_model_token(
        &self,
        credential: &ShijuCredential,
    ) -> Result<ShijuModelTokenDto, String> {
        gateway_shiju_account::GatewayShijuAccountClient::new(self.transport.clone())
            .shiju_model_token(credential).await
    }

    pub async fn shiju_account_login(&self) -> Result<ShijuLoginChallengeDto, String> {
        gateway_shiju_account::GatewayShijuAccountClient::new(self.transport.clone())
            .shiju_account_login().await
    }

    pub async fn shiju_account_login_poll(&self, state: &str) -> Result<ShijuLoginPollDto, String> {
        gateway_shiju_account::GatewayShijuAccountClient::new(self.transport.clone())
            .shiju_account_login_poll(state).await
    }

    pub async fn shiju_account_profile(
        &self,
        credential: &ShijuCredential,
    ) -> Result<ShijuAccountProfileDto, String> {
        gateway_shiju_account::GatewayShijuAccountClient::new(self.transport.clone())
            .shiju_account_profile(credential).await
    }

    pub async fn shiju_account_monthly_quota(
        &self,
        credential: &ShijuCredential,
    ) -> Result<ShijuMonthlyQuotaDto, String> {
        gateway_shiju_account::GatewayShijuAccountClient::new(self.transport.clone())
            .shiju_account_monthly_quota(credential).await
    }

    pub async fn shiju_account_me(
        &self,
        credential: &ShijuCredential,
    ) -> Result<ShijuAccountMeDto, String> {
        gateway_shiju_account::GatewayShijuAccountClient::new(self.transport.clone())
            .shiju_account_me(credential).await
    }

    pub async fn shiju_account_update(
        &self,
        credential: &ShijuCredential,
        body: &ShijuAccountUpdateRequest,
    ) -> Result<ShijuAccountUpdateResponseDto, String> {
        gateway_shiju_account::GatewayShijuAccountClient::new(self.transport.clone())
            .shiju_account_update(credential, body).await
    }

    pub async fn shiju_account_password_update(
        &self,
        credential: &ShijuCredential,
        body: &ShijuAccountPasswordUpdateRequest,
    ) -> Result<(), String> {
        gateway_shiju_account::GatewayShijuAccountClient::new(self.transport.clone())
            .shiju_account_password_update(credential, body).await
    }

    #[cfg(test)]
    pub(super) async fn shiju_account_sensitive_request<B, T>(
        &self,
        method: reqwest::Method,
        path: &str,
        query: &[(String, String)],
        body: Option<&B>,
        credential: Option<&ShijuCredential>,
    ) -> Result<T, String>
    where
        B: Serialize,
        T: DeserializeOwned,
    {
        gateway_shiju_account::GatewayShijuAccountClient::new(self.transport.clone())
            .shiju_account_sensitive_request(method, path, query, body, credential).await
    }

    #[cfg(test)]
    pub(super) async fn shiju_empty_sensitive_request<B>(
        &self,
        method: reqwest::Method,
        path: &str,
        query: &[(String, String)],
        body: Option<&B>,
        credential: Option<&ShijuCredential>,
    ) -> Result<(), String>
    where
        B: Serialize,
    {
        gateway_shiju_account::GatewayShijuAccountClient::new(self.transport.clone())
            .shiju_empty_sensitive_request(method, path, query, body, credential).await
    }

}
