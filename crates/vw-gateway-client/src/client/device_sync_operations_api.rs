use super::GatewayClient;
use super::{DeviceSyncOperationsError, DeviceSyncSelection, DeviceSyncUploadRequest, DeviceSyncDownloadRequest};
use {
    vw_api_app_operations::DataManagementTarget,
    vw_api_device_sync::device_sync_config::DeviceSyncTransport,
    vw_api_device_sync::device_sync_config::DeviceSyncRemoteResponse,
    vw_api_device_sync::device_sync_config::UpdateWebDavSyncConfigRequest,
    vw_api_device_sync::device_sync_config::UpdateS3SyncConfigRequest,
    vw_api_device_sync::device_sync_task::SyncTaskStatus,
    vw_api_transport::id::ProjectId,
};

impl GatewayClient {
    pub async fn device_sync_test_webdav(&self, project: &ProjectId, host: &str,
        draft: &UpdateWebDavSyncConfigRequest, token: &str) -> Result<(), DeviceSyncOperationsError> {
        gateway_device_sync::GatewayDeviceSyncClient::new(self.transport.clone())
            .device_sync_test_webdav(project, host, draft, token).await
    }
    pub async fn device_sync_test_s3(&self, project: &ProjectId, host: &str,
        draft: &UpdateS3SyncConfigRequest, token: &str) -> Result<(), DeviceSyncOperationsError> {
        gateway_device_sync::GatewayDeviceSyncClient::new(self.transport.clone())
            .device_sync_test_s3(project, host, draft, token).await
    }
    pub async fn device_sync_remote(&self, transport: DeviceSyncTransport, project: &ProjectId,
        host: &str, token: &str) -> Result<DeviceSyncRemoteResponse, DeviceSyncOperationsError> {
        gateway_device_sync::GatewayDeviceSyncClient::new(self.transport.clone())
            .device_sync_remote(transport, project, host, token).await
    }
    pub async fn device_sync_upload(&self, transport: DeviceSyncTransport,
        request: &DeviceSyncUploadRequest, token: &str) -> Result<SyncTaskStatus, DeviceSyncOperationsError> {
        gateway_device_sync::GatewayDeviceSyncClient::new(self.transport.clone())
            .device_sync_upload(transport, request, token).await
    }
    pub async fn device_sync_download(&self, transport: DeviceSyncTransport,
        request: &DeviceSyncDownloadRequest, token: &str) -> Result<SyncTaskStatus, DeviceSyncOperationsError> {
        gateway_device_sync::GatewayDeviceSyncClient::new(self.transport.clone())
            .device_sync_download(transport, request, token).await
    }
    pub async fn device_sync_local_preview(&self, transport: DeviceSyncTransport,
        target: &DataManagementTarget, selection: &DeviceSyncSelection, token: &str)
        -> Result<vw_api_device_sync::device_sync_config::DeviceSyncLocalPreview, DeviceSyncOperationsError> {
        gateway_device_sync::GatewayDeviceSyncClient::new(self.transport.clone())
            .device_sync_local_preview(transport, target, selection, token).await
    }
}
