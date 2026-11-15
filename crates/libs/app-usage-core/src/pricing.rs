//! USD price values and normalization.
use super::cost::ModelPricing;
use app_models::edits::validate_model_id;
use serde::{Deserialize, Serialize};
use vw_api_transport::id::ModelId;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelPricingInfo {
    pub model_id: ModelId,
    pub display_name: String,
    pub input_cost_per_million: String,
    pub output_cost_per_million: String,
    pub cache_read_cost_per_million: String,
    pub cache_creation_cost_per_million: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StoredModelPricing {
    pub price: ModelPricingInfo,
    pub source: String,
    pub is_custom: bool,
}

impl ModelPricingInfo {
    pub fn rates(&self) -> Result<ModelPricing, String> {
        ModelPricing::from_strings(
            self.input_cost_per_million.trim(),
            self.output_cost_per_million.trim(),
            self.cache_read_cost_per_million.trim(),
            self.cache_creation_cost_per_million.trim(),
        )
    }

    pub fn normalize(mut self) -> Result<Self, String> {
        self.model_id = ModelId::from(self.model_id.as_ref().trim().to_owned());
        validate_model_id(&self.model_id)?;
        self.display_name = self.display_name.trim().to_owned();
        if self.display_name.is_empty() {
            return Err("Model display name is required".into());
        }
        let rates = self.rates()?;
        self.input_cost_per_million = rates.input_cost_per_million.normalize().to_string();
        self.output_cost_per_million = rates.output_cost_per_million.normalize().to_string();
        self.cache_read_cost_per_million =
            rates.cache_read_cost_per_million.normalize().to_string();
        self.cache_creation_cost_per_million =
            rates.cache_creation_cost_per_million.normalize().to_string();
        Ok(self)
    }
}
