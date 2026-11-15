//! 授权设备同步配置、操作与任务客户端。
#[derive(Debug, Clone)]
pub struct GatewayDeviceSyncClient {
    transport: gateway_http::GatewayTransport,
}

impl GatewayDeviceSyncClient {
    pub fn new(transport: gateway_http::GatewayTransport) -> Self {
        Self { transport }
    }
}

mod device_sync_operations_api;
#[cfg(not(target_arch = "wasm32"))]
mod device_sync_config_api;
mod device_sync_tasks_api;
pub use device_sync_operations_api::{DeviceSyncOperationsError, DeviceSyncSelection, DeviceSyncUploadRequest, DeviceSyncDownloadRequest};
pub use device_sync_tasks_api::DeviceSyncTasksError;

#[cfg(not(target_arch = "wasm32"))]
pub mod device_sync_config;
#[cfg(not(target_arch = "wasm32"))]
pub mod device_sync_tasks;

#[cfg(not(target_arch = "wasm32"))]
pub mod device_sync_jobs;

#[cfg(not(target_arch = "wasm32"))]
pub mod device_sync_probe;

#[cfg(not(target_arch = "wasm32"))]
mod device_sync_download;
#[cfg(all(not(target_arch = "wasm32"), target_os = "macos"))]
pub mod device_sync_projection;
