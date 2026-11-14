//! Dashboard actions preserve tenant context, project/host scope and HTTP errors.
use super::GatewayClient;
use serde::de::DeserializeOwned;
use vw_api_types::hermes_dashboard::*;

#[derive(Debug, PartialEq, Eq)]
pub enum HermesDashboardError { Transport, InvalidInput, Decode, Http(u16) }
const BASE: &str = "/v1/agent-apps/hermes/dashboard";
impl GatewayClient {
    pub async fn hermes_dashboard_probe(&self, scope: &DashboardScope, token: &str) -> Result<DashboardProbe, HermesDashboardError> {
        self.hermes_dashboard_request(reqwest::Method::POST, "probe", scope, token).await
    }
    /// Returns process progress. Probe separately before opening the panel.
    pub async fn hermes_dashboard_start(&self, scope: &DashboardScope, token: &str) -> Result<DashboardOperation, HermesDashboardError> {
        self.hermes_dashboard_request(reqwest::Method::POST, "start", scope, token).await
    }
    pub async fn hermes_dashboard_operation(&self, scope: &DashboardScope, id: &str, token: &str) -> Result<DashboardOperation, HermesDashboardError> {
        self.hermes_dashboard_request(reqwest::Method::GET, &operation_path(id)?, scope, token).await
    }
    /// Cancellation returns only after the execution process has been reaped.
    pub async fn hermes_dashboard_cancel(&self, scope: &DashboardScope, id: &str, token: &str) -> Result<DashboardOperation, HermesDashboardError> {
        self.hermes_dashboard_request(reqwest::Method::DELETE, &operation_path(id)?, scope, token).await
    }
    async fn hermes_dashboard_request<T: DeserializeOwned>(&self, method: reqwest::Method, resource: &str, scope: &DashboardScope, token: &str) -> Result<T, HermesDashboardError> {
        use HermesDashboardError as E;
        let transport = self.native_transport().map_err(|_| E::Transport)?;
        let mut credential = reqwest::header::HeaderValue::from_str(token).map_err(|_| E::InvalidInput)?;
        credential.set_sensitive(true);
        let request = transport.request(method, format!("{}{BASE}/{resource}", self.endpoint.base_url()))
            .query(scope).header("x-agent-apps-token", credential);
        let response = crate::http::apply_request_context(request, &self.endpoint).send().await.map_err(|_| E::Transport)?;
        if !response.status().is_success() { return Err(E::Http(response.status().as_u16())); }
        response.json().await.map_err(|_| E::Decode)
    }
}
fn operation_path(id: &str) -> Result<String, HermesDashboardError> {
    if id.is_empty() || id.len() > 64 || !id.bytes().all(|b| b.is_ascii_hexdigit() || b == b'-') { return Err(HermesDashboardError::InvalidInput); }
    Ok(format!("operations/{id}"))
}
#[cfg(test)]
#[path = "hermes_dashboard_api_tests.rs"]
mod tests;
