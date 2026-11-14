//! Named SDK operations connected to the gateway's host-owned price service.
use super::{GatewayClient, UsageScanScope, usage_price_types::*};
use serde::de::DeserializeOwned;
use serde_json::{Value, json};

#[derive(Debug, Clone, PartialEq)]
pub enum UsagePriceError {
    Unauthenticated,
    InvalidInput(String),
    Transport(String),
    Http { status: u16, body: Value },
    Decode(String),
}
impl std::fmt::Display for UsagePriceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unauthenticated => f.write_str("usage pricing requires gateway credentials"),
            Self::InvalidInput(s) | Self::Transport(s) | Self::Decode(s) => f.write_str(s),
            Self::Http { status, .. } => write!(f, "usage pricing HTTP {status}"),
        }
    }
}
impl std::error::Error for UsagePriceError {}

impl GatewayClient {
    pub async fn usage_prices(&self, scope: &UsageScanScope) -> Result<Vec<StoredModelPricing>, UsagePriceError> {
        self.usage_price_request(reqwest::Method::GET, &["prices"], scope, None).await
    }
    pub async fn usage_create_price(&self, scope: &UsageScanScope, price: &ModelPricingInfo) -> Result<StoredModelPricing, UsagePriceError> {
        self.usage_price_request(reqwest::Method::POST, &["prices"], scope, Some(json!({"price":price}))).await
    }
    pub async fn usage_update_price(&self, scope: &UsageScanScope, price: &ModelPricingInfo) -> Result<StoredModelPricing, UsagePriceError> {
        validate_id(price.model_id.as_ref())?;
        self.usage_price_request(reqwest::Method::PUT, &["prices", price.model_id.as_ref()], scope, Some(json!({"price":price}))).await
    }
    pub async fn usage_delete_price(&self, scope: &UsageScanScope, model_id: &str) -> Result<UsagePriceDeleted, UsagePriceError> {
        validate_id(model_id)?;
        self.usage_price_request(reqwest::Method::DELETE, &["prices", model_id], scope, None).await
    }
    pub async fn usage_import_prices(&self, scope: &UsageScanScope, source: &str, prices: &[ModelPricingInfo]) -> Result<UsagePriceImportResult, UsagePriceError> {
        if source.trim().is_empty() || source.len() > 256 || prices.is_empty() || prices.len() > 256 {
            return Err(UsagePriceError::InvalidInput("explicit source and 1–256 prices required".into()));
        }
        self.usage_price_request(reqwest::Method::POST, &["prices", "import"], scope, Some(json!({"source":source,"prices":prices}))).await
    }
    pub async fn usage_models_dev_candidates(&self, scope: &UsageScanScope) -> Result<Vec<ModelsDevEntry>, UsagePriceError> {
        self.usage_price_request(reqwest::Method::GET, &["models-dev", "candidates"], scope, None).await
    }
    pub async fn usage_models_dev_config(&self, scope: &UsageScanScope) -> Result<ModelsDevSyncState, UsagePriceError> {
        self.usage_price_request(reqwest::Method::GET, &["models-dev", "config"], scope, None).await
    }
    pub async fn usage_save_models_dev_config(&self, scope: &UsageScanScope, config: &ModelsDevSyncConfig) -> Result<ModelsDevSyncState, UsagePriceError> {
        let body = serde_json::to_value(config).map_err(|e| UsagePriceError::InvalidInput(e.to_string()))?;
        self.usage_price_request(reqwest::Method::PUT, &["models-dev", "config"], scope, Some(body)).await
    }
    pub async fn usage_sync_models_dev(&self, scope: &UsageScanScope) -> Result<UsagePriceSyncAccepted, UsagePriceError> {
        self.usage_price_request(reqwest::Method::POST, &["models-dev", "sync"], scope, None).await
    }
    pub async fn usage_rates(&self, scope: &UsageScanScope) -> Result<UsageRatesState, UsagePriceError> {
        self.usage_price_request(reqwest::Method::GET, &["rates"], scope, None).await
    }
    /// A successful HTTP response may contain a retained stale cache and refresh_error.
    pub async fn usage_refresh_rates(&self, scope: &UsageScanScope) -> Result<UsageRatesState, UsagePriceError> {
        self.usage_price_request(reqwest::Method::POST, &["rates", "refresh"], scope, None).await
    }
    pub async fn usage_display(&self, scope: &UsageScanScope) -> Result<UsageRatesState, UsagePriceError> {
        self.usage_price_request(reqwest::Method::GET, &["display"], scope, None).await
    }
    /// The write endpoint returns settings; read usage_display for resolved conversion.
    pub async fn usage_save_display(&self, scope: &UsageScanScope, patch: &UsageDisplayPatch) -> Result<ExchangeSettings, UsagePriceError> {
        if patch.manual_rates.values().any(|r| !r.is_finite() || *r <= 0.0) {
            return Err(UsagePriceError::InvalidInput("manual rates must be finite and positive".into()));
        }
        let body = serde_json::to_value(patch).map_err(|e| UsagePriceError::InvalidInput(e.to_string()))?;
        self.usage_price_request(reqwest::Method::PUT, &["display"], scope, Some(body)).await
    }
    async fn usage_price_request<T: DeserializeOwned>(&self, method: reqwest::Method, path: &[&str], scope: &UsageScanScope, body: Option<Value>) -> Result<T, UsagePriceError> {
        if !self.endpoint.auth.as_ref().and_then(|a| a.skey.as_deref()).is_some_and(|s| !s.trim().is_empty()) {
            return Err(UsagePriceError::Unauthenticated);
        }
        if scope.project_id.as_ref().trim().is_empty() || scope.target_host.trim().is_empty() {
            return Err(UsagePriceError::InvalidInput("usage project and host required".into()));
        }
        let mut url = reqwest::Url::parse(&self.endpoint.base_url()).map_err(|e| UsagePriceError::InvalidInput(e.to_string()))?;
        url.path_segments_mut().map_err(|_| UsagePriceError::InvalidInput("invalid gateway URL".into()))?
            .pop_if_empty().extend(["v1", "usage"]).extend(path.iter().copied());
        // Price handlers authorize scope from query parameters for every HTTP method.
        let mut request = self.native_transport().map_err(UsagePriceError::Transport)?.request(method, url).query(scope);
        if let Some(body) = body { request = request.json(&body); }
        let response = crate::http::apply_request_context(request, &self.endpoint).send().await.map_err(|e| UsagePriceError::Transport(e.to_string()))?;
        let status = response.status();
        let bytes = response.bytes().await.map_err(|e| UsagePriceError::Transport(e.to_string()))?;
        if !status.is_success() {
            let body = serde_json::from_slice(&bytes).unwrap_or_else(|_| Value::String(String::from_utf8_lossy(&bytes).into_owned()));
            return Err(UsagePriceError::Http { status: status.as_u16(), body });
        }
        serde_json::from_slice(&bytes).map_err(|e| UsagePriceError::Decode(e.to_string()))
    }
}
fn validate_id(id: &str) -> Result<(), UsagePriceError> {
    if id.trim().is_empty() || matches!(id, "." | "..") {
        return Err(UsagePriceError::InvalidInput("invalid model id".into()));
    }
    Ok(())
}
