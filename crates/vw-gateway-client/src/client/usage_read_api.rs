//! Compatibility forwards for usage ledger queries.
use super::GatewayClient;
use gateway_usage_read::*;
use vw_api_usage::*;
use std::collections::BTreeMap;

impl GatewayClient {
    pub async fn usage_logs(&self, time_zone: &str, query: &UsageQuery) -> Result<UsageLogResult, UsageReadError> {
        gateway_usage_read::GatewayUsageReadClient::new(self.transport.clone())
            .usage_logs(time_zone, query).await
    }
    pub async fn usage_sources(&self, time_zone: &str, query: &UsageQuery) -> Result<Vec<UsageSourceDistribution>, UsageReadError> {
        gateway_usage_read::GatewayUsageReadClient::new(self.transport.clone())
            .usage_sources(time_zone, query).await
    }
    pub async fn usage_summary(&self, time_zone: &str, query: &UsageQuery) -> Result<UsageSummary, UsageReadError> {
        gateway_usage_read::GatewayUsageReadClient::new(self.transport.clone())
            .usage_summary(time_zone, query).await
    }
    pub async fn usage_summary_apps(&self, time_zone: &str, query: &UsageQuery) -> Result<BTreeMap<String, UsageSummary>, UsageReadError> {
        gateway_usage_read::GatewayUsageReadClient::new(self.transport.clone())
            .usage_summary_apps(time_zone, query).await
    }
    pub async fn usage_summary_sessions(&self, time_zone: &str, query: &UsageQuery) -> Result<Vec<UsageSessionSummary>, UsageReadError> {
        gateway_usage_read::GatewayUsageReadClient::new(self.transport.clone())
            .usage_summary_sessions(time_zone, query).await
    }
    pub async fn usage_first_date(&self, time_zone: &str, query: &UsageQuery) -> Result<Option<UsageUnixSeconds>, UsageReadError> {
        gateway_usage_read::GatewayUsageReadClient::new(self.transport.clone())
            .usage_first_date(time_zone, query).await
    }
    pub async fn usage_provider_stats(&self, time_zone: &str, query: &UsageQuery) -> Result<Vec<ProviderStats>, UsageReadError> {
        gateway_usage_read::GatewayUsageReadClient::new(self.transport.clone())
            .usage_provider_stats(time_zone, query).await
    }
    pub async fn usage_model_stats(&self, time_zone: &str, query: &UsageQuery) -> Result<Vec<ModelStats>, UsageReadError> {
        gateway_usage_read::GatewayUsageReadClient::new(self.transport.clone())
            .usage_model_stats(time_zone, query).await
    }
    pub async fn usage_heatmap(&self, time_zone: &str, query: &UsageQuery) -> Result<UsageSeries, UsageReadError> {
        gateway_usage_read::GatewayUsageReadClient::new(self.transport.clone())
            .usage_heatmap(time_zone, query).await
    }
    pub async fn usage_log_detail(&self, time_zone: &str, query: &UsageQuery, id: &str) -> Result<UsageLogDetail, UsageReadError> {
        gateway_usage_read::GatewayUsageReadClient::new(self.transport.clone())
            .usage_log_detail(time_zone, query, id).await
    }
    pub async fn usage_trends(&self, time_zone: &str, query: &UsageQuery, interval: Option<UsageSeriesInterval>) -> Result<UsageSeries, UsageReadError> {
        gateway_usage_read::GatewayUsageReadClient::new(self.transport.clone())
            .usage_trends(time_zone, query, interval).await
    }
}
