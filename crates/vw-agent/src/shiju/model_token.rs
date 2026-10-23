//! 通过 Passport 登录凭据领取独立的模型凭据，禁止跟随重定向或透传错误正文。

use super::{ShijuAccountClient, ShijuAccountError, endpoint};
use reqwest::{StatusCode, redirect::Policy};
use serde::Deserialize;
use std::time::Duration;
use vw_gateway_client::{ShijuCredential, ShijuModelTokenDto};

const MODEL_ENDPOINT: &str = "https://router.shiju.cloud/v1";

#[derive(Deserialize)]
struct TokenEnvelope {
    success: bool,
    data: Option<TokenData>,
}

#[derive(Deserialize)]
struct TokenData {
    key: String,
    #[serde(default)]
    endpoint_url: Option<String>,
}

impl ShijuAccountClient {
    pub(crate) async fn model_token(
        &self,
        credential: &str,
    ) -> Result<ShijuModelTokenDto, ShijuAccountError> {
        let failure =
            || ShijuAccountError::Message("识矩模型自动授权失败，请重试或联系管理员检查模型权限");
        if credential.is_empty() || !credential.bytes().all(|byte| (33..=126).contains(&byte)) {
            return Err(ShijuAccountError::Message("识矩登录凭据无效，请重新登录"));
        }
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(20))
            .redirect(Policy::none())
            .build()
            .map_err(|_| failure())?;
        let response = http
            .put(endpoint(&self.gateway_base, "/model/api/runtime/model-token")?)
            .header("X-Api-Key", credential)
            .send()
            .await
            .map_err(|_| failure())?;
        let status = response.status();
        if !status.is_success() {
            let message = match status {
                StatusCode::UNAUTHORIZED => "识矩登录凭据已失效，请重新登录",
                StatusCode::FORBIDDEN => "当前账户没有模型授权权限，请联系管理员",
                _ => "识矩模型自动授权失败，请重试或联系管理员检查模型权限",
            };
            return Err(ShijuAccountError::Upstream { status, message: message.to_string() });
        }
        let envelope = response.json::<TokenEnvelope>().await.map_err(|_| failure())?;
        let token = envelope.data.filter(|_| envelope.success).ok_or_else(failure)?;
        if token.key.is_empty() || !token.key.bytes().all(|byte| (33..=126).contains(&byte)) {
            return Err(failure());
        }
        // 自动账户模型只连接已知生产路由，拒绝把凭据发往任意上游地址。
        let endpoint_url = token.endpoint_url.as_deref().unwrap_or(MODEL_ENDPOINT);
        let url = reqwest::Url::parse(endpoint_url)
            .map_err(|_| ShijuAccountError::Message("识矩模型服务地址不受信任"))?;
        if url.scheme() != "https"
            || url.host_str() != Some("router.shiju.cloud")
            || !url.username().is_empty()
            || url.password().is_some()
            || url.port().is_some_and(|port| port != 443)
            || url.query().is_some()
            || url.fragment().is_some()
            || !matches!(url.path().trim_end_matches('/'), "" | "/v1")
        {
            return Err(ShijuAccountError::Message("识矩模型服务地址不受信任"));
        }
        Ok(ShijuModelTokenDto {
            key: ShijuCredential::new(token.key).map_err(|_| failure())?,
            endpoint_url: MODEL_ENDPOINT.to_string(),
        })
    }
}

#[cfg(test)]
#[path = "model_token_tests.rs"]
mod tests;
