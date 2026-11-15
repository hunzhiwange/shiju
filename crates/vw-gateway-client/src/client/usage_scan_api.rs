//! Compatibility forwards for host usage scans.
use super::GatewayClient;
use gateway_usage_scan::*;

impl GatewayClient {
    pub async fn usage_start_scan(&self, scope: &UsageScanScope) -> Result<UsageJobSnapshot, UsageScanError> {
        gateway_usage_scan::GatewayUsageScanClient::new(self.transport.clone())
            .usage_start_scan(scope).await
    }
    pub async fn usage_last_scan(&self, scope: &UsageScanScope) -> Result<UsageLastScan, UsageScanError> {
        gateway_usage_scan::GatewayUsageScanClient::new(self.transport.clone())
            .usage_last_scan(scope).await
    }
    pub async fn usage_job(&self, scope: &UsageScanScope, id: &str) -> Result<UsageJobSnapshot, UsageScanError> {
        gateway_usage_scan::GatewayUsageScanClient::new(self.transport.clone())
            .usage_job(scope, id).await
    }
    pub async fn usage_cancel_job(&self, scope: &UsageScanScope, id: &str) -> Result<UsageJobSnapshot, UsageScanError> {
        gateway_usage_scan::GatewayUsageScanClient::new(self.transport.clone())
            .usage_cancel_job(scope, id).await
    }
    pub async fn usage_rebuild_codex(&self, scope: &UsageScanScope) -> Result<UsageJobSnapshot, UsageScanError> {
        gateway_usage_scan::GatewayUsageScanClient::new(self.transport.clone())
            .usage_rebuild_codex(scope).await
    }
    pub async fn usage_rollup(&self, scope: &UsageScanScope) -> Result<UsageJobSnapshot, UsageScanError> {
        gateway_usage_scan::GatewayUsageScanClient::new(self.transport.clone())
            .usage_rollup(scope).await
    }
    pub async fn usage_backfill(&self, scope: &UsageScanScope) -> Result<UsageJobSnapshot, UsageScanError> {
        gateway_usage_scan::GatewayUsageScanClient::new(self.transport.clone())
            .usage_backfill(scope).await
    }
    pub async fn usage_source_settings(&self, scope: &UsageScanScope) -> Result<Vec<UsageSourceSettingsState>, UsageScanError> {
        gateway_usage_scan::GatewayUsageScanClient::new(self.transport.clone())
            .usage_source_settings(scope).await
    }
    pub async fn usage_save_source_settings(&self, scope: &UsageScanScope, app: &str, expected_revision: u64, settings: &UsageSourceSettings) -> Result<UsageSettingsSaved, UsageScanError> {
        gateway_usage_scan::GatewayUsageScanClient::new(self.transport.clone())
            .usage_save_source_settings(scope, app, expected_revision, settings).await
    }
    pub async fn usage_activity(&self, scope: &UsageScanScope, active: bool, paused: bool) -> Result<(), UsageScanError> {
        gateway_usage_scan::GatewayUsageScanClient::new(self.transport.clone())
            .usage_activity(scope, active, paused).await
    }
    pub async fn stream_usage_revisions(&self, scope: &UsageScanScope, on_event: impl FnMut(Result<UsageRevisionEvent, UsageScanError>) -> bool) -> Result<crate::stream::GatewayStreamAttemptResult, UsageScanError> {
        gateway_usage_scan::GatewayUsageScanClient::new(self.transport.clone())
            .stream_usage_revisions(scope, on_event).await
    }
}
