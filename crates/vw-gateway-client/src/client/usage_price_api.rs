//! Compatibility forwards for usage prices and display preferences.
use super::{GatewayClient, UsageScanScope};
use gateway_usage_price::*;

impl GatewayClient {
    pub async fn usage_prices(&self, scope: &UsageScanScope) -> Result<Vec<StoredModelPricing>, UsagePriceError> {
        gateway_usage_price::GatewayUsagePriceClient::new(self.transport.clone())
            .usage_prices(scope).await
    }
    pub async fn usage_create_price(&self, scope: &UsageScanScope, price: &ModelPricingInfo) -> Result<StoredModelPricing, UsagePriceError> {
        gateway_usage_price::GatewayUsagePriceClient::new(self.transport.clone())
            .usage_create_price(scope, price).await
    }
    pub async fn usage_update_price(&self, scope: &UsageScanScope, price: &ModelPricingInfo) -> Result<StoredModelPricing, UsagePriceError> {
        gateway_usage_price::GatewayUsagePriceClient::new(self.transport.clone())
            .usage_update_price(scope, price).await
    }
    pub async fn usage_delete_price(&self, scope: &UsageScanScope, model_id: &str) -> Result<UsagePriceDeleted, UsagePriceError> {
        gateway_usage_price::GatewayUsagePriceClient::new(self.transport.clone())
            .usage_delete_price(scope, model_id).await
    }
    pub async fn usage_import_prices(&self, scope: &UsageScanScope, source: &str, prices: &[ModelPricingInfo]) -> Result<UsagePriceImportResult, UsagePriceError> {
        gateway_usage_price::GatewayUsagePriceClient::new(self.transport.clone())
            .usage_import_prices(scope, source, prices).await
    }
    pub async fn usage_models_dev_candidates(&self, scope: &UsageScanScope) -> Result<Vec<ModelsDevEntry>, UsagePriceError> {
        gateway_usage_price::GatewayUsagePriceClient::new(self.transport.clone())
            .usage_models_dev_candidates(scope).await
    }
    pub async fn usage_models_dev_config(&self, scope: &UsageScanScope) -> Result<ModelsDevSyncState, UsagePriceError> {
        gateway_usage_price::GatewayUsagePriceClient::new(self.transport.clone())
            .usage_models_dev_config(scope).await
    }
    pub async fn usage_save_models_dev_config(&self, scope: &UsageScanScope, config: &ModelsDevSyncConfig) -> Result<ModelsDevSyncState, UsagePriceError> {
        gateway_usage_price::GatewayUsagePriceClient::new(self.transport.clone())
            .usage_save_models_dev_config(scope, config).await
    }
    pub async fn usage_sync_models_dev(&self, scope: &UsageScanScope) -> Result<UsagePriceSyncAccepted, UsagePriceError> {
        gateway_usage_price::GatewayUsagePriceClient::new(self.transport.clone())
            .usage_sync_models_dev(scope).await
    }
    pub async fn usage_auto_sync_models_dev(&self, scope: &UsageScanScope) -> Result<UsagePriceSyncAccepted, UsagePriceError> {
        gateway_usage_price::GatewayUsagePriceClient::new(self.transport.clone())
            .usage_auto_sync_models_dev(scope).await
    }
    pub async fn usage_rates(&self, scope: &UsageScanScope) -> Result<UsageRatesState, UsagePriceError> {
        gateway_usage_price::GatewayUsagePriceClient::new(self.transport.clone())
            .usage_rates(scope).await
    }
    pub async fn usage_refresh_rates(&self, scope: &UsageScanScope) -> Result<UsageRatesState, UsagePriceError> {
        gateway_usage_price::GatewayUsagePriceClient::new(self.transport.clone())
            .usage_refresh_rates(scope).await
    }
    pub async fn usage_display(&self, scope: &UsageScanScope) -> Result<UsageRatesState, UsagePriceError> {
        gateway_usage_price::GatewayUsagePriceClient::new(self.transport.clone())
            .usage_display(scope).await
    }
    pub async fn usage_save_display(&self, scope: &UsageScanScope, patch: &UsageDisplayPatch) -> Result<ExchangeSettings, UsagePriceError> {
        gateway_usage_price::GatewayUsagePriceClient::new(self.transport.clone())
            .usage_save_display(scope, patch).await
    }
}
