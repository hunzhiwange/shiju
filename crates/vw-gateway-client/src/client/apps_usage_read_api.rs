//! Read usage resources from the authorized execution host.

use super::GatewayClient;

use vw_api_apps_usage::apps_usage::{UsageSnapshotQuery, UsageSnapshotResponse};

pub use gateway_app_usage::{AppsUsageReadError, AppsUsageTemplate, AppsUsageSettings, AppsUsageDisplay, AppsUsageCredentialRefs, AppsUsageSigningCredentialRefs};

impl GatewayClient {
    pub async fn apps_usage_snapshot(
        &self,
        selection: &UsageSnapshotQuery,
        principal_token: &str,
    ) -> Result<UsageSnapshotResponse, AppsUsageReadError> {
        gateway_app_usage::GatewayAppUsageClient::new(self.transport.clone())
            .apps_usage_snapshot(selection, principal_token).await
    }

    pub async fn usage_limits(
        &self,
        selection: &UsageSnapshotQuery,
        principal_token: &str,
    ) -> Result<UsageSnapshotResponse, AppsUsageReadError> {
        gateway_app_usage::GatewayAppUsageClient::new(self.transport.clone())
            .usage_limits(selection, principal_token).await
    }

    pub async fn apps_usage_templates(
        &self,
        selection: &UsageSnapshotQuery,
        principal_token: &str,
    ) -> Result<Vec<AppsUsageTemplate>, AppsUsageReadError> {
        gateway_app_usage::GatewayAppUsageClient::new(self.transport.clone())
            .apps_usage_templates(selection, principal_token).await
    }

    pub async fn apps_usage_settings(
        &self,
        selection: &UsageSnapshotQuery,
        principal_token: &str,
    ) -> Result<Option<AppsUsageSettings>, AppsUsageReadError> {
        gateway_app_usage::GatewayAppUsageClient::new(self.transport.clone())
            .apps_usage_settings(selection, principal_token).await
    }

}
