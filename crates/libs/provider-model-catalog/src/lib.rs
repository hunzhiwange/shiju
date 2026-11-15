//! 模型基线解析、内置目录覆盖与精确模型元数据。

use serde_json::Value;
use std::collections::HashMap;
use tracing::debug;

pub use vw_shared::provider::models::{
    Model, ModelCost, ModelCostOver200k, ModelInterleaved, ModelLimit, ModelModalities,
    ModelProviderInfo, Provider,
};

pub use vw_shared::provider::reasoning::ReasoningOption;

/// 打包内置的模型元数据，作为本地兜底数据源。
const BUNDLED_MODELS_TEXT: &str = include_str!("../assets/model/api.json");
/// 仓库自有的识矩 provider 与离线模型目录。
const BUNDLED_SHIJU_MODELS_TEXT: &str = include_str!("../assets/model/shiju.json");

/// Exact IDs only: prefer Shiju's curated entry, then upstream catalogs before relays.
/// Do not infer capabilities from a family name or strip provider prefixes.
pub fn shiju_model_metadata<'a>(
    catalog: &'a HashMap<String, Provider>,
    model_id: &str,
) -> Option<&'a Model> {
    [
        "shiju",
        "alibaba-cn",
        "alibaba-token-plan-cn",
        "moonshotai-cn",
        "deepseek",
        "zhipuai",
        "alibaba",
        "alibaba-token-plan",
        "moonshotai",
        "hyper",
    ]
    .into_iter()
    .find_map(|provider| catalog.get(provider)?.models.get(model_id))
}

/// 尝试兼容多种 JSON 结构并解析为 provider 映射。
///
/// 当前支持的输入形状包括：
/// - 直接的 `HashMap<String, Provider>`
/// - 带 `providers` 字段的对象
/// - 带 `data` 字段的对象
/// - `Vec<Provider>` 数组
pub fn parse_models_text(label: &str, text: &str) -> Option<HashMap<String, Provider>> {
    let value = match serde_json::from_str::<Value>(text) {
        Ok(v) => v,
        Err(_) => {
            debug!("{label}_parse_invalid");
            return None;
        }
    };

    match &value {
        Value::Object(obj) => {
            debug!(kind = "object", keys_count = obj.len(), "{label}_parse_shape");

            if let Some(providers) = obj.get("providers")
                && let Ok(json) =
                    serde_json::from_value::<HashMap<String, Provider>>(providers.clone())
            {
                debug!(providers = json.len(), "{label}_parsed_wrapped");
                return Some(json);
            }

            if let Some(data) = obj.get("data")
                && let Ok(json) = serde_json::from_value::<HashMap<String, Provider>>(data.clone())
            {
                debug!(providers = json.len(), "{label}_parsed_data");
                return Some(json);
            }

            match serde_json::from_value::<HashMap<String, Provider>>(value.clone()) {
                Ok(json) => {
                    debug!(providers = json.len(), "{label}_parsed");
                    return Some(json);
                }
                Err(_) => debug!("{label}_parse_failed"),
            }
        }
        Value::Array(arr) => {
            debug!(kind = "array", len = arr.len(), "{label}_parse_shape");

            if let Ok(list) = serde_json::from_value::<Vec<Provider>>(value) {
                let map = list
                    .into_iter()
                    .filter(|p| !p.id.is_empty())
                    .map(|p| (p.id.clone(), p))
                    .collect::<HashMap<_, _>>();
                debug!(providers = map.len(), "{label}_parsed_array");
                return Some(map);
            }
        }
        _ => {
            debug!(kind = "other", "{label}_parse_shape");
        }
    }
    None
}

/// 读取打包内置的模型数据。
pub fn bundled_models(label: &str) -> HashMap<String, Provider> {
    let mut providers = parse_models_text(label, BUNDLED_MODELS_TEXT).unwrap_or_default();
    merge_bundled_shiju(&mut providers, label);
    merge_bundled_protocols(&mut providers);
    providers
}

/// Supplement independently from the Shiju-only overlay. This is also applied to
/// disk snapshots and refreshes; source bytes are never rewritten with local facts.
pub fn merge_bundled_protocols(providers: &mut HashMap<String, Provider>) {
    protocol_catalog::merge(providers);
}

#[path = "protocol_catalog.rs"]
mod protocol_catalog;

pub fn merge_bundled_shiju(providers: &mut HashMap<String, Provider>, label: &str) {
    let overlay_label = format!("{label}_shiju");
    let Some(mut overlay) = parse_models_text(&overlay_label, BUNDLED_SHIJU_MODELS_TEXT) else {
        return;
    };
    let Some(shiju) = overlay.remove("shiju") else {
        return;
    };
    providers.insert("shiju".to_string(), shiju);
}

#[cfg(test)]
#[path = "catalog_tests.rs"]
mod tests;
