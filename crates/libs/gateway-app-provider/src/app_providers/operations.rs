//! Fixed provider operation endpoints; raw worker errors never leave the host.
use gateway_app_read::AgentAppsReadError as Error;
use super::GatewayAppProviderClient;
use vw_api_app_aggregation::operations::*;
impl GatewayAppProviderClient {
    pub async fn provider_operation_start(
        &self,
        request: &ProviderOperationRequest,
        token: &str,
    ) -> Result<ProviderOperationProgress, Error> {
        self.provider_operation_request(
            "POST",
            None,
            &request.target_host_id,
            Some(request),
            token,
        )
        .await
    }
    pub async fn provider_operation_get(
        &self,
        id: &str,
        host: &str,
        token: &str,
    ) -> Result<ProviderOperationProgress, Error> {
        self.provider_operation_request("GET", Some(id), host, None, token).await
    }
    pub async fn provider_operation_cancel(
        &self,
        id: &str,
        host: &str,
        token: &str,
    ) -> Result<ProviderOperationProgress, Error> {
        self.provider_operation_request("DELETE", Some(id), host, None, token).await
    }
    async fn provider_operation_request(
        &self,
        method: &str,
        id: Option<&str>,
        host: &str,
        body: Option<&ProviderOperationRequest>,
        _token: &str,
    ) -> Result<ProviderOperationProgress, Error> {
        if id.is_some_and(|id| {
            id.is_empty()
                || id.len() > 128
                || !id.bytes().all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_'))
        }) {
            return Err(Error::Transport("invalid operation identity".into()));
        }
        #[cfg(target_arch = "wasm32")]
        {
            let _ = (method, id, host, body, _token);
            Err(Error::Transport("Provider operations require native transport".into()))
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let path = match (id, body.map(|request| &request.action)) {
                (Some(id), _) => format!("/{id}"),
                (None, Some(ProviderOperationAction::EditorSave { .. })) => "/editor-save".into(),
                _ => String::new(),
            };
            let method = reqwest::Method::from_bytes(method.as_bytes())
                .map_err(|_| Error::Transport("invalid operation method".into()))?;
            let url = format!("{}/v1/app-providers/operations{path}", self.transport.endpoint().base_url());
            let mut request = self.transport.dynamic_host_client().request(method, url);
            if let Some(body) = body {
                request = request.json(body);
            } else {
                request = request.query(&[("targetHostId", host)]);
            }
            let response = gateway_http::apply_request_context(request, self.transport.endpoint())
                .send()
                .await
                .map_err(|_| Error::Transport("Provider operation request failed".into()))?;
            let status = response.status();
            if !status.is_success() {
                return Err(Error::Http { status: status.as_u16(), body: serde_json::Value::Null });
            }
            response
                .json()
                .await
                .map_err(|_| Error::Decode("invalid Provider operation response".into()))
        }
    }
}
