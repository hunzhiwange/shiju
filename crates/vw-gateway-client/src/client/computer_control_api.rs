//! Desktop management requests are sent once; errors retain HTTP status without raw payloads.
use super::GatewayClient;
use serde::{Serialize, de::DeserializeOwned};
use vw_api_types::computer_control::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ComputerControlApiError { Transport, Decode, Http(u16) }
impl std::fmt::Display for ComputerControlApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self { Self::Transport => f.write_str("computer control transport unavailable"), Self::Decode => f.write_str("invalid computer control response"), Self::Http(status) => write!(f, "computer control HTTP {status}") }
    }
}
impl std::error::Error for ComputerControlApiError {}
impl GatewayClient {
    pub async fn computer_control_status(&self) -> Result<ComputerControlStatus, ComputerControlApiError> {
        self.control_request::<(), _>(reqwest::Method::GET, "status", None).await
    }
    pub async fn computer_control_approvals(&self) -> Result<ControlApprovals, ComputerControlApiError> {
        self.control_request::<(), _>(reqwest::Method::GET, "approvals", None).await
    }
    pub async fn computer_control_save(&self, body: &ControlSettingsUpdate) -> Result<ControlVersionResponse, ComputerControlApiError> {
        self.control_request(reqwest::Method::PUT, "settings", Some(body)).await
    }
    pub async fn computer_control_revoke(&self, body: &ControlGrantRevocation) -> Result<ControlVersionResponse, ComputerControlApiError> {
        self.control_request(reqwest::Method::DELETE, "approvals", Some(body)).await
    }
    pub async fn computer_control_pair(&self, body: &ControlPairingConfirmation) -> Result<ControlPairingResponse, ComputerControlApiError> {
        self.control_request(reqwest::Method::POST, "pairing", Some(body)).await
    }
    pub async fn computer_control_stop(&self) -> Result<(), ComputerControlApiError> {
        self.control_send::<()>(reqwest::Method::POST, "stop", None).await.map(|_| ())
    }
    async fn control_send<B: Serialize>(&self, method: reqwest::Method, resource: &str, body: Option<&B>) -> Result<reqwest::Response, ComputerControlApiError> {
        let transport = self.native_transport().map_err(|_| ComputerControlApiError::Transport)?;
        let mut request = transport.request(method, format!("{}/v1/computer-control/{resource}", self.endpoint.base_url()));
        if let Some(body) = body { request = request.json(body); }
        let response = crate::http::apply_request_context(request, &self.endpoint).send().await.map_err(|_| ComputerControlApiError::Transport)?;
        if !response.status().is_success() { return Err(ComputerControlApiError::Http(response.status().as_u16())); }
        Ok(response)
    }
    async fn control_request<B: Serialize, T: DeserializeOwned>(&self, method: reqwest::Method, resource: &str, body: Option<&B>) -> Result<T, ComputerControlApiError> {
        self.control_send(method, resource, body).await?.json().await.map_err(|_| ComputerControlApiError::Decode)
    }
}
