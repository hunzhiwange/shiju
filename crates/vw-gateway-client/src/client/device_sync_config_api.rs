use super::{DeviceSyncOperationsError, GatewayClient};
use {
    vw_api_device_sync::device_sync_config::DeviceSyncTransport,
    vw_api_device_sync::device_sync_config::S3SyncConfigResponse,
    vw_api_device_sync::device_sync_config::UpdateS3SyncConfigRequest,
    vw_api_device_sync::device_sync_config::UpdateWebDavSyncConfigRequest,
    vw_api_device_sync::device_sync_config::WebDavSyncConfigResponse,
    vw_api_transport::id::ProjectId,
};

impl GatewayClient {
    pub async fn device_sync_webdav_config(
        &self, project: &ProjectId, host: &str, token: &str,
    ) -> Result<WebDavSyncConfigResponse, DeviceSyncOperationsError> {
        gateway_device_sync::GatewayDeviceSyncClient::new(self.transport.clone())
            .device_sync_webdav_config(project, host, token).await
    }
    pub async fn device_sync_save_webdav_config(
        &self, project: &ProjectId, host: &str,
        draft: &UpdateWebDavSyncConfigRequest, token: &str,
    ) -> Result<WebDavSyncConfigResponse, DeviceSyncOperationsError> {
        gateway_device_sync::GatewayDeviceSyncClient::new(self.transport.clone())
            .device_sync_save_webdav_config(project, host, draft, token).await
    }
    pub async fn device_sync_clear_webdav_config(
        &self, project: &ProjectId, host: &str, token: &str,
    ) -> Result<(), DeviceSyncOperationsError> {
        gateway_device_sync::GatewayDeviceSyncClient::new(self.transport.clone())
            .device_sync_clear_webdav_config(project, host, token).await
    }
    pub async fn device_sync_s3_config(
        &self, project: &ProjectId, host: &str, token: &str,
    ) -> Result<S3SyncConfigResponse, DeviceSyncOperationsError> {
        gateway_device_sync::GatewayDeviceSyncClient::new(self.transport.clone())
            .device_sync_s3_config(project, host, token).await
    }
    pub async fn device_sync_save_s3_config(
        &self, project: &ProjectId, host: &str,
        draft: &UpdateS3SyncConfigRequest, token: &str,
    ) -> Result<S3SyncConfigResponse, DeviceSyncOperationsError> {
        gateway_device_sync::GatewayDeviceSyncClient::new(self.transport.clone())
            .device_sync_save_s3_config(project, host, draft, token).await
    }
}
