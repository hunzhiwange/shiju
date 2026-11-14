//! Device synchronization operations use native transport without retries or redirects.
use super::GatewayClient;
use serde::{Serialize, de::DeserializeOwned};
use vw_api_types::{
    agent_apps::{AppId, DataManagementTarget},
    device_sync_config::{DeviceSyncTransport, DeviceSyncRemoteResponse, UpdateWebDavSyncConfigRequest, UpdateS3SyncConfigRequest},
    device_sync_task::{SyncExpectedRemote, SyncRemoteSnapshot, SyncTaskStatus},
    id::ProjectId,
};

/// Explicit selection; empty collections never select all objects.
#[derive(Default, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceSyncSelection {
    pub providers: Vec<(AppId, String)>,
    pub mcp: Vec<String>,
    pub prompts: Vec<(AppId, String)>,
    pub preferences: bool,
    pub repositories: Vec<(String, String)>,
    pub skills: Vec<String>,
    pub profiles: Vec<String>,
    pub profile_skill_directories: std::collections::BTreeMap<String, String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceSyncUploadRequest {
    pub target: DataManagementTarget,
    pub request_id: String,
    pub expected_remote_snapshot: SyncExpectedRemote,
    pub selection: DeviceSyncSelection,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceSyncDownloadRequest {
    pub target: DataManagementTarget,
    pub request_id: String,
    pub expected_remote_snapshot: SyncRemoteSnapshot,
    pub expected_local_snapshot: String,
    pub connection_revision: String,
    pub selection: DeviceSyncSelection,
}

/// Keep error bodies available to conflict handling, but never format secret material.
#[derive(Clone, PartialEq)]
pub enum DeviceSyncOperationsError {
    InvalidInput,
    Transport,
    Http { status: u16, body: serde_json::Value },
    Decode,
}
impl std::fmt::Debug for DeviceSyncOperationsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(self, f)
    }
}
impl std::fmt::Display for DeviceSyncOperationsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidInput => f.write_str("invalid device sync request"),
            Self::Transport => f.write_str("device sync gateway unavailable"),
            Self::Http { status, .. } => write!(f, "device sync gateway HTTP {status}"),
            Self::Decode => f.write_str("invalid device sync gateway response"),
        }
    }
}
impl std::error::Error for DeviceSyncOperationsError {}

impl GatewayClient {
    pub async fn device_sync_test_webdav(&self, project: &ProjectId, host: &str,
        draft: &UpdateWebDavSyncConfigRequest, token: &str) -> Result<(), DeviceSyncOperationsError> {
        self.device_sync_operation_request(reqwest::Method::POST, DeviceSyncTransport::Webdav,
            "test", Some((project, host)), Some(draft), token, 204).await.map(|_| ())
    }
    pub async fn device_sync_test_s3(&self, project: &ProjectId, host: &str,
        draft: &UpdateS3SyncConfigRequest, token: &str) -> Result<(), DeviceSyncOperationsError> {
        self.device_sync_operation_request(reqwest::Method::POST, DeviceSyncTransport::S3,
            "test", Some((project, host)), Some(draft), token, 204).await.map(|_| ())
    }
    pub async fn device_sync_remote(&self, transport: DeviceSyncTransport, project: &ProjectId,
        host: &str, token: &str) -> Result<DeviceSyncRemoteResponse, DeviceSyncOperationsError> {
        let bytes = self.device_sync_operation_request::<()>(reqwest::Method::GET, transport,
            "remote", Some((project, host)), None, token, 200).await?;
        decode(&bytes)
    }
    /// Submit once; ambiguous network failures require inspecting durable task state.
    pub async fn device_sync_upload(&self, transport: DeviceSyncTransport,
        request: &DeviceSyncUploadRequest, token: &str) -> Result<SyncTaskStatus, DeviceSyncOperationsError> {
        let bytes = self.device_sync_operation_request(reqwest::Method::POST, transport,
            "upload", None, Some(request), token, 202).await?;
        decode(&bytes)
    }
    pub async fn device_sync_download(&self, transport: DeviceSyncTransport,
        request: &DeviceSyncDownloadRequest, token: &str) -> Result<SyncTaskStatus, DeviceSyncOperationsError> {
        let bytes = self.device_sync_operation_request(reqwest::Method::POST, transport,
            "download", None, Some(request), token, 202).await?;
        decode(&bytes)
    }
    pub(super) async fn device_sync_operation_request<B: Serialize>(&self, method: reqwest::Method,
        transport: DeviceSyncTransport, operation: &str, scope: Option<(&ProjectId, &str)>,
        body: Option<&B>, token: &str, expected_status: u16) -> Result<Vec<u8>, DeviceSyncOperationsError> {
        use DeviceSyncOperationsError as E;
        let mut credential = reqwest::header::HeaderValue::from_str(token).map_err(|_| E::InvalidInput)?;
        if token.trim().is_empty() { return Err(E::InvalidInput); }
        credential.set_sensitive(true);
        let transport = match transport { DeviceSyncTransport::Webdav => "webdav", DeviceSyncTransport::S3 => "s3" };
        let mut request = self.native_transport().map_err(|_| E::Transport)?.request(method,
            format!("{}/v1/agent-apps/device-sync/{transport}/{operation}", self.endpoint.base_url()))
            .header("x-agent-apps-token", credential);
        if let Some((project, host)) = scope {
            if host.trim().is_empty() { return Err(E::InvalidInput); }
            request = request.query(&[("projectId", project.as_ref()), ("targetHostId", host)]);
        }
        if let Some(body) = body { request = request.json(body); }
        let response = crate::http::apply_request_context(request, &self.endpoint).send().await.map_err(|_| E::Transport)?;
        let status = response.status().as_u16();
        let bytes = response.bytes().await.map_err(|_| E::Transport)?;
        if !(200..300).contains(&status) {
            let body = serde_json::from_slice(&bytes).unwrap_or_else(|_| serde_json::Value::String(String::from_utf8_lossy(&bytes).into_owned()));
            return Err(E::Http { status, body });
        }
        if status != expected_status { return Err(E::Decode); }
        Ok(bytes.to_vec())
    }
}
fn decode<T: DeserializeOwned>(bytes: &[u8]) -> Result<T, DeviceSyncOperationsError> {
    serde_json::from_slice(bytes).map_err(|_| DeviceSyncOperationsError::Decode)
}
