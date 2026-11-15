//! 模型元数据加载统一委托给 `provider-model-cache`，确保 agent 与桌面端使用同一份模型目录。

use std::collections::HashMap;

pub use provider_model_cache::{
    Model, ModelCost, ModelCostOver200k, ModelInterleaved, ModelLimit, ModelModalities,
    ModelProviderInfo, Provider,
};

pub async fn get() -> HashMap<String, Provider> {
    provider_model_cache::get().await
}

pub async fn refresh() {
    provider_model_cache::refresh().await;
}

pub fn init() {
    provider_model_cache::init();
}

#[cfg(test)]
#[path = "models_tests.rs"]
mod models_tests;
