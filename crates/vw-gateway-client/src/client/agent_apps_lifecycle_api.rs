//! 兼容门面：生命周期行为由 gateway-app-lifecycle 拥有。
use vw_api_app_operations::{AppActionExecutionResult, AppActionPreview, AppActionPreviewRequest, AppActionSubmitRequest, AppOperationStatus};
use super::GatewayClient;
use serde::{Serialize, de::DeserializeOwned};
use vw_api_types::agent_apps::*;
use vw_api_transport::id::TaskId;
pub use gateway_app_lifecycle::AgentAppsLifecycleError;
pub(super) use gateway_app_lifecycle::operation_path;

impl GatewayClient {
    pub async fn agent_apps_action_preview(
        &self,
        host: &str,
        request: &AppActionPreviewRequest,
    ) -> Result<AppActionPreview, AgentAppsLifecycleError> {
        gateway_app_lifecycle::GatewayAppLifecycleClient::new(self.transport.clone())
            .agent_apps_action_preview(host, request).await
    }

    pub async fn agent_apps_action_submit(
        &self,
        host: &str,
        request: &AppActionSubmitRequest,
    ) -> Result<AppActionExecutionResult, AgentAppsLifecycleError> {
        gateway_app_lifecycle::GatewayAppLifecycleClient::new(self.transport.clone())
            .agent_apps_action_submit(host, request).await
    }

    pub async fn agent_apps_operation(
        &self,
        host: &str,
        id: &TaskId,
    ) -> Result<AppOperationStatus, AgentAppsLifecycleError> {
        gateway_app_lifecycle::GatewayAppLifecycleClient::new(self.transport.clone())
            .agent_apps_operation(host, id).await
    }

    pub async fn agent_apps_operation_events(
        &self,
        host: &str,
        id: &TaskId,
    ) -> Result<AppOperationStatus, AgentAppsLifecycleError> {
        gateway_app_lifecycle::GatewayAppLifecycleClient::new(self.transport.clone())
            .agent_apps_operation_events(host, id).await
    }

    pub async fn agent_apps_operation_cancel(
        &self,
        host: &str,
        id: &TaskId,
    ) -> Result<AppOperationStatus, AgentAppsLifecycleError> {
        gateway_app_lifecycle::GatewayAppLifecycleClient::new(self.transport.clone())
            .agent_apps_operation_cancel(host, id).await
    }

    pub async fn agent_apps_action_preview_authorized(
        &self,
        host: &str,
        request: &AppActionPreviewRequest,
        token: &str,
    ) -> Result<AppActionPreview, AgentAppsLifecycleError> {
        gateway_app_lifecycle::GatewayAppLifecycleClient::new(self.transport.clone())
            .agent_apps_action_preview_authorized(host, request, token).await
    }

    pub async fn agent_apps_action_submit_authorized(
        &self,
        host: &str,
        request: &AppActionSubmitRequest,
        token: &str,
    ) -> Result<AppActionExecutionResult, AgentAppsLifecycleError> {
        gateway_app_lifecycle::GatewayAppLifecycleClient::new(self.transport.clone())
            .agent_apps_action_submit_authorized(host, request, token).await
    }

    pub async fn agent_apps_operation_authorized(
        &self,
        host: &str,
        id: &TaskId,
        token: &str,
    ) -> Result<AppOperationStatus, AgentAppsLifecycleError> {
        gateway_app_lifecycle::GatewayAppLifecycleClient::new(self.transport.clone())
            .agent_apps_operation_authorized(host, id, token).await
    }

    pub async fn agent_apps_operation_events_authorized(
        &self,
        host: &str,
        id: &TaskId,
        token: &str,
    ) -> Result<AppOperationStatus, AgentAppsLifecycleError> {
        gateway_app_lifecycle::GatewayAppLifecycleClient::new(self.transport.clone())
            .agent_apps_operation_events_authorized(host, id, token).await
    }

    pub async fn agent_apps_operation_cancel_authorized(
        &self,
        host: &str,
        id: &TaskId,
        token: &str,
    ) -> Result<AppOperationStatus, AgentAppsLifecycleError> {
        gateway_app_lifecycle::GatewayAppLifecycleClient::new(self.transport.clone())
            .agent_apps_operation_cancel_authorized(host, id, token).await
    }

    pub(super) async fn agent_apps_lifecycle_request<B: Serialize, T: DeserializeOwned>(
        &self,
        method: reqwest::Method,
        resource: &str,
        host: &str,
        body: Option<&B>,
        token: Option<&str>,
    ) -> Result<T, AgentAppsLifecycleError> {
        gateway_app_lifecycle::GatewayAppLifecycleClient::new(self.transport.clone())
            .agent_apps_lifecycle_request(method, resource, host, body, token).await
    }
}
