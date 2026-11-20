//! Compatibility forwards for usage prices and display preferences.
use super::GatewayClient;
use gateway_usage_price::*;

impl GatewayClient {
    pub async fn usage_prices(&self) -> Result<Vec<StoredModelPricing>, UsagePriceError> {
        gateway_usage_price::GatewayUsagePriceClient::new(self.transport.clone())
            .usage_prices().await
    }
    pub async fn usage_create_price(&self, price: &ModelPricingInfo) -> Result<StoredModelPricing, UsagePriceError> {
        gateway_usage_price::GatewayUsagePriceClient::new(self.transport.clone())
            .usage_create_price(price).await
    }
    pub async fn usage_update_price(&self, price: &ModelPricingInfo) -> Result<StoredModelPricing, UsagePriceError> {
        gateway_usage_price::GatewayUsagePriceClient::new(self.transport.clone())
            .usage_update_price(price).await
    }
    pub async fn usage_delete_price(&self, model_id: &str) -> Result<UsagePriceDeleted, UsagePriceError> {
        gateway_usage_price::GatewayUsagePriceClient::new(self.transport.clone())
            .usage_delete_price(model_id).await
    }
    pub async fn usage_import_prices(&self, source: &str, prices: &[ModelPricingInfo]) -> Result<UsagePriceImportResult, UsagePriceError> {
        gateway_usage_price::GatewayUsagePriceClient::new(self.transport.clone())
            .usage_import_prices(source, prices).await
    }
    pub async fn usage_models_dev_candidates(&self) -> Result<Vec<ModelsDevEntry>, UsagePriceError> {
        gateway_usage_price::GatewayUsagePriceClient::new(self.transport.clone())
            .usage_models_dev_candidates().await
    }
    pub async fn usage_models_dev_config(&self) -> Result<ModelsDevSyncState, UsagePriceError> {
        gateway_usage_price::GatewayUsagePriceClient::new(self.transport.clone())
            .usage_models_dev_config().await
    }
    pub async fn usage_save_models_dev_config(&self, config: &ModelsDevSyncConfig) -> Result<ModelsDevSyncState, UsagePriceError> {
        gateway_usage_price::GatewayUsagePriceClient::new(self.transport.clone())
            .usage_save_models_dev_config(config).await
    }
    pub async fn usage_sync_models_dev(&self) -> Result<UsagePriceSyncAccepted, UsagePriceError> {
        gateway_usage_price::GatewayUsagePriceClient::new(self.transport.clone())
            .usage_sync_models_dev().await
    }
    pub async fn usage_auto_sync_models_dev(&self) -> Result<UsagePriceSyncAccepted, UsagePriceError> {
        gateway_usage_price::GatewayUsagePriceClient::new(self.transport.clone())
            .usage_auto_sync_models_dev().await
    }
    pub async fn usage_rates(&self) -> Result<UsageRatesState, UsagePriceError> {
        gateway_usage_price::GatewayUsagePriceClient::new(self.transport.clone())
            .usage_rates().await
    }
    pub async fn usage_refresh_rates(&self) -> Result<UsageRatesState, UsagePriceError> {
        gateway_usage_price::GatewayUsagePriceClient::new(self.transport.clone())
            .usage_refresh_rates().await
    }
    pub async fn usage_display(&self) -> Result<UsageRatesState, UsagePriceError> {
        gateway_usage_price::GatewayUsagePriceClient::new(self.transport.clone())
            .usage_display().await
    }
    pub async fn usage_save_display(&self, patch: &UsageDisplayPatch) -> Result<ExchangeSettings, UsagePriceError> {
        gateway_usage_price::GatewayUsagePriceClient::new(self.transport.clone())
            .usage_save_display(patch).await
    }
}
