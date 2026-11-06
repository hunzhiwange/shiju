use super::GatewayClient;
use vw_api_types::app_providers::universal::{UniversalProviderSummary, UniversalProviderWrite};
pub(super) fn path(id: &str) -> Result<String, String> {
    if id.is_empty()
        || id.len() > 128
        || !id.bytes().all(|c| c.is_ascii_alphanumeric() || matches!(c, b'-' | b'_'))
    {
        return Err("invalid universal provider identity".into());
    }
    Ok(format!("/v1/universal-providers/{id}"))
}
impl GatewayClient {
    pub(super) async fn universal_response<T: serde::de::DeserializeOwned>(
        &self,
        builder: reqwest::RequestBuilder,
    ) -> Result<T, String> {
        let response = crate::http::apply_request_context(builder, &self.endpoint)
            .send()
            .await
            .map_err(|_| "universal provider transport failed")?;
        let status = response.status();
        if !status.is_success() {
            return Err(format!("universal provider HTTP {status}"));
        }
        response.json().await.map_err(|_| "invalid universal provider response".into())
    }
    pub async fn universal_provider_save(
        &self,
        id: &str,
        request: &UniversalProviderWrite,
    ) -> Result<UniversalProviderSummary, String> {
        self.universal_response(
            self.client.put(format!("{}{}", self.endpoint.base_url(), path(id)?)).json(request),
        )
        .await
    }
    pub async fn universal_provider_get(
        &self,
        id: &str,
    ) -> Result<UniversalProviderSummary, String> {
        self.universal_response(self.client.get(format!(
            "{}{}",
            self.endpoint.base_url(),
            path(id)?
        )))
        .await
    }
    pub async fn universal_providers_list(&self) -> Result<Vec<UniversalProviderSummary>, String> {
        self.universal_response(
            self.client.get(format!("{}/v1/universal-providers", self.endpoint.base_url())),
        )
        .await
    }
    pub async fn universal_provider_delete(&self, id: &str) -> Result<bool, String> {
        self.universal_response(self.client.delete(format!(
            "{}{}",
            self.endpoint.base_url(),
            path(id)?
        )))
        .await
    }
}
