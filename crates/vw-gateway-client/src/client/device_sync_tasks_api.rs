//! Explicit task polling and cancellation. Transport failures never restart work.
use super::GatewayClient;
use crate::http::apply_request_context;
use reqwest::{Method, StatusCode, Url};
use vw_api_types::{
    TaskId,
    agent_apps::DataManagementTarget,
    device_sync_task::{SyncTaskCancelResponse, SyncTaskStatus},
};

/// Safe SDK errors: response bodies and authorization tokens are never retained.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeviceSyncTasksError {
    InvalidTarget,
    InvalidTaskId,
    InvalidToken,
    Transport,
    Http { status: u16 },
    Decode,
}

impl std::fmt::Display for DeviceSyncTasksError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "device sync task request failed: {self:?}")
    }
}
impl std::error::Error for DeviceSyncTasksError {}

impl GatewayClient {
    /// Re-read the original task after a disconnect; this never starts a new transfer.
    pub async fn device_sync_task_status(
        &self,
        target: &DataManagementTarget,
        task_id: &TaskId,
        token: &str,
    ) -> Result<SyncTaskStatus, DeviceSyncTasksError> {
        let response = self.device_sync_task_request(Method::GET, target, task_id, token).await?;
        if response.status() != StatusCode::OK {
            return Err(DeviceSyncTasksError::Http { status: response.status().as_u16() });
        }
        let status: SyncTaskStatus = response.json().await.map_err(|_| DeviceSyncTasksError::Decode)?;
        if status.task_id != *task_id || status.target != *target {
            return Err(DeviceSyncTasksError::Decode);
        }
        Ok(status)
    }

    /// Request cancellation once. Accepted running work must still be polled;
    /// TooLate preserves the server state and warnings even though HTTP is 409.
    pub async fn device_sync_task_cancel(
        &self,
        target: &DataManagementTarget,
        task_id: &TaskId,
        token: &str,
    ) -> Result<SyncTaskCancelResponse, DeviceSyncTasksError> {
        let response = self.device_sync_task_request(Method::DELETE, target, task_id, token).await?;
        let http_status = response.status();
        if !matches!(http_status, StatusCode::OK | StatusCode::CONFLICT) {
            return Err(DeviceSyncTasksError::Http { status: http_status.as_u16() });
        }
        let result: SyncTaskCancelResponse = response.json().await.map_err(|_| DeviceSyncTasksError::Decode)?;
        let (status, expected_http) = match &result {
            SyncTaskCancelResponse::Accepted { status } => (status, StatusCode::OK),
            SyncTaskCancelResponse::TooLate { status } => (status, StatusCode::CONFLICT),
        };
        if http_status != expected_http || status.task_id != *task_id || status.target != *target {
            return Err(DeviceSyncTasksError::Decode);
        }
        Ok(result)
    }

    async fn device_sync_task_request(
        &self,
        method: Method,
        target: &DataManagementTarget,
        task_id: &TaskId,
        token: &str,
    ) -> Result<reqwest::Response, DeviceSyncTasksError> {
        if target.scope.tenant_scope() != self.endpoint.tenant_scope()
            || target.target_host_id.trim().is_empty()
        {
            return Err(DeviceSyncTasksError::InvalidTarget);
        }
        let id = task_id.as_ref();
        if id.is_empty() || matches!(id, "." | "..") {
            return Err(DeviceSyncTasksError::InvalidTaskId);
        }
        if token.trim().is_empty() {
            return Err(DeviceSyncTasksError::InvalidToken);
        }
        let mut authorization = reqwest::header::HeaderValue::from_str(&format!("Bearer {token}"))
            .map_err(|_| DeviceSyncTasksError::InvalidToken)?;
        authorization.set_sensitive(true);
        let mut url = Url::parse(&self.endpoint.base_url()).map_err(|_| DeviceSyncTasksError::InvalidTarget)?;
        url.path_segments_mut().map_err(|_| DeviceSyncTasksError::InvalidTarget)?
            .pop_if_empty().extend(["v1", "agent-apps", "device-sync", "tasks", id]);
        let client = self.native_transport().map_err(|_| DeviceSyncTasksError::Transport)?;
        let request = client.request(method, url).query(&[
            ("projectId", target.scope.project_id().as_ref()),
            ("targetHostId", target.target_host_id.as_str()),
        ]);
        apply_request_context(request, &self.endpoint)
            .header(reqwest::header::AUTHORIZATION, authorization)
            .header(reqwest::header::CACHE_CONTROL, "no-store")
            .send().await.map_err(|_| DeviceSyncTasksError::Transport)
    }
}
