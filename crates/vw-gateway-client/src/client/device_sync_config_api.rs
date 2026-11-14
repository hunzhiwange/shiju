//! Host-owned synchronization settings; secrets are sent only as explicit write intent.
use super::{DeviceSyncOperationsError, GatewayClient};
use vw_api_types::{
    device_sync_config::{
        DeviceSyncTransport, S3SyncConfigResponse, UpdateS3SyncConfigRequest,
        UpdateWebDavSyncConfigRequest, WebDavSyncConfigResponse,
    },
    id::ProjectId,
};

impl GatewayClient {
    pub async fn device_sync_webdav_config(
        &self, project: &ProjectId, host: &str, token: &str,
    ) -> Result<WebDavSyncConfigResponse, DeviceSyncOperationsError> {
        let bytes = self.device_sync_operation_request::<()>(
            reqwest::Method::GET, DeviceSyncTransport::Webdav, "config",
            Some((project, host)), None, token, 200,
        ).await?;
        serde_json::from_slice(&bytes).map_err(|_| DeviceSyncOperationsError::Decode)
    }

    /// Secret Keep/Replace/Clear intent is serialized unchanged for host validation.
    pub async fn device_sync_save_webdav_config(
        &self, project: &ProjectId, host: &str,
        draft: &UpdateWebDavSyncConfigRequest, token: &str,
    ) -> Result<WebDavSyncConfigResponse, DeviceSyncOperationsError> {
        let bytes = self.device_sync_operation_request(
            reqwest::Method::PUT, DeviceSyncTransport::Webdav, "config",
            Some((project, host)), Some(draft), token, 200,
        ).await?;
        serde_json::from_slice(&bytes).map_err(|_| DeviceSyncOperationsError::Decode)
    }

    /// Clear only this authorized host connection; remote objects remain intact.
    pub async fn device_sync_clear_webdav_config(
        &self, project: &ProjectId, host: &str, token: &str,
    ) -> Result<(), DeviceSyncOperationsError> {
        self.device_sync_operation_request::<()>(
            reqwest::Method::DELETE, DeviceSyncTransport::Webdav, "config",
            Some((project, host)), None, token, 204,
        ).await.map(|_| ())
    }

    pub async fn device_sync_s3_config(
        &self, project: &ProjectId, host: &str, token: &str,
    ) -> Result<S3SyncConfigResponse, DeviceSyncOperationsError> {
        let bytes = self.device_sync_operation_request::<()>(
            reqwest::Method::GET, DeviceSyncTransport::S3, "config",
            Some((project, host)), None, token, 200,
        ).await?;
        serde_json::from_slice(&bytes).map_err(|_| DeviceSyncOperationsError::Decode)
    }

    pub async fn device_sync_save_s3_config(
        &self, project: &ProjectId, host: &str,
        draft: &UpdateS3SyncConfigRequest, token: &str,
    ) -> Result<S3SyncConfigResponse, DeviceSyncOperationsError> {
        let bytes = self.device_sync_operation_request(
            reqwest::Method::PUT, DeviceSyncTransport::S3, "config",
            Some((project, host)), Some(draft), token, 200,
        ).await?;
        serde_json::from_slice(&bytes).map_err(|_| DeviceSyncOperationsError::Decode)
    }
}
