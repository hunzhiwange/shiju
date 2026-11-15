//! 模型元数据加载统一委托给 `vw-provider-resolver`，确保 agent 与桌面端使用同一份模型目录。

use std::collections::HashMap;

pub use vw_provider_resolver::models::{
    Model, ModelCost, ModelCostOver200k, ModelInterleaved, ModelLimit, ModelModalities,
    ModelProviderInfo, Provider,
};

pub async fn get() -> HashMap<String, Provider> {
    vw_provider_resolver::models::get().await
}

pub async fn refresh() {
    vw_provider_resolver::models::refresh().await;
}

pub fn init() {
    vw_provider_resolver::models::init();
}

#[cfg(test)]
#[path = "models_tests.rs"]
mod models_tests;
