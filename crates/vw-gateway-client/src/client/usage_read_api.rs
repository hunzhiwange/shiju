//! Compatibility forwards for usage ledger queries.
use super::GatewayClient;
use gateway_usage_read::*;
use vw_api_usage::*;
use std::collections::BTreeMap;

impl GatewayClient {
    pub async fn usage_logs(&self, scope: &UsageReadScope, query: &UsageQuery) -> Result<UsageLogResult, UsageReadError> {
        gateway_usage_read::GatewayUsageReadClient::new(self.transport.clone())
            .usage_logs(scope, query).await
    }
    pub async fn usage_sources(&self, scope: &UsageReadScope, query: &UsageQuery) -> Result<Vec<UsageSourceDistribution>, UsageReadError> {
        gateway_usage_read::GatewayUsageReadClient::new(self.transport.clone())
            .usage_sources(scope, query).await
    }
    pub async fn usage_summary(&self, scope: &UsageReadScope, query: &UsageQuery) -> Result<UsageSummary, UsageReadError> {
        gateway_usage_read::GatewayUsageReadClient::new(self.transport.clone())
            .usage_summary(scope, query).await
    }
    pub async fn usage_summary_apps(&self, scope: &UsageReadScope, query: &UsageQuery) -> Result<BTreeMap<String, UsageSummary>, UsageReadError> {
        gateway_usage_read::GatewayUsageReadClient::new(self.transport.clone())
            .usage_summary_apps(scope, query).await
    }
    pub async fn usage_summary_sessions(&self, scope: &UsageReadScope, query: &UsageQuery) -> Result<Vec<UsageSessionSummary>, UsageReadError> {
        gateway_usage_read::GatewayUsageReadClient::new(self.transport.clone())
            .usage_summary_sessions(scope, query).await
    }
    pub async fn usage_first_date(&self, scope: &UsageReadScope, query: &UsageQuery) -> Result<Option<UsageUnixSeconds>, UsageReadError> {
        gateway_usage_read::GatewayUsageReadClient::new(self.transport.clone())
            .usage_first_date(scope, query).await
    }
    pub async fn usage_provider_stats(&self, scope: &UsageReadScope, query: &UsageQuery) -> Result<Vec<ProviderStats>, UsageReadError> {
        gateway_usage_read::GatewayUsageReadClient::new(self.transport.clone())
            .usage_provider_stats(scope, query).await
    }
    pub async fn usage_model_stats(&self, scope: &UsageReadScope, query: &UsageQuery) -> Result<Vec<ModelStats>, UsageReadError> {
        gateway_usage_read::GatewayUsageReadClient::new(self.transport.clone())
            .usage_model_stats(scope, query).await
    }
    pub async fn usage_heatmap(&self, scope: &UsageReadScope, query: &UsageQuery) -> Result<UsageSeries, UsageReadError> {
        gateway_usage_read::GatewayUsageReadClient::new(self.transport.clone())
            .usage_heatmap(scope, query).await
    }
    pub async fn usage_log_detail(&self, scope: &UsageReadScope, query: &UsageQuery, id: &str) -> Result<UsageLogDetail, UsageReadError> {
        gateway_usage_read::GatewayUsageReadClient::new(self.transport.clone())
            .usage_log_detail(scope, query, id).await
    }
    pub async fn usage_trends(&self, scope: &UsageReadScope, query: &UsageQuery, interval: Option<UsageSeriesInterval>) -> Result<UsageSeries, UsageReadError> {
        gateway_usage_read::GatewayUsageReadClient::new(self.transport.clone())
            .usage_trends(scope, query, interval).await
    }
}
