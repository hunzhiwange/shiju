//! Fixed provider operation endpoints; raw worker errors never leave the host.
use super::super::agent_apps_read_api::AgentAppsReadError as Error;
use super::GatewayClient;
use vw_api_types::{agent_apps::aggregation::AggregationQuery, app_providers::operations::*};
impl GatewayClient {
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
            Some(token),
            None,
        )
        .await
    }
    pub async fn provider_operation_get(
        &self,
        id: &str,
        host: &str,
        token: &str,
    ) -> Result<ProviderOperationProgress, Error> {
        self.provider_operation_request("GET", Some(id), host, None, Some(token), None).await
    }
    pub async fn provider_operation_cancel(
        &self,
        id: &str,
        host: &str,
        token: &str,
    ) -> Result<ProviderOperationProgress, Error> {
        self.provider_operation_request("DELETE", Some(id), host, None, Some(token), None).await
    }
    /// Query the original project-owned aggregation operation using endpoint Bearer auth.
    /// A failed outcome remains a typed operation result; never replay the submit here.
    pub async fn provider_aggregation_operation_get(
        &self,
        id: &str,
        scope: &AggregationQuery,
    ) -> Result<ProviderOperationProgress, Error> {
        self.provider_operation_request(
            "GET",
            Some(id),
            &scope.target_host_id,
            None,
            None,
            Some(scope),
        )
        .await
    }
    /// Request cancellation once; the returned snapshot is authoritative even if
    /// cancellation was requested after the irreversible publication boundary.
    pub async fn provider_aggregation_operation_cancel(
        &self,
        id: &str,
        scope: &AggregationQuery,
    ) -> Result<ProviderOperationProgress, Error> {
        self.provider_operation_request(
            "DELETE",
            Some(id),
            &scope.target_host_id,
            None,
            None,
            Some(scope),
        )
        .await
    }
    async fn provider_operation_request(
        &self,
        method: &str,
        id: Option<&str>,
        host: &str,
        body: Option<&ProviderOperationRequest>,
        token: Option<&str>,
        scope: Option<&AggregationQuery>,
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
            let _ = (method, id, host, body, token, scope);
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
            let url = format!("{}/v1/app-providers/operations{path}", self.endpoint.base_url());
            let mut request = self.dynamic_host_client.request(method, url);
            if let Some(token) = token {
                let mut credential = reqwest::header::HeaderValue::from_str(token)
                    .map_err(|_| Error::Transport("invalid application credential".into()))?;
                credential.set_sensitive(true);
                request = request.header("x-agent-apps-token", credential);
            }
            if let Some(body) = body {
                request = request.json(body);
            } else if let Some(scope) = scope {
                request = request.query(scope);
            } else {
                request = request.query(&[("targetHostId", host)]);
            }
            let response = crate::http::apply_request_context(request, &self.endpoint)
                .send()
                .await
                .map_err(|_| Error::Transport("Provider operation request failed".into()))?;
            let status = response.status();
            if !status.is_success() {
                let body = if scope.is_some() {
                    response.json().await.unwrap_or(serde_json::Value::Null)
                } else {
                    serde_json::Value::Null
                };
                return Err(Error::Http { status: status.as_u16(), body });
            }
            response
                .json()
                .await
                .map_err(|_| Error::Decode("invalid Provider operation response".into()))
        }
    }
}
