//! Compatibility forwards for host usage scans.
use super::GatewayClient;
use gateway_usage_scan::*;

impl GatewayClient {
    pub async fn usage_start_scan(&self) -> Result<UsageJobSnapshot, UsageScanError> {
        gateway_usage_scan::GatewayUsageScanClient::new(self.transport.clone())
            .usage_start_scan().await
    }
    pub async fn usage_last_scan(&self) -> Result<UsageLastScan, UsageScanError> {
        gateway_usage_scan::GatewayUsageScanClient::new(self.transport.clone())
            .usage_last_scan().await
    }
    pub async fn usage_job(&self, id: &str) -> Result<UsageJobSnapshot, UsageScanError> {
        gateway_usage_scan::GatewayUsageScanClient::new(self.transport.clone())
            .usage_job(id).await
    }
    pub async fn usage_cancel_job(&self, id: &str) -> Result<UsageJobSnapshot, UsageScanError> {
        gateway_usage_scan::GatewayUsageScanClient::new(self.transport.clone())
            .usage_cancel_job(id).await
    }
    pub async fn usage_rebuild_codex(&self) -> Result<UsageJobSnapshot, UsageScanError> {
        gateway_usage_scan::GatewayUsageScanClient::new(self.transport.clone())
            .usage_rebuild_codex().await
    }
    pub async fn usage_rollup(&self) -> Result<UsageJobSnapshot, UsageScanError> {
        gateway_usage_scan::GatewayUsageScanClient::new(self.transport.clone())
            .usage_rollup().await
    }
    pub async fn usage_backfill(&self) -> Result<UsageJobSnapshot, UsageScanError> {
        gateway_usage_scan::GatewayUsageScanClient::new(self.transport.clone())
            .usage_backfill().await
    }
    pub async fn usage_source_settings(&self) -> Result<Vec<UsageSourceSettingsState>, UsageScanError> {
        gateway_usage_scan::GatewayUsageScanClient::new(self.transport.clone())
            .usage_source_settings().await
    }
    pub async fn usage_save_source_settings(&self, app: &str, expected_revision: u64, settings: &UsageSourceSettings) -> Result<UsageSettingsSaved, UsageScanError> {
        gateway_usage_scan::GatewayUsageScanClient::new(self.transport.clone())
            .usage_save_source_settings(app, expected_revision, settings).await
    }
    pub async fn usage_activity(&self, active: bool, paused: bool) -> Result<(), UsageScanError> {
        gateway_usage_scan::GatewayUsageScanClient::new(self.transport.clone())
            .usage_activity(active, paused).await
    }
    pub async fn stream_usage_revisions(&self, on_event: impl FnMut(Result<UsageRevisionEvent, UsageScanError>) -> bool) -> Result<crate::stream::GatewayStreamAttemptResult, UsageScanError> {
        gateway_usage_scan::GatewayUsageScanClient::new(self.transport.clone())
            .stream_usage_revisions(on_event).await
    }
}
