use super::GatewayClient;
use super::DeviceSyncTasksError;

use {
    vw_api_transport::TaskId,
    vw_api_app_operations::DataManagementTarget,
    vw_api_device_sync::device_sync_task::SyncTaskCancelResponse,
    vw_api_device_sync::device_sync_task::SyncTaskStatus,
};

impl GatewayClient {
    pub async fn device_sync_task_status(
        &self,
        target: &DataManagementTarget,
        task_id: &TaskId,
        token: &str,
    ) -> Result<SyncTaskStatus, DeviceSyncTasksError> {
        gateway_device_sync::GatewayDeviceSyncClient::new(self.transport.clone())
            .device_sync_task_status(target, task_id, token).await
    }
    pub async fn device_sync_task_cancel(
        &self,
        target: &DataManagementTarget,
        task_id: &TaskId,
        token: &str,
    ) -> Result<SyncTaskCancelResponse, DeviceSyncTasksError> {
        gateway_device_sync::GatewayDeviceSyncClient::new(self.transport.clone())
            .device_sync_task_cancel(target, task_id, token).await
    }
}
