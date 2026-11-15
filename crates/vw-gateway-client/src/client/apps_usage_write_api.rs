//! Submit usage refreshes, manage their operations, and configure usage scripts.

use super::{
    AppsUsageSettings, GatewayClient,
};

use {
    vw_api_app_operations::AppActionExecutionResult,
    vw_api_apps_usage::apps_usage::UsageScriptPackage,
    vw_api_apps_usage::apps_usage::UsageSnapshotQuery,
    vw_api_transport::id::ProjectId,
    vw_api_transport::id::TaskId,
};

pub use gateway_app_usage::{AppsUsageWriteError, AppsUsageOperation, AppsUsageOperationItem, AppsUsageScriptTrial};

impl GatewayClient {
    pub async fn apps_usage_refresh(
        &self,
        selection: &UsageSnapshotQuery,
        principal_token: &str,
    ) -> Result<AppActionExecutionResult, AppsUsageWriteError> {
        gateway_app_usage::GatewayAppUsageClient::new(self.transport.clone())
            .apps_usage_refresh(selection, principal_token).await
    }

    pub async fn apps_usage_refresh_all(
        &self,
        project_id: &ProjectId,
        target_host_id: &str,
        principal_token: &str,
    ) -> Result<AppActionExecutionResult, AppsUsageWriteError> {
        gateway_app_usage::GatewayAppUsageClient::new(self.transport.clone())
            .apps_usage_refresh_all(project_id, target_host_id, principal_token).await
    }

    pub async fn apps_usage_operation(
        &self,
        target_host_id: &str,
        id: &TaskId,
        principal_token: &str,
    ) -> Result<AppsUsageOperation, AppsUsageWriteError> {
        gateway_app_usage::GatewayAppUsageClient::new(self.transport.clone())
            .apps_usage_operation(target_host_id, id, principal_token).await
    }

    pub async fn apps_usage_operation_events(
        &self,
        target_host_id: &str,
        id: &TaskId,
        principal_token: &str,
    ) -> Result<AppsUsageOperation, AppsUsageWriteError> {
        gateway_app_usage::GatewayAppUsageClient::new(self.transport.clone())
            .apps_usage_operation_events(target_host_id, id, principal_token).await
    }

    pub async fn apps_usage_operation_cancel(
        &self,
        target_host_id: &str,
        id: &TaskId,
        principal_token: &str,
    ) -> Result<AppsUsageOperation, AppsUsageWriteError> {
        gateway_app_usage::GatewayAppUsageClient::new(self.transport.clone())
            .apps_usage_operation_cancel(target_host_id, id, principal_token).await
    }

    pub async fn apps_usage_save_settings(
        &self,
        selection: &UsageSnapshotQuery,
        settings: &AppsUsageSettings,
        principal_token: &str,
    ) -> Result<(), AppsUsageWriteError> {
        gateway_app_usage::GatewayAppUsageClient::new(self.transport.clone())
            .apps_usage_save_settings(selection, settings, principal_token).await
    }

    pub async fn apps_usage_script_test(
        &self,
        trial: &AppsUsageScriptTrial,
        principal_token: &str,
    ) -> Result<Vec<UsageScriptPackage>, AppsUsageWriteError> {
        gateway_app_usage::GatewayAppUsageClient::new(self.transport.clone())
            .apps_usage_script_test(trial, principal_token).await
    }

}
